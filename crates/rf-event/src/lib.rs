//! RustForge 事件层（IF-050 ~ IF-052）：类型安全总线 + 输入映射。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 事件标记 trait（IF-050）。
pub trait Event: Send + Sync + 'static {}

/// 订阅优先级（Critical → Low 依次执行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Priority {
    Critical,
    High,
    #[default]
    Normal,
    Low,
}

/// 订阅凭据（IF-051）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubId(pub u64);

/// 类型擦除处理器：内部按 TypeId 下转回具体事件。
type ErasedHandler = Arc<dyn Fn(&(dyn Any + Send + Sync)) + Send + Sync>;

struct Entry {
    handler: ErasedHandler,
    priority: Priority,
    once: bool,
    fired: bool,
}

/// 类型安全事件总线（IF-051）。
/// 发布 `&self` 任意线程；订阅/退订 `&mut self`；同类型按优先级顺序执行。
pub struct EventBus {
    next_id: AtomicU64,
    slots: Mutex<Vec<(SubId, TypeId, Entry)>>,
    deferred: Mutex<Vec<Box<dyn FnOnce() + Send>>>,
    rate_limits: Mutex<HashMap<TypeId, (usize, usize)>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            slots: Mutex::new(Vec::new()),
            deferred: Mutex::new(Vec::new()),
            rate_limits: Mutex::new(HashMap::new()),
        }
    }

    /// 立即同步发布；限流计数超出时丢弃。
    pub fn publish<E: Event>(&self, event: &E) {
        let tid = TypeId::of::<E>();
        if let Ok(mut limits) = self.rate_limits.lock() {
            if let Some((max, used)) = limits.get_mut(&tid) {
                if *used >= *max {
                    return; // 事件风暴限流
                }
                *used += 1;
            }
        }
        let any = event as &(dyn Any + Send + Sync);
        let mut matched: Vec<Entry> = Vec::new();
        if let Ok(mut slots) = self.slots.lock() {
            for (_, t, entry) in slots.iter_mut() {
                if *t == tid && !entry.fired {
                    matched.push(Entry {
                        handler: entry.handler.clone(),
                        priority: entry.priority,
                        once: entry.once,
                        fired: false,
                    });
                    if entry.once {
                        entry.fired = true;
                    }
                }
            }
        }
        matched.sort_by_key(|e| e.priority);
        for mut e in matched {
            (e.handler)(any);
            let _ = &mut e;
        }
    }

    /// 延迟事件：入队闭包，帧末 `flush_deferred` 执行。
    pub fn publish_deferred<E: Event, F: FnOnce(&E) + Send + 'static>(&self, event: E, f: F) {
        let ev = Arc::new(event);
        if let Ok(mut q) = self.deferred.lock() {
            q.push(Box::new(move || f(&ev)));
        }
    }

    /// 冲洗延迟事件并重置限流计数。
    pub fn flush_deferred(&mut self) {
        let drained: Vec<_> = std::mem::take(&mut *self.deferred.lock().unwrap());
        for f in drained {
            f();
        }
        self.reset_rate_limits();
    }

    /// 订阅；返回 SubId。
    pub fn subscribe<E: Event, F: Fn(&E) + Send + Sync + 'static>(&mut self, f: F) -> SubId {
        let typed: Arc<dyn Fn(&E) + Send + Sync> = Arc::new(f);
        let erased: ErasedHandler = Arc::new(move |any: &(dyn Any + Send + Sync)| {
            if let Some(e) = any.downcast_ref::<E>() {
                typed(e);
            }
        });
        let id = SubId(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.slots.lock().unwrap().push((
            id,
            TypeId::of::<E>(),
            Entry { handler: erased, priority: Priority::Normal, once: false, fired: false },
        ));
        id
    }

    /// 一次性订阅：首次触发后自动失效。
    pub fn subscribe_once<E: Event, F: Fn(&E) + Send + Sync + 'static>(&mut self, f: F) -> SubId {
        let id = self.subscribe::<E, _>(f);
        if let Ok(mut slots) = self.slots.lock() {
            if let Some((_, _, e)) = slots.iter_mut().find(|(sid, _, _)| *sid == id) {
                e.once = true;
            }
        }
        id
    }

    /// 退订（幂等）。
    pub fn unsubscribe(&mut self, id: SubId) -> bool {
        let mut slots = self.slots.lock().unwrap();
        let before = slots.len();
        slots.retain(|(sid, _, _)| *sid != id);
        slots.len() < before
    }

    /// 调整优先级。
    pub fn set_priority(&mut self, id: SubId, p: Priority) -> bool {
        if let Ok(mut slots) = self.slots.lock() {
            if let Some((_, _, e)) = slots.iter_mut().find(|(sid, _, _)| *sid == id) {
                e.priority = p;
                return true;
            }
        }
        false
    }

    /// 每类型限流（两次 flush 之间最多投递 n 次）。
    pub fn set_rate_limit<E: Event>(&mut self, max_per_frame: usize) {
        self.rate_limits.lock().unwrap().insert(TypeId::of::<E>(), (max_per_frame, 0));
    }

    pub fn reset_rate_limits(&mut self) {
        if let Ok(mut l) = self.rate_limits.lock() {
            for v in l.values_mut() {
                v.1 = 0;
            }
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.slots.lock().map(|s| s.len()).unwrap_or(0)
    }
}

