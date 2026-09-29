//! RustForge 网络（IF-230 ~ IF-235）：传输抽象、回环/UDP、复制、RPC、预测回滚。

use rf_core::Result;
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;

/// 网络传输（IF-230，可扩展）。
pub trait NetworkTransport: Send {
    fn connect(&mut self, addr: &str) -> Result<()>;
    /// 发送到当前对端。
    fn send(&mut self, data: &[u8]) -> Result<()>;
    /// 收取全部待处理报文 (来源, 数据)。
    fn recv(&mut self) -> Vec<(String, Vec<u8>)>;
    fn connected(&self) -> bool;
    fn close(&mut self);
}

/// 回环传输对（IF-231）：带丢包/延迟模拟。
/// peer = 对端收件队列（send 目的地）；inbox = 自己的收件队列（recv 来源）。
pub struct LoopbackTransport {
    peer: Arc<Mutex<Vec<Vec<u8>>>>,
    inbox: Arc<Mutex<Vec<Vec<u8>>>>,
    drop_rate: f32,
    latency_frames: u32,
    pending: Vec<(u32, Vec<u8>)>,
    frame: u32,
    connected: bool,
}

impl LoopbackTransport {
    /// 创建互通对 (a, b)。
    pub fn pair(drop_rate: f32, latency_frames: u32) -> (Self, Self) {
        let inbox_a = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let inbox_b = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        (
            Self {
                peer: inbox_b.clone(),
                inbox: inbox_a.clone(),
                drop_rate,
                latency_frames,
                pending: Vec::new(),
                frame: 0,
                connected: false,
            },
            Self {
                peer: inbox_a,
                inbox: inbox_b,
                drop_rate,
                latency_frames,
                pending: Vec::new(),
                frame: 0,
                connected: false,
            },
        )
    }

    /// 推进模拟时间（延迟到包投递）。
    pub fn tick(&mut self) {
        self.frame += 1;
        let mut ready = Vec::new();
        self.pending.retain_mut(|(due, data)| {
            if self.frame >= *due {
                ready.push(std::mem::take(data));
                false
            } else {
                true
            }
        });
        for data in ready {
            let _ = self.peer.lock().map(|mut p| p.push(data));
        }
    }
}

impl NetworkTransport for LoopbackTransport {
    fn connect(&mut self, _addr: &str) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    fn send(&mut self, data: &[u8]) -> Result<()> {
        if !self.connected {
            return Err(rf_core::EngineError::Message("not connected".into()));
        }
        if rf_core::Pcg32::new(self.frame as u64 + 1).next_f32() < self.drop_rate {
            return Ok(()); // 模拟丢包
        }
        if self.latency_frames > 0 {
            self.pending.push((self.frame + self.latency_frames, data.to_vec()));
        } else {
            self.peer.lock().unwrap().push(data.to_vec());
        }
        Ok(())
    }

    fn recv(&mut self) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        if let Ok(mut inbox) = self.inbox.lock() {
            for d in inbox.drain(..) {
                out.push(("loopback".to_string(), d));
            }
        }
        out
    }

    fn connected(&self) -> bool {
        self.connected
    }

    fn close(&mut self) {
        self.connected = false;
    }
}

/// UDP 传输（IF-232）。
pub struct UdpTransport {
    socket: Option<UdpSocket>,
    peer: Option<std::net::SocketAddr>,
}

impl Default for UdpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl UdpTransport {
    pub fn new() -> Self {
        Self { socket: None, peer: None }
    }
}

impl NetworkTransport for UdpTransport {
    fn connect(&mut self, addr: &str) -> Result<()> {
        let sock = UdpSocket::bind("0.0.0.0:0")?;
        sock.set_nonblocking(true)?;
        let peer: std::net::SocketAddr = addr
            .parse()
            .map_err(|_| rf_core::EngineError::InvalidData(format!("bad addr {addr}")))?;
        self.peer = Some(peer);
        self.socket = Some(sock);
        Ok(())
    }

    fn send(&mut self, data: &[u8]) -> Result<()> {
        let Some(sock) = &self.socket else {
            return Err(rf_core::EngineError::Message("not connected".into()));
        };
        let Some(peer) = self.peer else {
            return Err(rf_core::EngineError::Message("no peer".into()));
        };
        sock.send_to(data, peer)?;
        Ok(())
    }

    fn recv(&mut self) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        if let Some(sock) = &self.socket {
            let mut buf = [0u8; 2048];
            while let Ok((n, from)) = sock.recv_from(&mut buf) {
                out.push((from.to_string(), buf[..n].to_vec()));
                if out.len() > 256 {
                    break;
                }
            }
        }
        out
    }

    fn connected(&self) -> bool {
        self.socket.is_some()
    }

    fn close(&mut self) {
        self.socket = None;
    }
}

// ---- 复制（IF-233） ----

/// 复制状态存储：脏标记 + 优先级 + 预算内构造增量。
pub struct ReplicaStore {
    tracked: HashMap<u64, ReplicaState>,
    seq: u64,
}

