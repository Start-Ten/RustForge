//! RustForge 平台抽象（IF-120 ~ IF-129）：窗口/输入/文件/时间。
//! 实现：HeadlessPlatform（全平台可用，CI/测试）；Win32Platform（feature `win32`）。

pub mod fs;

#[cfg(all(windows, feature = "win32"))]
pub mod win32;

use rf_core::{EngineError, Result, Rgba8Image};
use std::time::Instant;

/// 窗口配置（IF-120）。
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub vsync: bool,
    pub fullscreen: bool,
    pub high_dpi: bool,
    pub visible: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "RustForge".into(),
            width: 800,
            height: 600,
            resizable: true,
            vsync: true,
            fullscreen: false,
            high_dpi: true,
            visible: true,
        }
    }
}

/// 窗口句柄（IF-121）。
pub type WindowId = u64;

/// 窗口事件（IF-121，可扩展）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowEvent {
    CloseRequested(WindowId),
    Resized(WindowId, u32, u32),
    Focused(WindowId, bool),
    Minimized(WindowId),
    ScaleChanged(WindowId, f32),
}

/// 键码（IF-122，可扩展）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Escape,
    Enter,
    Space,
    Left,
    Right,
    Up,
    Down,
    ShiftLeft,
    ShiftRight,
    CtrlLeft,
    CtrlRight,
    AltLeft,
    AltRight,
    Tab,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Comma,
    Period,
    Slash,
    Semicolon,
    Apostrophe,
    BracketLeft,
    BracketRight,
    Backslash,
    Minus,
    Equal,
    Backquote,
    CapsLock,
    Unknown,
}

/// 鼠标键（IF-123）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// 输入事件（IF-123，可扩展）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    KeyPressed(Key),
    KeyReleased(Key),
    TextInput(char),
    MouseMoved { window: WindowId, x: f32, y: f32 },
    MouseWheel { dx: f32, dy: f32 },
    MousePressed(MouseButton),
    MouseReleased(MouseButton),
    TouchBegin { id: u32, x: f32, y: f32 },
    TouchMove { id: u32, x: f32, y: f32 },
    TouchEnd { id: u32, x: f32, y: f32 },
    TouchCancel { id: u32 },
    GamepadConnected { id: u32 },
    GamepadDisconnected { id: u32 },
    GamepadButton { id: u32, button: u8, pressed: bool },
    GamepadAxis { id: u32, axis: u8, value: f32 },
}

/// 平台事件（IF-124，可扩展）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlatformEvent {
    Window(WindowEvent),
    Input(InputEvent),
    Suspended,
    Resumed,
    DestroyRequested,
    LowPower,
    Battery { level: Option<u8>, charging: bool },
}

/// 平台后端（IF-125，可扩展）。
pub trait Platform: Send {
    fn name(&self) -> &'static str;
    fn create_window(&mut self, cfg: WindowConfig) -> Result<WindowId>;
    fn window_size(&self, id: WindowId) -> (u32, u32);
    fn set_title(&mut self, id: WindowId, title: &str);
    /// 将 RGBA8 帧呈现到窗口（Software 后端渲染结果的出口）。
    fn present(&mut self, id: WindowId, frame: &Rgba8Image) -> Result<()>;
    /// 非阻塞轮询事件。
    fn poll_events(&mut self) -> Vec<PlatformEvent>;
    fn dpi_scale(&self, _id: WindowId) -> f32 {
        1.0
    }
    fn is_mobile(&self) -> bool {
        false
    }
    fn request_exit(&mut self);
}

/// 平台类型（IF-126）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformKind {
    Default,
    Headless,
    Win32,
    X11,
    Wayland,
    Android,
}

/// 创建平台（IF-126）。Win32 由 feature `win32` 门控（Windows 目标默认启用）。
pub fn create_platform(kind: PlatformKind) -> Result<Box<dyn Platform>> {
    match kind {
        PlatformKind::Headless => Ok(Box::new(HeadlessPlatform::new())),
        PlatformKind::Default => {
            #[cfg(all(windows, feature = "win32"))]
            {
                Ok(Box::new(win32::Win32Platform::new()))
            }
            #[cfg(not(all(windows, feature = "win32")))]
            {
                Ok(Box::new(HeadlessPlatform::new()))
            }
        }
        #[cfg(all(windows, feature = "win32"))]
        PlatformKind::Win32 => Ok(Box::new(win32::Win32Platform::new())),
        #[cfg(not(all(windows, feature = "win32")))]
        PlatformKind::Win32 => {
            Err(EngineError::Unsupported("win32 platform requires windows + feature `win32`"))
        }
        PlatformKind::X11 | PlatformKind::Wayland | PlatformKind::Android => {
            Err(EngineError::NotYetSupported {
                what: "x11/wayland/android native window (headless 可用)",
                priority: "P1",
            })
        }
    }
}

/// 无窗口平台：事件可注入、present 记录最后一帧（测试/CI）。
pub struct HeadlessPlatform {
    windows: Vec<(WindowId, (u32, u32), String)>,
    next_id: WindowId,
    queued: Vec<PlatformEvent>,
    last_frame: Option<Rgba8Image>,
    exit_requested: bool,
}

