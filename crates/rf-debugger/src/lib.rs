//! RustForge 调试器（IF-300 ~ IF-305）：日志、控制台、Profiler、远程调试接口。

use rf_core::Result;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

// ---- 日志（IF-301） ----

/// 日志级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" | "WARNING" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// 日志条目。
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub time_ms: u64,
    pub level: LogLevel,
    pub category: String,
    pub message: String,
}

/// 日志输出 sink。
pub type LogSink = Box<dyn Fn(&LogEntry) + Send + Sync>;

/// 日志器：级别过滤 + 环形缓冲 + 外部 sink。
pub struct Logger {
    level: LogLevel,
    ring: VecDeque<LogEntry>,
    capacity: usize,
    start: Instant,
    sinks: Vec<LogSink>,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    pub fn new() -> Self {
        Self {
            level: LogLevel::Info,
            ring: VecDeque::new(),
            capacity: 4096,
            start: Instant::now(),
            sinks: Vec::new(),
        }
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    pub fn set_ring_capacity(&mut self, cap: usize) {
        self.capacity = cap.max(16);
    }

    pub fn add_sink(&mut self, sink: Box<dyn Fn(&LogEntry) + Send + Sync>) {
        self.sinks.push(sink);
    }

    pub fn log(&mut self, level: LogLevel, category: &str, message: String) {
        if level < self.level {
            return;
        }
        let entry = LogEntry {
            time_ms: self.start.elapsed().as_millis() as u64,
            level,
            category: category.to_string(),
            message,
        };
        self.ring.push_back(entry.clone());
        while self.ring.len() > self.capacity {
            self.ring.pop_front();
        }
        for sink in &self.sinks {
            sink(&entry);
        }
    }

    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.ring
    }

    pub fn filter(&self, min_level: LogLevel, category_contains: &str) -> Vec<LogEntry> {
        self.ring
            .iter()
            .filter(|e| e.level >= min_level && e.category.contains(category_contains))
            .cloned()
            .collect()
    }
}

/// 全局日志（IF-301）。
pub fn init_global() {
    let _ = global();
}

pub fn global() -> &'static Mutex<Logger> {
    static G: OnceLock<Mutex<Logger>> = OnceLock::new();
    G.get_or_init(|| Mutex::new(Logger::new()))
}

/// 日志宏。
#[macro_export]
macro_rules! rf_log {
    ($level:expr, $category:expr, $($arg:tt)*) => {
        $crate::global().lock().map(|mut l| l.log($level, $category, format!($($arg)*))).ok()
    };
}

#[macro_export]
macro_rules! rf_trace {
    ($category:expr, $($arg:tt)*) => {
        $crate::rf_log!($crate::LogLevel::Trace, $category, $($arg)*)
    };
}

#[macro_export]
macro_rules! rf_debug {
    ($category:expr, $($arg:tt)*) => {
        $crate::rf_log!($crate::LogLevel::Debug, $category, $($arg)*)
    };
}

#[macro_export]
macro_rules! rf_info {
    ($category:expr, $($arg:tt)*) => {
        $crate::rf_log!($crate::LogLevel::Info, $category, $($arg)*)
    };
}

#[macro_export]
macro_rules! rf_warn {
    ($category:expr, $($arg:tt)*) => {
        $crate::rf_log!($crate::LogLevel::Warn, $category, $($arg)*)
    };
}

#[macro_export]
macro_rules! rf_error {
    ($category:expr, $($arg:tt)*) => {
        $crate::rf_log!($crate::LogLevel::Error, $category, $($arg)*)
    };
}

// ---- 控制台（IF-302） ----

/// CVar 值。
#[derive(Debug, Clone, PartialEq)]
pub enum CVarValue {
    Bool(bool),
    F32(f32),
    I64(i64),
    Str(String),
}

impl std::fmt::Display for CVarValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CVarValue::Bool(b) => write!(f, "{b}"),
            CVarValue::F32(v) => write!(f, "{v}"),
            CVarValue::I64(v) => write!(f, "{v}"),
            CVarValue::Str(s) => write!(f, "{s}"),
        }
    }
}