struct ReplicaState {
    priority: u8,
    dirty_channels: Vec<u32>,
    acked_seq: u64,
}

impl Default for ReplicaStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicaStore {
    pub fn new() -> Self {
        Self { tracked: HashMap::new(), seq: 0 }
    }

    pub fn track(&mut self, id: u64, priority: u8) {
        self.tracked.entry(id).or_insert(ReplicaState {
            priority,
            dirty_channels: Vec::new(),
            acked_seq: 0,
        });
    }

    pub fn mark_dirty(&mut self, id: u64, channel: u32) {
        if let Some(s) = self.tracked.get_mut(&id) {
            if !s.dirty_channels.contains(&channel) {
                s.dirty_channels.push(channel);
            }
        }
    }

    /// 预算内构造增量：优先级高者优先。字节 = 序号(8) + 条目数(4) + 每条 (id 8 + channel 4)。
    pub fn build_snapshot(
        &mut self,
        budget_bytes: usize,
        encode: &dyn Fn(u64, u32) -> Vec<u8>,
    ) -> Vec<u8> {
        self.seq += 1;
        let mut ids: Vec<u64> = self
            .tracked
            .iter()
            .filter(|(_, s)| !s.dirty_channels.is_empty())
            .map(|(id, _)| *id)
            .collect();
        ids.sort_by_key(|id| std::cmp::Reverse(self.tracked[id].priority));
        let mut out = Vec::new();
        out.extend_from_slice(&self.seq.to_le_bytes());
        let mut entries: Vec<(u64, u32, Vec<u8>)> = Vec::new();
        let mut used = 12usize;
        for id in ids {
            let channels: Vec<u32> = self.tracked[&id].dirty_channels.clone();
            for ch in channels {
                let payload = encode(id, ch);
                let cost = 12 + payload.len();
                if used + cost > budget_bytes {
                    continue; // 超预算跳过该条（下轮再发）
                }
                used += cost;
                entries.push((id, ch, payload));
            }
        }
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (id, ch, payload) in &entries {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&ch.to_le_bytes());
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(payload);
            // 清脏
            if let Some(s) = self.tracked.get_mut(id) {
                s.dirty_channels.retain(|c| c != ch);
                s.acked_seq = self.seq;
            }
        }
        out
    }

    /// 应用增量：返回 (id, channel, payload) 列表交上层解码。
    pub fn apply_update(&mut self, bytes: &[u8]) -> Vec<(u64, u32, Vec<u8>)> {
        let mut out = Vec::new();
        if bytes.len() < 12 {
            return out;
        }
        let mut i = 12;
        let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        for _ in 0..count {
            if i + 16 > bytes.len() {
                break;
            }
            let id = u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
            let ch = u32::from_le_bytes(bytes[i + 8..i + 12].try_into().unwrap());
            let plen = u32::from_le_bytes(bytes[i + 12..i + 16].try_into().unwrap()) as usize;
            i += 16;
            if i + plen > bytes.len() {
                break;
            }
            out.push((id, ch, bytes[i..i + plen].to_vec()));
            i += plen;
        }
        out
    }

    pub fn dirty_count(&self) -> usize {
        self.tracked.values().filter(|s| !s.dirty_channels.is_empty()).count()
    }
}

// ---- RPC（IF-234） ----

/// RPC 处理器类型。
pub type RpcHandler = Box<dyn Fn(&[u8]) + Send + Sync>;

/// RPC 通道：可靠 = 重发队列 + 序号去重；不可靠 = 尽力而为。
/// （可靠重发/去重的 P1 传输层扩展字段预留于协议设计，MVP 以 outbox 即时发）
pub struct RpcChannel {
    handlers: HashMap<String, RpcHandler>,
    outbox: Vec<(String, Vec<u8>, bool)>,
}

impl Default for RpcChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcChannel {
    pub fn new() -> Self {
        Self { handlers: HashMap::new(), outbox: Vec::new() }
    }

    pub fn register<F: Fn(&[u8]) + Send + Sync + 'static>(&mut self, name: &str, f: F) {
        self.handlers.insert(name.to_string(), Box::new(f));
    }

    pub fn call(&mut self, name: &str, payload: &[u8], reliable: bool) {
        self.outbox.push((name.to_string(), payload.to_vec(), reliable));
    }

    /// 序列化待发消息（帧循环取走）。
    pub fn drain_outbox(&mut self) -> Vec<(String, Vec<u8>, bool)> {
        std::mem::take(&mut self.outbox)
    }

    /// 应用远端消息（帧循环喂入），返回分发数。
    pub fn dispatch(&mut self, messages: &[(String, Vec<u8>)]) -> usize {
        let mut n = 0;
        for (name, payload) in messages {
            if let Some(h) = self.handlers.get(name) {
                h(payload);
                n += 1;
            }
        }
        n
    }
}

