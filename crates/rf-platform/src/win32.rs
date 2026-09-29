//! Win32 平台后端（feature `win32`，仅 Windows）：窗口、消息循环、GDI 呈现。

use crate::{InputEvent, Platform, PlatformEvent, WindowConfig, WindowEvent, WindowId};
use rf_core::{EngineError, Result, Rgba8Image};
use std::collections::HashMap;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetDC, InvalidateRect, ReleaseDC, StretchDIBits, BITMAPINFO,
    BITMAPINFOHEADER, DIB_RGB_COLORS, PAINTSTRUCT, SRCCOPY,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, PeekMessageW,
    PostQuitMessage, RegisterClassW, SetWindowTextW, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, MSG, PM_REMOVE, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

// windows-sys 0.59 的 WM_* 为 newtype 常量；match(u32) 使用本地字面量。
const WM_DESTROY: u32 = 0x0002;
const WM_CLOSE: u32 = 0x0010;
const WM_QUIT: u32 = 0x0012;
const WM_SIZE: u32 = 0x0005;
const WM_PAINT: u32 = 0x000F;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_CHAR: u32 = 0x0102;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_MBUTTONDOWN: u32 = 0x0207;
const WM_MBUTTONUP: u32 = 0x0208;
const SIZE_MINIMIZED: usize = 1;

/// 可跨线程持有的 HWND（Win32 句柄本身线程无关；访问由 STATE 锁串行化）。
#[derive(Clone, Copy)]
struct SendHwnd(HWND);
unsafe impl Send for SendHwnd {}

/// 全局状态（Win32 回调无法携带用户数据的标准 workaround）。
struct Win32State {
    next_id: WindowId,
    windows: HashMap<WindowId, SendHwnd>,
    sizes: HashMap<WindowId, (u32, u32)>,
    events: Vec<PlatformEvent>,
    exit: bool,
}

static STATE: std::sync::Mutex<Option<Win32State>> = std::sync::Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut Win32State) -> R) -> R {
    let mut guard = STATE.lock().unwrap();
    if guard.is_none() {
        *guard = Some(Win32State {
            next_id: 1,
            windows: HashMap::new(),
            sizes: HashMap::new(),
            events: Vec::new(),
            exit: false,
        });
    }
    f(guard.as_mut().unwrap())
}