/// CVar。
#[derive(Debug, Clone)]
pub struct CVar {
    pub value: CVarValue,
    pub default: CVarValue,
    pub desc: String,
}

type CommandFn = Box<dyn Fn(&[&str]) -> String + Send + Sync>;

/// 控制台：cvar + 命令 + 历史记录 + 自动补全。
pub struct Console {
    cvars: HashMap<String, CVar>,
    commands: HashMap<String, (String, CommandFn)>,
    pub history: Vec<String>,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    pub fn new() -> Self {
        let mut c = Self { cvars: HashMap::new(), commands: HashMap::new(), history: Vec::new() };
        c.register_command(
            "help",
            "列出全部命令",
            Box::new(|args| {
                let _ = args;
                "commands: help, list_cvars, find <text>, set <name> <value>, get <name>, clear"
                    .into()
            }),
        );
        c.register_command(
            "list_cvars",
            "列出全部 cvar",
            Box::new(|_| "use find <text> to filter cvars".into()),
        );
        c.register_command("clear", "清空历史", Box::new(|_| String::new()));
        c
    }

    pub fn register_cvar(&mut self, name: &str, desc: &str, default: CVarValue) {
        self.cvars.insert(
            name.to_string(),
            CVar { value: default.clone(), default, desc: desc.to_string() },
        );
    }

    pub fn cvar(&self, name: &str) -> Option<&CVar> {
        self.cvars.get(name)
    }

    pub fn set_cvar(&mut self, name: &str, value: CVarValue) -> Result<()> {
        let cv = self
            .cvars
            .get_mut(name)
            .ok_or_else(|| rf_core::EngineError::Message(format!("unknown cvar `{name}`")))?;
        // 类型校验
        match (&cv.value, &value) {
            (CVarValue::Bool(_), CVarValue::Bool(_))
            | (CVarValue::F32(_), CVarValue::F32(_))
            | (CVarValue::I64(_), CVarValue::I64(_))
            | (CVarValue::Str(_), CVarValue::Str(_)) => {
                cv.value = value;
                Ok(())
            }
            _ => Err(rf_core::EngineError::Message(format!("cvar `{name}` type mismatch"))),
        }
    }

    pub fn register_command(&mut self, name: &str, help: &str, f: CommandFn) {
        self.commands.insert(name.to_string(), (help.to_string(), f));
    }

