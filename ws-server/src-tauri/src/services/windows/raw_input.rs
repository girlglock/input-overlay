#![cfg(windows)]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, SetThreadPriorityBoost, THREAD_PRIORITY_BELOW_NORMAL,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    LoadKeyboardLayoutW, MapVirtualKeyExW, MAPVK_VSC_TO_VK_EX,
};
use windows::Win32::UI::Input::{
    GetRawInputBuffer, RegisterRawInputDevices, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
    RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, PeekMessageW, PostMessageW, HWND_MESSAGE, MSG, PM_REMOVE,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_INPUT, WM_QUIT, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

use crate::services::consts::now_ms;
use crate::ws_server::InputEvent;

#[link(name = "winmm")]
unsafe extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
}

#[repr(C)]
struct OsVersionInfo {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    csd_version: [u16; 128],
    sp_major: u16,
    sp_minor: u16,
    suite_mask: u16,
    product_type: u8,
    reserved: u8,
}

fn needs_timer_boost() -> bool {
    unsafe {
        let mut info: OsVersionInfo = std::mem::zeroed();
        info.size = std::mem::size_of::<OsVersionInfo>() as u32;
        RtlGetVersion(&mut info);
        info.major > 10 || (info.major == 10 && info.build >= 19041)
    }
}

const RI_KEY_BREAK: u16 = 0x01;
const RI_KEY_E0: u16 = 0x02;
const RI_KEY_E1: u16 = 0x04;

const RI_MOUSE_LEFT_BUTTON_DOWN: u16 = 0x0001;
const RI_MOUSE_LEFT_BUTTON_UP: u16 = 0x0002;
const RI_MOUSE_RIGHT_BUTTON_DOWN: u16 = 0x0004;
const RI_MOUSE_RIGHT_BUTTON_UP: u16 = 0x0008;
const RI_MOUSE_MIDDLE_BUTTON_DOWN: u16 = 0x0010;
const RI_MOUSE_MIDDLE_BUTTON_UP: u16 = 0x0020;
const RI_MOUSE_BUTTON_4_DOWN: u16 = 0x0040;
const RI_MOUSE_BUTTON_4_UP: u16 = 0x0080;
const RI_MOUSE_BUTTON_5_DOWN: u16 = 0x0100;
const RI_MOUSE_BUTTON_5_UP: u16 = 0x0200;
const RI_MOUSE_WHEEL: u16 = 0x0400;

const BTN_MAP: &[(u16, u16, u8)] = &[
    (RI_MOUSE_LEFT_BUTTON_DOWN, RI_MOUSE_LEFT_BUTTON_UP, 1),
    (RI_MOUSE_RIGHT_BUTTON_DOWN, RI_MOUSE_RIGHT_BUTTON_UP, 2),
    (RI_MOUSE_MIDDLE_BUTTON_DOWN, RI_MOUSE_MIDDLE_BUTTON_UP, 3),
    (RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP, 4),
    (RI_MOUSE_BUTTON_5_DOWN, RI_MOUSE_BUTTON_5_UP, 5),
];