// ---- 输入映射（IF-052） ----

/// 动作触发类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Pressed,
    Released,
    Held,
}

/// 输入动作/轴事件。
#[derive(Debug, Clone, PartialEq)]
pub struct InputActionEvent {
    pub name: String,
    pub value: f32,
    pub kind: ActionKind,
}

/// 通用按键绑定映射（B = 平台按键码类型）。
pub struct InputMap<B: Copy + Eq + std::hash::Hash> {
    actions: HashMap<String, Vec<B>>,
    axes: HashMap<String, (B, B, f32)>,
    prev_state: HashMap<B, bool>,
}

impl<B: Copy + Eq + std::hash::Hash> Default for InputMap<B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: Copy + Eq + std::hash::Hash> InputMap<B> {
    pub fn new() -> Self {
        Self { actions: HashMap::new(), axes: HashMap::new(), prev_state: HashMap::new() }
    }

    pub fn bind_action(&mut self, name: &str, key: B) {
        self.actions.entry(name.to_string()).or_default().push(key);
    }

    /// 轴：值 = (positive按下 - negative按下) × scale。
    pub fn bind_axis(&mut self, name: &str, positive: B, negative: B, scale: f32) {
        self.axes.insert(name.to_string(), (positive, negative, scale));
    }

    /// 轮询当前按键状态，产出动作（Pressed/Released/Held）与轴值事件。
    pub fn poll(&mut self, is_down: impl Fn(&B) -> bool) -> Vec<InputActionEvent> {
        let mut out = Vec::new();
        let keys: Vec<B> = self
            .actions
            .values()
            .flatten()
            .chain(self.axes.values().flat_map(|(p, n, _)| [p, n]))
            .copied()
            .collect();
        let mut cur = HashMap::new();
        for k in keys {
            cur.insert(k, is_down(&k));
        }
        for (name, binds) in &self.actions {
            let any_down = binds.iter().any(|k| cur.get(k).copied().unwrap_or(false));
            let any_prev = binds.iter().any(|k| self.prev_state.get(k).copied().unwrap_or(false));
            let kind = match (any_prev, any_down) {
                (false, true) => ActionKind::Pressed,
                (true, false) => ActionKind::Released,
                (true, true) => ActionKind::Held,
                (false, false) => continue,
            };
            out.push(InputActionEvent {
                name: name.clone(),
                value: if any_down { 1.0 } else { 0.0 },
                kind,
            });
        }
        for (name, (pos, neg, scale)) in &self.axes {
            let v = (cur.get(pos).copied().unwrap_or(false) as i32
                - cur.get(neg).copied().unwrap_or(false) as i32) as f32
                * scale;
            out.push(InputActionEvent { name: name.clone(), value: v, kind: ActionKind::Held });
        }
        self.prev_state = cur;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[derive(Debug, Clone, Copy)]
    struct EvA(u32);
    impl Event for EvA {}

    #[derive(Debug, Clone, Copy)]
    struct EvB(#[allow(dead_code)] &'static str);
    impl Event for EvB {}

    #[test]
    fn publish_subscribe_order() {
        let mut bus = EventBus::new();
        let order = Arc::new(Mutex::new(Vec::new()));
        let o1 = order.clone();
        let id1 = bus.subscribe::<EvA, _>(move |e| o1.lock().unwrap().push(format!("1:{}", e.0)));
        let o2 = order.clone();
        let id2 = bus.subscribe::<EvA, _>(move |e| o2.lock().unwrap().push(format!("2:{}", e.0)));
        bus.set_priority(id2, Priority::High);
        let _ = id1;
        bus.publish(&EvA(5));
        assert_eq!(*order.lock().unwrap(), vec!["2:5", "1:5"]);
    }

    #[test]
    fn typed_isolation() {
        let mut bus = EventBus::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        bus.subscribe::<EvA, _>(move |_| {
            h.fetch_add(1, Ordering::Relaxed);
        });
        bus.publish(&EvB("wrong")); // 不同类型不触发
        assert_eq!(hits.load(Ordering::Relaxed), 0);
        bus.publish(&EvA(1));
        assert_eq!(hits.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn once_and_unsubscribe() {
        let mut bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let id_once = bus.subscribe_once::<EvA, _>(move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });
        bus.publish(&EvA(1));
        bus.publish(&EvA(2));
        assert_eq!(count.load(Ordering::Relaxed), 1); // 只触发一次
        assert!(bus.unsubscribe(id_once));
        assert!(!bus.unsubscribe(id_once)); // 幂等
    }

    #[test]
    fn deferred_flush() {
        let mut bus = EventBus::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = seen.clone();
        bus.subscribe::<EvA, _>(move |e| s.lock().unwrap().push(e.0));
        let seen2 = seen.clone();
        bus.publish_deferred(EvA(9), move |e| {
            seen2.lock().unwrap().push(e.0 * 10);
        });
        assert_eq!(seen.lock().unwrap().len(), 0); // 尚未 flush
        bus.flush_deferred();
        assert_eq!(*seen.lock().unwrap(), vec![90]); // 延迟回调已执行
    }

    #[test]
    fn rate_limit() {
        let mut bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        bus.subscribe::<EvA, _>(move |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });
        bus.set_rate_limit::<EvA>(3);
        for _ in 0..10 {
            bus.publish(&EvA(1));
        }
        assert_eq!(count.load(Ordering::Relaxed), 3); // 限流
        bus.reset_rate_limits();
        bus.publish(&EvA(1));
        assert_eq!(count.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn cross_thread_publish() {
        let mut bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));
        {
            let c = count.clone();
            bus.subscribe::<EvA, _>(move |_| {
                c.fetch_add(1, Ordering::Relaxed);
            });
        }
        let bus = Arc::new(bus); // 订阅完成后再共享（publish 只需 &self）
        let b = bus.clone();
        let t = std::thread::spawn(move || {
            for _ in 0..100 {
                b.publish(&EvA(1));
            }
        });
        t.join().unwrap();
        assert_eq!(count.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn input_map_press_release() {
        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        enum K {
            Space,
            Left,
            Right,
        }
        let mut map: InputMap<K> = InputMap::new();
        map.bind_action("jump", K::Space);
        map.bind_axis("move_x", K::Right, K::Left, 1.0);
        // 初始：无动作事件（轴持续上报 0 值是预期行为）
        assert!(map.poll(|_| false).iter().all(|e| e.kind == ActionKind::Held && e.value == 0.0));
        let evs = map.poll(|k| matches!(k, K::Space));
        assert_eq!(evs.len(), 2);
        assert!(evs.iter().any(|e| e.name == "jump" && e.kind == ActionKind::Pressed));
        let evs = map.poll(|k| matches!(k, K::Space));
        assert!(evs.iter().any(|e| e.name == "jump" && e.kind == ActionKind::Held));
        let evs = map.poll(|_| false);
        assert!(evs.iter().any(|e| e.name == "jump" && e.kind == ActionKind::Released));
        let evs = map.poll(|k| matches!(k, K::Right));
        assert!(evs.iter().any(|e| e.name == "move_x" && (e.value - 1.0).abs() < 1e-6));
    }
}