    /// 执行一行命令：内建 set/get/find + 注册命令。
    pub fn execute(&mut self, line: &str) -> Result<String> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(String::new());
        }
        self.history.push(line.to_string());
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            "set" => {
                if parts.len() != 3 {
                    return Err(rf_core::EngineError::Message("usage: set <name> <value>".into()));
                }
                let cv = self.cvar(parts[1]).ok_or_else(|| {
                    rf_core::EngineError::Message(format!("unknown cvar `{}`", parts[1]))
                })?;
                let value = match cv.value {
                    CVarValue::Bool(_) => CVarValue::Bool(
                        parts[2]
                            .parse()
                            .map_err(|_| rf_core::EngineError::Message("bad bool".into()))?,
                    ),
                    CVarValue::F32(_) => CVarValue::F32(
                        parts[2]
                            .parse()
                            .map_err(|_| rf_core::EngineError::Message("bad f32".into()))?,
                    ),
                    CVarValue::I64(_) => CVarValue::I64(
                        parts[2]
                            .parse()
                            .map_err(|_| rf_core::EngineError::Message("bad i64".into()))?,
                    ),
                    CVarValue::Str(_) => CVarValue::Str(parts[2].to_string()),
                };
                self.set_cvar(parts[1], value)?;
                Ok(format!("{} = {}", parts[1], self.cvar(parts[1]).unwrap().value))
            }
            "get" => {
                let cv = self
                    .cvar(parts.get(1).copied().unwrap_or(""))
                    .ok_or_else(|| rf_core::EngineError::Message("unknown cvar".into()))?;
                Ok(format!("{}", cv.value))
            }
            "find" => {
                let needle = parts.get(1).copied().unwrap_or("");
                let mut names: Vec<&String> =
                    self.cvars.keys().filter(|n| n.contains(needle)).collect();
                names.sort();
                Ok(names
                    .iter()
                    .map(|n| format!("{} = {}  # {}", n, self.cvars[*n].value, self.cvars[*n].desc))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
            "list_cvars" => {
                let mut names: Vec<&String> = self.cvars.keys().collect();
                names.sort();
                Ok(names.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" "))
            }
            name => {
                let (help, cmd) = self.commands.get(name).ok_or_else(|| {
                    rf_core::EngineError::Message(format!(
                        "unknown command `{name}`（{help}）",
                        help = "试 help"
                    ))
                })?;
                let _ = help;
                Ok(cmd(&parts[1..]))
            }
        }
    }

    /// 前缀自动补全（内建命令 + 注册命令 + cvar）。
    pub fn autocomplete(&self, prefix: &str) -> Vec<String> {
        const BUILTIN: [&str; 4] = ["set", "get", "find", "help"];
        let mut out: Vec<String> = BUILTIN
            .into_iter()
            .chain(self.commands.keys().map(|s| s.as_str()))
            .chain(self.cvars.keys().map(|s| s.as_str()))
            .filter(|n| n.starts_with(prefix))
            .map(|s| format!("{s} "))
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

// ---- Profiler（IF-303） ----

/// 火焰事件。
#[derive(Debug, Clone)]
pub struct FlameEvent {
    pub name: String,
    pub start_ns: u64,
    pub dur_ns: u64,
    pub depth: u32,
}

#[derive(Debug, Clone, Default)]
struct ScopeStats {
    total_ns: u64,
    count: u64,
    last_ns: u64,
    max_ns: u64,
}

/// CPU Profiler：作用域计时 + 计数器 + 帧统计 + 火焰数据。
pub struct Profiler {
    scopes: HashMap<String, ScopeStats>,
    counters: HashMap<String, f64>,
    frame_times: VecDeque<f32>,
    flame: VecDeque<FlameEvent>,
    depth: u32,
    start: Instant,
    frames: u64,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
            counters: HashMap::new(),
            frame_times: VecDeque::with_capacity(120),
            flame: VecDeque::with_capacity(2048),
            depth: 0,
            start: Instant::now(),
            frames: 0,
        }
    }

    /// 【CP-003 变更提案】作用域计时改为线程本地全局（原 `&mut self` 版本无法嵌套）。
    fn end_scope(&mut self, name: &str, begin: Instant, depth: u32) {
        self.depth = self.depth.saturating_sub(1);
        let dur = begin.elapsed().as_nanos() as u64;
        let e = self.scopes.entry(name.to_string()).or_default();
        e.total_ns += dur;
        e.count += 1;
        e.last_ns = dur;
        e.max_ns = e.max_ns.max(dur);
        self.flame.push_back(FlameEvent {
            name: name.to_string(),
            start_ns: begin.duration_since(self.start).as_nanos() as u64,
            dur_ns: dur,
            depth,
        });
        while self.flame.len() > 2048 {
            self.flame.pop_front();
        }
    }

    pub fn counter_add(&mut self, name: &str, delta: f64) {
        *self.counters.entry(name.to_string()).or_insert(0.0) += delta;
    }

    pub fn counter_set(&mut self, name: &str, value: f64) {
        self.counters.insert(name.to_string(), value);
    }

    pub fn counters(&self) -> Vec<(String, f64)> {
        let mut v: Vec<_> = self.counters.iter().map(|(k, x)| (k.clone(), *x)).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    pub fn frame_start(&mut self) {
        self.frames += 1;
    }

    pub fn frame_end(&mut self, dt: f32) {
        self.frame_times.push_back(dt);
        if self.frame_times.len() > 120 {
            self.frame_times.pop_front();
        }
    }

    pub fn fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        let avg: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        if avg > 0.0 {
            1.0 / avg
        } else {
            0.0
        }
    }

    pub fn frame_times(&self) -> &VecDeque<f32> {
        &self.frame_times
    }

    pub fn flame_events(&self) -> impl Iterator<Item = &FlameEvent> {
        self.flame.iter()
    }

    pub fn export_text(&self) -> String {
        let mut s = format!("profiler: {} frames, {:.1} fps\n", self.frames, self.fps());
        let mut scopes: Vec<_> = self.scopes.iter().collect();
        scopes.sort_by_key(|(_, e)| std::cmp::Reverse(e.total_ns));
        for (name, e) in scopes.iter().take(20) {
            s.push_str(&format!(
                "  {name:<32} total {:>10.3}ms n{:>6} avg {:>8.3}ms max {:>8.3}ms\n",
                e.total_ns as f64 / 1e6,
                e.count,
                e.last_ns as f64 / 1e6,
                e.max_ns as f64 / 1e6
            ));
        }
        for (k, v) in self.counters() {
            s.push_str(&format!("  counter {k} = {v}\n"));
        }
        s
    }
}