fn vk_to_key(vk: u32) -> crate::Key {
    use crate::Key::*;
    match vk {
        0x41 => A,
        0x42 => B,
        0x43 => C,
        0x44 => D,
        0x45 => E,
        0x46 => F,
        0x47 => G,
        0x48 => H,
        0x49 => I,
        0x4A => J,
        0x4B => K,
        0x4C => L,
        0x4D => M,
        0x4E => N,
        0x4F => O,
        0x50 => P,
        0x51 => Q,
        0x52 => R,
        0x53 => S,
        0x54 => T,
        0x55 => U,
        0x56 => V,
        0x57 => W,
        0x58 => X,
        0x59 => Y,
        0x5A => Z,
        0x30 => Num0,
        0x31 => Num1,
        0x32 => Num2,
        0x33 => Num3,
        0x34 => Num4,
        0x35 => Num5,
        0x36 => Num6,
        0x37 => Num7,
        0x38 => Num8,
        0x39 => Num9,
        0x70 => F1,
        0x71 => F2,
        0x72 => F3,
        0x73 => F4,
        0x74 => F5,
        0x75 => F6,
        0x76 => F7,
        0x77 => F8,
        0x78 => F9,
        0x79 => F10,
        0x7A => F11,
        0x7B => F12,
        0x1B => Escape,
        0x0D => Enter,
        0x20 => Space,
        0x25 => Left,
        0x26 => Up,
        0x27 => Right,
        0x28 => Down,
        0xA0 | 0xA1 => ShiftLeft,
        0xA2 | 0xA3 => CtrlLeft,
        0xA4 | 0xA5 => AltLeft,
        0x09 => Tab,
        0x08 => Backspace,
        0x2E => Delete,
        0x24 => Home,
        0x23 => End,
        0x21 => PageUp,
        0x22 => PageDown,
        0xBC => Comma,
        0xBE => Period,
        0xBF => Slash,
        0xBA => Semicolon,
        0xDE => Apostrophe,
        0xDB => BracketLeft,
        0xDD => BracketRight,
        0xDC => Backslash,
        0xBD => Minus,
        0xBB => Equal,
        0xC0 => Backquote,
        0x14 => CapsLock,
        _ => crate::Key::Unknown,
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CLOSE => {
            with_state(|s| {
                if let Some(&id) = s.windows.iter().find(|(_, h)| h.0 == hwnd).map(|(id, _)| id) {
                    s.events.push(PlatformEvent::Window(WindowEvent::CloseRequested(id)));
                }
            });
            0
        }
        WM_DESTROY => {
            with_state(|s| s.events.push(PlatformEvent::DestroyRequested));
            unsafe { PostQuitMessage(0) };
            0
        }
        WM_SIZE => {
            let w = (lparam as u32) & 0xFFFF;
            let h = ((lparam as u32) >> 16) & 0xFFFF;
            with_state(|s| {
                if let Some(&id) = s.windows.iter().find(|(_, hh)| hh.0 == hwnd).map(|(id, _)| id) {
                    s.sizes.insert(id, (w, h));
                    let ev = if w == 0 || h == 0 || wparam == SIZE_MINIMIZED {
                        WindowEvent::Minimized(id)
                    } else {
                        WindowEvent::Resized(id, w, h)
                    };
                    s.events.push(PlatformEvent::Window(ev));
                }
            });
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = (wparam as u32) & 0xFF;
            with_state(|s| {
                s.events.push(PlatformEvent::Input(InputEvent::KeyPressed(vk_to_key(vk))))
            });
            0
        }
        WM_KEYUP | WM_SYSKEYUP => {
            let vk = (wparam as u32) & 0xFF;
            with_state(|s| {
                s.events.push(PlatformEvent::Input(InputEvent::KeyReleased(vk_to_key(vk))))
            });
            0
        }
        WM_CHAR => {
            if let Some(c) = char::from_u32(wparam as u32).filter(|c| !c.is_control()) {
                with_state(|s| s.events.push(PlatformEvent::Input(InputEvent::TextInput(c))));
            }
            0
        }
        WM_MOUSEMOVE => {
            let x = (lparam as u16) as f32;
            let y = ((lparam as usize >> 16) & 0xFFFF) as u16 as f32;
            with_state(|s| {
                if let Some(&id) = s.windows.iter().find(|(_, h)| h.0 == hwnd).map(|(id, _)| id) {
                    s.events.push(PlatformEvent::Input(InputEvent::MouseMoved {
                        window: id,
                        x,
                        y,
                    }));
                }
            });
            0
        }
        WM_LBUTTONDOWN => {
            with_state(|s| {
                s.events
                    .push(PlatformEvent::Input(InputEvent::MousePressed(crate::MouseButton::Left)))
            });
            0
        }
        WM_LBUTTONUP => {
            with_state(|s| {
                s.events
                    .push(PlatformEvent::Input(InputEvent::MouseReleased(crate::MouseButton::Left)))
            });
            0
        }
        WM_RBUTTONDOWN => {
            with_state(|s| {
                s.events
                    .push(PlatformEvent::Input(InputEvent::MousePressed(crate::MouseButton::Right)))
            });
            0
        }
        WM_RBUTTONUP => {
            with_state(|s| {
                s.events.push(PlatformEvent::Input(InputEvent::MouseReleased(
                    crate::MouseButton::Right,
                )))
            });
            0
        }
        WM_MBUTTONDOWN => {
            with_state(|s| {
                s.events.push(PlatformEvent::Input(InputEvent::MousePressed(
                    crate::MouseButton::Middle,
                )))
            });
            0
        }
        WM_MBUTTONUP => {
            with_state(|s| {
                s.events.push(PlatformEvent::Input(InputEvent::MouseReleased(
                    crate::MouseButton::Middle,
                )))
            });
            0
        }
        WM_PAINT => {
            // 标准绘制循环（呈现由 present 驱动）
            unsafe {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                let _ = hdc;
                EndPaint(hwnd, &ps);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Win32 平台。
pub struct Win32Platform {
    class_registered: bool,
}

impl Win32Platform {
    pub fn new() -> Self {
        Self { class_registered: false }
    }

    fn ensure_class(&mut self) {
        if self.class_registered {
            return;
        }
        unsafe {
            let class_name: Vec<u16> = "RustForgeWindow\0".encode_utf16().collect();
            let mut wc: WNDCLASSW = std::mem::zeroed();
            wc.style = CS_HREDRAW | CS_VREDRAW;
            wc.lpfnWndProc = Some(wnd_proc);
            wc.hInstance = GetModuleHandleW(std::ptr::null());
            wc.lpszClassName = class_name.as_ptr();
            RegisterClassW(&wc);
        }
        self.class_registered = true;
    }
}

impl Default for Win32Platform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for Win32Platform {
    fn name(&self) -> &'static str {
        "win32"
    }

    fn create_window(&mut self, cfg: WindowConfig) -> Result<WindowId> {
        self.ensure_class();
        let id = with_state(|s| {
            let id = s.next_id;
            s.next_id += 1;
            id
        });
        unsafe {
            let class_name: Vec<u16> = "RustForgeWindow\0".encode_utf16().collect();
            let title: Vec<u16> = format!("{}\0", cfg.title).encode_utf16().collect();
            let style: u32 = WS_OVERLAPPEDWINDOW | if cfg.visible { WS_VISIBLE } else { 0 };
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                cfg.width as i32,
                cfg.height as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return Err(EngineError::Message("CreateWindowExW failed".into()));
            }
            let mut rect =
                RECT { left: 0, top: 0, right: cfg.width as i32, bottom: cfg.height as i32 };
            let _ = GetClientRect(hwnd, &mut rect);
            with_state(|s| {
                s.windows.insert(id, SendHwnd(hwnd));
                s.sizes.insert(
                    id,
                    (
                        (rect.right - rect.left).max(1) as u32,
                        (rect.bottom - rect.top).max(1) as u32,
                    ),
                );
            });
        }
        Ok(id)
    }

    fn window_size(&self, id: WindowId) -> (u32, u32) {
        with_state(|s| s.sizes.get(&id).copied().unwrap_or((0, 0)))
    }

    fn set_title(&mut self, id: WindowId, title: &str) {
        if let Some(sh) = with_state(|s| s.windows.get(&id).copied()) {
            let hwnd = sh.0;
            let wtitle: Vec<u16> = format!("{title}\0").encode_utf16().collect();
            unsafe {
                SetWindowTextW(hwnd, wtitle.as_ptr());
            }
        }
    }

    fn present(&mut self, id: WindowId, frame: &Rgba8Image) -> Result<()> {
        let sh = with_state(|s| s.windows.get(&id).copied())
            .ok_or_else(|| EngineError::Message("present: unknown window".into()))?;
        let hwnd = sh.0;
        unsafe {
            let hdc = GetDC(hwnd);
            let mut rect: RECT = std::mem::zeroed();
            if GetClientRect(hwnd, &mut rect) == 0 {
                ReleaseDC(hwnd, hdc);
                return Err(EngineError::Message("GetClientRect failed".into()));
            }
            let w = (rect.right - rect.left).max(1);
            let h = (rect.bottom - rect.top).max(1);
            // RGBA → BGRA（GDI DIB 通道序）
            let mut bgra = vec![0u8; frame.data.len()];
            for (dst, px) in bgra.chunks_exact_mut(4).zip(frame.data.chunks_exact(4)) {
                dst[0] = px[2];
                dst[1] = px[1];
                dst[2] = px[0];
                dst[3] = px[3];
            }
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = frame.width as i32;
            bmi.bmiHeader.biHeight = -(frame.height as i32); // 自顶向下
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = 0; // BI_RGB
            StretchDIBits(
                hdc,
                0,
                0,
                w,
                h,
                0,
                0,
                frame.width as i32,
                frame.height as i32,
                bgra.as_ptr().cast(),
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            ReleaseDC(hwnd, hdc);
            let _ = InvalidateRect(hwnd, std::ptr::null(), 0);
        }
        Ok(())
    }

    fn poll_events(&mut self) -> Vec<PlatformEvent> {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
                if msg.message == WM_QUIT {
                    with_state(|s| s.exit = true);
                }
            }
        }
        with_state(|s| std::mem::take(&mut s.events))
    }

    fn dpi_scale(&self, _id: WindowId) -> f32 {
        // DPI 感知清单未设时系统统一 96 DPI → 1.0；P1 接 GetDpiForWindow
        1.0
    }

    fn request_exit(&mut self) {
        with_state(|s| {
            s.exit = true;
            s.events.push(PlatformEvent::DestroyRequested);
        });
    }
}