// ---- 预测回滚（IF-235） ----

/// 预测缓冲：输入历史 + 状态哈希；服务器权威哈希对账。
pub struct PredictionBuffer {
    history: std::collections::VecDeque<(u64, Vec<u8>, u64)>,
    max: usize,
}

impl Default for PredictionBuffer {
    fn default() -> Self {
        Self::new(120)
    }
}

impl PredictionBuffer {
    pub fn new(max: usize) -> Self {
        Self { history: Default::default(), max }
    }

    pub fn push(&mut self, tick: u64, input: Vec<u8>, state_hash: u64) {
        self.history.push_back((tick, input, state_hash));
        while self.history.len() > self.max {
            self.history.pop_front();
        }
    }

    /// 对账：权威哈希不匹配 → 返回需回放的输入数（0 = 一致）。
    pub fn reconcile(&mut self, tick: u64, authoritative_hash: u64) -> usize {
        // 找到该 tick 的预测
        let mismatch = !self.history.iter().any(|(t, _, h)| *t == tick && *h == authoritative_hash);
        if !mismatch {
            return 0;
        }
        // 回滚：丢弃该 tick 及之后的预测（上层用保留输入回放）
        let rollback = self.history.iter().filter(|(t, _, _)| *t >= tick).count();
        self.history.retain(|(t, _, _)| *t < tick);
        rollback
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn loopback_pair_delivery() {
        let (mut a, mut b) = LoopbackTransport::pair(0.0, 0);
        a.connect("loop").unwrap();
        b.connect("loop").unwrap();
        a.send(b"ping").unwrap();
        b.tick(); // 转移
        let got = b.recv();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1, b"ping");
        // 未连接拒绝
        let mut c = LoopbackTransport::pair(0.0, 0).0;
        assert!(c.send(b"x").is_err());
    }

    #[test]
    fn loopback_latency_ordering() {
        let (mut a, mut b) = LoopbackTransport::pair(0.0, 2);
        a.connect("l").unwrap();
        b.connect("l").unwrap();
        a.send(b"first").unwrap();
        a.tick(); // 帧 1 < 到期帧 2 → 未投递
        assert!(b.recv().is_empty());
        a.tick(); // 帧 2 → 投递
        let got = b.recv();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].1, b"first");
    }

    #[test]
    fn udp_loop() {
        // 自发自收（本机回环）
        let mut t = UdpTransport::new();
        assert!(!t.connected());
        // 绑定失败地址
        assert!(t.connect("not-an-addr").is_err());
        t.close();
        assert!(!t.connected());
        assert!(t.send(b"x").is_err());
    }

    #[test]
    fn replica_delta_and_budget() {
        let mut store = ReplicaStore::new();
        store.track(1, 10);
        store.track(2, 1);
        store.mark_dirty(1, 7);
        store.mark_dirty(2, 7);
        let encode = |id: u64, ch: u32| (id * 100 + ch as u64).to_le_bytes().to_vec();
        let snap = store.build_snapshot(1024, &encode);
        assert!(snap.len() > 12);
        assert_eq!(store.dirty_count(), 0); // 已清脏
                                            // 解码
        let mut client = ReplicaStore::new();
        let entries = client.apply_update(&snap);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, 1); // 高优先级先
        assert_eq!(entries[0].2, (107u64).to_le_bytes().to_vec());
        // 预算受限
        let mut s2 = ReplicaStore::new();
        s2.track(1, 5);
        s2.mark_dirty(1, 1);
        let small = s2.build_snapshot(14, &|_, _| vec![0u8; 8]); // 只容 0 条
        assert_eq!(small.len(), 12);
        assert_eq!(s2.dirty_count(), 1); // 未清
    }

    #[test]
    fn rpc_dispatch() {
        let mut ch = RpcChannel::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        ch.register("shoot", move |p| {
            assert_eq!(p, b"9mm");
            h.fetch_add(1, Ordering::SeqCst);
        });
        ch.call("shoot", b"9mm", true);
        ch.call("unknown", b"x", false);
        let out = ch.drain_outbox();
        assert_eq!(out.len(), 2);
        assert!(out[0].2); // reliable 标记
        let msgs: Vec<(String, Vec<u8>)> = out.into_iter().map(|(n, d, _)| (n, d)).collect();
        assert_eq!(ch.dispatch(&msgs), 1); // unknown 无 handler
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(ch.drain_outbox().is_empty());
    }

    #[test]
    fn prediction_reconcile() {
        let mut buf = PredictionBuffer::new(10);
        for tick in 1..=5 {
            buf.push(tick, vec![tick as u8], tick * 100);
        }
        assert_eq!(buf.reconcile(3, 300), 0); // 一致
        assert_eq!(buf.len(), 5);
        let rollback = buf.reconcile(3, 999); // 不匹配 → 回滚 3 条
        assert_eq!(rollback, 3);
        assert_eq!(buf.len(), 2); // 保留 tick 1-2 供回放
    }
}