// 线程本地 Profiler（作用域计时用）。
thread_local! {
    static TL_PROFILER: std::cell::RefCell<Profiler> = std::cell::RefCell::new(Profiler::new());
}

/// 计时作用域守卫（嵌套安全）。用法：`let _g = rf_debugger::scope("physics");`
pub fn scope(name: &'static str) -> ScopeGuard {
    TL_PROFILER.with_borrow_mut(|p| p.depth += 1);
    ScopeGuard { name, begin: Instant::now() }
}

/// 作用域守卫。
pub struct ScopeGuard {
    name: &'static str,
    begin: Instant,
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        TL_PROFILER.with_borrow_mut(|p| {
            let depth = p.depth;
            p.end_scope(self.name, self.begin, depth);
        });
    }
}

/// 读取线程本地 Profiler 快照（导出/面板）。
pub fn tl_profiler_snapshot(f: impl FnOnce(&Profiler)) {
    TL_PROFILER.with_borrow(f);
}

// ---- 渲染统计（IF-305） ----

/// 渲染统计（Profiler 展示）。
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    pub sprites: u32,
    pub draw_calls_2d: u32,
    pub batches: u32,
    pub texture_switches: u32,
    pub triangles: u32,
    pub passes: u32,
}

// ---- 远程调试（IF-304） ----

/// 远程调试传输。
pub trait DebugTransport {
    fn send(&mut self, data: &[u8]) -> Result<()>;
    fn recv(&mut self) -> Vec<Vec<u8>>;
}

/// 内存回环传输对（测试 + adb forward 文档示例）：双向队列互通。
pub struct LoopbackTransport {
    to_peer: std::sync::Arc<Mutex<Vec<Vec<u8>>>>,
    from_peer: std::sync::Arc<Mutex<Vec<Vec<u8>>>>,
}

impl LoopbackTransport {
    pub fn pair() -> (Self, Self) {
        let a2b = std::sync::Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        let b2a = std::sync::Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
        (
            Self { to_peer: a2b.clone(), from_peer: b2a.clone() },
            Self { to_peer: b2a, from_peer: a2b },
        )
    }
}

impl DebugTransport for LoopbackTransport {
    fn send(&mut self, data: &[u8]) -> Result<()> {
        self.to_peer.lock().unwrap().push(data.to_vec());
        Ok(())
    }

    fn recv(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.from_peer.lock().unwrap())
    }
}

/// 远程调试服务（JSON 行协议：`stats` / `exec <cmd>`）。
pub struct RemoteDebugServer {
    transport: Box<dyn DebugTransport>,
}

impl RemoteDebugServer {
    pub fn new(transport: Box<dyn DebugTransport>) -> Self {
        Self { transport }
    }