impl HeadlessPlatform {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            next_id: 1,
            queued: Vec::new(),
            last_frame: None,
            exit_requested: false,
        }
    }

    /// 测试注入口。
    pub fn push_event(&mut self, ev: PlatformEvent) {
        self.queued.push(ev);
    }

    pub fn last_frame(&self) -> Option<&Rgba8Image> {
        self.last_frame.as_ref()
    }
}

impl Default for HeadlessPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for HeadlessPlatform {
    fn name(&self) -> &'static str {
        "headless"
    }

    fn create_window(&mut self, cfg: WindowConfig) -> Result<WindowId> {
        let id = self.next_id;
        self.next_id += 1;
        self.windows.push((id, (cfg.width, cfg.height), cfg.title));
        Ok(id)
    }

    fn window_size(&self, id: WindowId) -> (u32, u32) {
        self.windows.iter().find(|(i, ..)| *i == id).map(|(_, s, _)| *s).unwrap_or((0, 0))
    }

    fn set_title(&mut self, id: WindowId, title: &str) {
        if let Some(w) = self.windows.iter_mut().find(|(i, ..)| *i == id) {
            w.2 = title.to_string();
        }
    }

    fn present(&mut self, id: WindowId, frame: &Rgba8Image) -> Result<()> {
        if !self.windows.iter().any(|(i, ..)| *i == id) {
            return Err(EngineError::Message("present: unknown window".into()));
        }
        self.last_frame = Some(frame.clone());
        Ok(())
    }

    fn poll_events(&mut self) -> Vec<PlatformEvent> {
        std::mem::take(&mut self.queued)
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
        self.queued.push(PlatformEvent::DestroyRequested);
    }
}

/// 帧计时器（IF-128）。
pub struct FrameTimer {
    last: Instant,
    times: std::collections::VecDeque<f32>,
    frames: u64,
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameTimer {
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
            times: std::collections::VecDeque::with_capacity(120),
            frames: 0,
        }
    }

    /// 返回 dt（秒，上限 0.25 防断点尖峰）。
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.25);
        self.last = now;
        self.frames += 1;
        self.times.push_back(dt);
        if self.times.len() > 120 {
            self.times.pop_front();
        }
        dt
    }

    pub fn fps(&self) -> f32 {
        if self.times.is_empty() {
            return 0.0;
        }
        let avg: f32 = self.times.iter().sum::<f32>() / self.times.len() as f32;
        if avg > 0.0 {
            1.0 / avg
        } else {
            0.0
        }
    }

    pub fn frame_count(&self) -> u64 {
        self.frames
    }
}

/// 系统信息（IF-128）。
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cores: u32,
    pub memory_mb: u64,
}

pub fn system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        cores: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1),
        memory_mb: 0, // 跨平台内存总量查询列 P1（Windows GlobalMemoryStatusEx）
    }
}

/// 剪贴板（IF-129）。
pub trait Clipboard {
    fn set_text(&mut self, text: &str);
    fn text(&mut self) -> Option<String>;
}

/// 内存剪贴板（测试）。
#[derive(Default)]
pub struct HeadlessClipboard {
    content: Option<String>,
}

impl Clipboard for HeadlessClipboard {
    fn set_text(&mut self, text: &str) {
        self.content = Some(text.to_string());
    }
    fn text(&mut self) -> Option<String> {
        self.content.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_window_lifecycle() {
        let mut p = HeadlessPlatform::new();
        let id =
            p.create_window(WindowConfig { width: 64, height: 48, ..Default::default() }).unwrap();
        assert_eq!(p.window_size(id), (64, 48));
        p.set_title(id, "X");
        let frame = Rgba8Image::filled(64, 48, [255, 0, 0, 255]);
        p.present(id, &frame).unwrap();
        assert_eq!(p.last_frame().unwrap().get(0, 0), Some([255, 0, 0, 255]));
        assert!(p.present(999, &frame).is_err());
    }

    #[test]
    fn headless_events() {
        let mut p = HeadlessPlatform::new();
        p.push_event(PlatformEvent::Suspended);
        p.push_event(PlatformEvent::Input(InputEvent::KeyPressed(Key::Escape)));
        let evs = p.poll_events();
        assert_eq!(evs.len(), 2);
        assert!(p.poll_events().is_empty()); // 取尽
    }

    #[test]
    fn create_platform_headless() {
        let mut p = create_platform(PlatformKind::Headless).unwrap();
        assert_eq!(p.name(), "headless");
        assert!(!p.is_mobile());
        let id = p.create_window(Default::default()).unwrap();
        assert_eq!(p.window_size(id), (800, 600));
    }

    #[test]
    fn frame_timer() {
        let mut t = FrameTimer::new();
        let dt = t.tick();
        assert!((0.0..=0.25).contains(&dt));
        assert_eq!(t.frame_count(), 1);
    }

    #[test]
    fn clipboard_roundtrip() {
        let mut c = HeadlessClipboard::default();
        c.set_text("hello");
        assert_eq!(c.text().as_deref(), Some("hello"));
    }
}