pub struct RawInputThread {
    hwnd_ref: Arc<Mutex<Option<isize>>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl RawInputThread {
    pub fn start(tx: UnboundedSender<InputEvent>, min_delta: i32, flush_hz: u32) -> Self {
        let hwnd_ref = Arc::new(Mutex::new(None::<isize>));
        let hwnd_clone = Arc::clone(&hwnd_ref);
        let handle = thread::Builder::new()
            .name("RawInputBuffer".into())
            .spawn(move || run_raw_input(tx, min_delta, flush_hz, hwnd_clone))
            .expect("failed to spawn raw input thread");
        RawInputThread {
            hwnd_ref,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        if let Some(hwnd) = *self.hwnd_ref.lock().unwrap() {
            unsafe {
                let _ = PostMessageW(HWND(hwnd as *mut _), WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    #[allow(dead_code)]
    pub fn is_alive(&self) -> bool {
        self.handle.as_ref().is_some_and(|h| !h.is_finished())
    }
}

impl Drop for RawInputThread {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_raw_input(
    tx: UnboundedSender<InputEvent>,
    min_delta: i32,
    flush_hz: u32,
    hwnd_out: Arc<Mutex<Option<isize>>>,
) {
    unsafe {
        set_background_priority();

        let hwnd = match create_message_window() {
            Some(h) => h,
            None => {
                tracing::error!("raw_input: failed to create message window");
                return;
            }
        };
        *hwnd_out.lock().unwrap() = Some(hwnd.0 as isize);

        if !register_devices(hwnd) {
            tracing::error!("raw_input: RegisterRawInputDevices failed");
            let _ = DestroyWindow(hwnd);
            return;
        }

        //fuck michaelsoft fuck bimbows and fuck tracy bennett for ruining my wordle
        let timer_boosted = needs_timer_boost();
        if timer_boosted {
            timeBeginPeriod(1);
        }
        tracing::info!("raw_input: started (min_delta={min_delta}, flush_hz={flush_hz}, timer_boost={timer_boosted})");

        let us_layout = LoadKeyboardLayoutW(
            windows::core::w!("00000409"),
            windows::Win32::UI::Input::KeyboardAndMouse::ACTIVATE_KEYBOARD_LAYOUT_FLAGS(0),
        )
        .unwrap_or(windows::Win32::UI::Input::KeyboardAndMouse::HKL(
            std::ptr::null_mut(),
        ));

        let accum_dx: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
        let accum_dy: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
        let accum_ts: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));

        {
            let flush_tx = tx.clone();
            let adx = Arc::clone(&accum_dx);
            let ady = Arc::clone(&accum_dy);
            let ats = Arc::clone(&accum_ts);
            let _ = thread::Builder::new()
                .name("RawInputFlush".into())
                .spawn(move || {
                    let iv = Duration::from_micros(1_000_000 / flush_hz as u64);
                    loop {
                        thread::sleep(iv);
                        let dx = {
                            let mut g = adx.lock().unwrap();
                            std::mem::take(&mut *g)
                        };
                        let dy = {
                            let mut g = ady.lock().unwrap();
                            std::mem::take(&mut *g)
                        };
                        if dx != 0 || dy != 0 {
                            let timestamp = {
                                let mut g = ats.lock().unwrap();
                                std::mem::take(&mut *g).unwrap_or_else(now_ms)
                            };
                            let _ = flush_tx.send(InputEvent::MouseMove { dx, dy, timestamp });
                        } else {
                            *ats.lock().unwrap() = None;
                        }
                    }
                });
        }

        let interval = Duration::from_micros(1_000_000 / flush_hz as u64);
        let mut msg = MSG::default();

        loop {
            thread::sleep(interval);
            drain_buffer(&tx, us_layout, min_delta, &accum_dx, &accum_dy, &accum_ts);

            while PeekMessageW(&mut msg, HWND::default(), 0, WM_INPUT - 1, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    unregister_devices();
                    let _ = DestroyWindow(hwnd);
                    if timer_boosted {
                        timeEndPeriod(1);
                    }
                    tracing::info!("raw_input: stopped");
                    return;
                }
            }
        }
    }
}

unsafe fn set_background_priority() {
    let h = GetCurrentThread();
    let _ = SetThreadPriority(h, THREAD_PRIORITY_BELOW_NORMAL);
    let _ = SetThreadPriorityBoost(h, BOOL(1));
}

unsafe fn create_message_window() -> Option<HWND> {
    let hinstance = GetModuleHandleW(PCWSTR::null()).ok()?;
    CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0),
        windows::core::w!("STATIC"),
        PCWSTR::null(),
        WINDOW_STYLE(0),
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        None,
        hinstance,
        None,
    )
    .ok()
}

unsafe fn register_devices(hwnd: HWND) -> bool {
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x06,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        },
        RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x02,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        },
    ];
    RegisterRawInputDevices(&devices, std::mem::size_of::<RAWINPUTDEVICE>() as u32).is_ok()
}

unsafe fn unregister_devices() {
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x06,
            dwFlags: RIDEV_REMOVE,
            hwndTarget: HWND::default(),
        },
        RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x02,
            dwFlags: RIDEV_REMOVE,
            hwndTarget: HWND::default(),
        },
    ];
    let _ = RegisterRawInputDevices(&devices, std::mem::size_of::<RAWINPUTDEVICE>() as u32);
}

unsafe fn drain_buffer(
    tx: &UnboundedSender<InputEvent>,
    us_layout: windows::Win32::UI::Input::KeyboardAndMouse::HKL,
    min_delta: i32,
    accum_dx: &Mutex<i32>,
    accum_dy: &Mutex<i32>,
    accum_ts: &Mutex<Option<u64>>,
) {
    loop {
        let mut sz: u32 = 0;
        if GetRawInputBuffer(None, &mut sz, std::mem::size_of::<RAWINPUTHEADER>() as u32) != 0 {
            break;
        }
        if sz == 0 {
            break;
        }
        let alloc = (sz * 8).max(std::mem::size_of::<RAWINPUT>() as u32) as usize;
        let mut buf = vec![0u8; alloc];
        let mut read_sz = alloc as u32;

        let count = GetRawInputBuffer(
            Some(buf.as_mut_ptr() as *mut RAWINPUT),
            &mut read_sz,
            std::mem::size_of::<RAWINPUTHEADER>() as u32,
        );
        if count == 0 || count == u32::MAX {
            break;
        }

        let ri_sz = std::mem::size_of::<RAWINPUT>();
        let mut offset = 0usize;
        for _ in 0..count {
            if offset + ri_sz > buf.len() {
                break;
            }
            let ri = &*(buf.as_ptr().add(offset) as *const RAWINPUT);
            handle_rawinput(ri, tx, us_layout, min_delta, accum_dx, accum_dy, accum_ts);
            let item_sz = ri.header.dwSize as usize;
            offset = (offset + item_sz + 7) & !7;
        }
    }
}