    /// 轮询请求并响应（引擎帧循环调用）。
    pub fn poll_request_and_respond(
        &mut self,
        console: &mut Console,
        profiler: &Profiler,
    ) -> usize {
        let requests = self.transport.recv();
        let n = requests.len();
        for req in requests {
            let line = String::from_utf8_lossy(&req).trim().to_string();
            let response = if line == "stats" {
                profiler.export_text()
            } else if let Some(cmd) = line.strip_prefix("exec ") {
                console.execute(cmd).unwrap_or_else(|e| format!("error: {e}"))
            } else {
                format!("unknown request: {line}")
            };
            let _ = self.transport.send(format!("{response}\n").as_bytes());
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger_levels_and_filter() {
        let mut l = Logger::new();
        l.set_level(LogLevel::Info);
        l.log(LogLevel::Debug, "noise", "hidden".into()); // 被级别过滤
        l.log(LogLevel::Warn, "physics", "w1".into());
        l.log(LogLevel::Error, "render", "e1".into());
        assert_eq!(l.entries().len(), 2);
        let warns = l.filter(LogLevel::Warn, "phys");
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0].message, "w1");
        // 环形容量
        l.set_ring_capacity(16);
        for i in 0..100 {
            l.log(LogLevel::Error, "x", format!("m{i}"));
        }
        assert!(l.entries().len() <= 16);
        // 宏
        rf_info!("test", "hello {}", 42);
        let g = global().lock().unwrap();
        assert!(g.entries().iter().any(|e| e.message.contains("hello 42")));
    }

    #[test]
    fn log_level_parse() {
        assert_eq!(LogLevel::parse("warn"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse("ERROR"), Some(LogLevel::Error));
        assert_eq!(LogLevel::parse("bogus"), None);
    }

    #[test]
    fn console_cvars_and_commands() {
        let mut c = Console::new();
        c.register_cvar("r_vsync", "垂直同步", CVarValue::Bool(true));
        c.register_cvar("fov", "视场角", CVarValue::F32(75.0));
        assert!(c.execute("set fov 90.0").is_ok());
        assert_eq!(c.cvar("fov").unwrap().value, CVarValue::F32(90.0));
        assert!(c.execute("set fov notanumber").is_err()); // 类型错误
        assert!(c.execute("set missing 1").is_err());
        assert!(c.execute("get fov").unwrap().contains("90"));
        assert!(c.execute("find fo").unwrap().contains("fov"));
        // 自定义命令
        c.register_command("echo2", "双回声", Box::new(|args| args.join("!")));
        assert_eq!(c.execute("echo2 a b").unwrap(), "a!b");
        assert!(c.execute("nope").is_err());
        // 补全
        assert!(c.autocomplete("f").contains(&"find ".to_string()));
        assert!(!c.history.is_empty());
    }

    #[test]
    fn profiler_scopes_and_counters() {
        {
            let _g = scope("frame");
            {
                let _g2 = scope("physics");
                std::thread::sleep(std::time::Duration::from_micros(200));
            }
        }
        let mut p = Profiler::new();
        tl_profiler_snapshot(|tl| {
            p.scopes = tl.scopes.clone();
            p.flame = tl.flame.clone();
        });
        p.counter_add("sprites", 10.0);
        p.counter_add("sprites", 5.0);
        p.counter_set("fps", 60.0);
        p.frame_start();
        p.frame_end(1.0 / 60.0);
        assert!(p.fps() > 55.0 && p.fps() < 65.0);
        let counters = p.counters();
        assert!(counters.contains(&("sprites".to_string(), 15.0)));
        let report = p.export_text();
        assert!(report.contains("frame"));
        assert!(report.contains("physics"));
        let flame: Vec<&FlameEvent> = p.flame_events().collect();
        assert_eq!(flame.len(), 2);
        assert_eq!(flame[0].depth, 2); // physics（先完成）
        assert_eq!(flame[1].depth, 1); // frame（后完成）
    }

    #[test]
    fn remote_debug_protocol() {
        let (mut client, server_side) = LoopbackTransport::pair();
        let mut server = RemoteDebugServer::new(Box::new(server_side));
        let mut console = Console::new();
        console.register_cvar("g_speed", "速度", CVarValue::F32(1.0));
        let mut profiler = Profiler::new();
        profiler.frame_end(1.0 / 60.0);
        // 客户端发送请求（client.send → server.recv）
        client.send(b"stats").unwrap();
        client.send(b"exec get g_speed").unwrap();
        let handled = server.poll_request_and_respond(&mut console, &profiler);
        assert_eq!(handled, 2);
        // 响应回流（server.send → client.recv）
        let responses = client.recv();
        assert_eq!(responses.len(), 2);
        assert!(String::from_utf8_lossy(&responses[0]).contains("fps"));
        assert!(String::from_utf8_lossy(&responses[1]).contains("1"));
    }
}