unsafe fn handle_rawinput(
    ri: &RAWINPUT,
    tx: &UnboundedSender<InputEvent>,
    us_layout: windows::Win32::UI::Input::KeyboardAndMouse::HKL,
    min_delta: i32,
    accum_dx: &Mutex<i32>,
    accum_dy: &Mutex<i32>,
    accum_ts: &Mutex<Option<u64>>,
) {
    if ri.header.dwType == RIM_TYPEKEYBOARD.0 {
        //hDevice is null for syntehtic events (eg when pressing altgr) so ignore then to avoid another input event
        if ri.header.hDevice.0.is_null() {
            return;
        }
        handle_keyboard(&ri.data.keyboard, tx, us_layout);
    } else if ri.header.dwType == RIM_TYPEMOUSE.0 {
        handle_mouse(&ri.data.mouse, tx, min_delta, accum_dx, accum_dy, accum_ts);
    }
}

unsafe fn handle_keyboard(
    kb: &windows::Win32::UI::Input::RAWKEYBOARD,
    tx: &UnboundedSender<InputEvent>,
    us_layout: windows::Win32::UI::Input::KeyboardAndMouse::HKL,
) {
    if kb.VKey == 0xFF {
        return;
    }
    let is_release = (kb.Flags & RI_KEY_BREAK) != 0;
    let rawcode: u16 = if (kb.Flags & RI_KEY_E1) != 0 {
        kb.VKey
    } else {
        let ext_scan = kb.MakeCode
            | if (kb.Flags & RI_KEY_E0) != 0 {
                0x100
            } else {
                0
            };
        let vk = MapVirtualKeyExW(ext_scan as u32, MAPVK_VSC_TO_VK_EX, us_layout);
        if vk != 0 {
            vk as u16
        } else {
            //MapVirtualKeyExW fail..
            //fallback to manual convertin
            match ext_scan {
                0x11D => 163, // E0 + 0x1D = key_rightctrl
                0x138 => 165, // E0 + 0x38 = ley_rightalt
                _ => kb.VKey,
            }
        }
    };
    if rawcode == 0 {
        return;
    }
    let timestamp = now_ms();
    let _ = tx.send(if is_release {
        InputEvent::KeyRelease { rawcode, timestamp }
    } else {
        InputEvent::KeyPress { rawcode, timestamp }
    });
}

unsafe fn handle_mouse(
    m: &windows::Win32::UI::Input::RAWMOUSE,
    tx: &UnboundedSender<InputEvent>,
    min_delta: i32,
    accum_dx: &Mutex<i32>,
    accum_dy: &Mutex<i32>,
    accum_ts: &Mutex<Option<u64>>,
) {
    let timestamp = now_ms();
    let flags = m.Anonymous.Anonymous.usButtonFlags;
    if flags != 0 {
        for &(dn, up, btn) in BTN_MAP {
            if flags & dn != 0 {
                let _ = tx.send(InputEvent::MouseButton {
                    button: btn,
                    pressed: true,
                    timestamp,
                });
            }
            if flags & up != 0 {
                let _ = tx.send(InputEvent::MouseButton {
                    button: btn,
                    pressed: false,
                    timestamp,
                });
            }
        }
        if flags & RI_MOUSE_WHEEL != 0 {
            let raw = m.Anonymous.Anonymous.usButtonData as i16;
            let rot: i8 = if raw > 0 {
                -1
            } else if raw < 0 {
                1
            } else {
                0
            };
            if rot != 0 {
                let _ = tx.send(InputEvent::MouseScroll {
                    rotation: rot,
                    timestamp,
                });
            }
        }
    }
    if m.usFlags.0 & 0x0001 != 0 {
        return;
    }
    let dx = m.lLastX;
    let dy = m.lLastY;
    if dx == 0 && dy == 0 {
        return;
    }
    if min_delta > 0 && dx.abs() + dy.abs() < min_delta {
        return;
    }
    *accum_dx.lock().unwrap() += dx;
    *accum_dy.lock().unwrap() += dy;
    let mut ts = accum_ts.lock().unwrap();
    if ts.is_none() {
        *ts = Some(timestamp);
    }
}
