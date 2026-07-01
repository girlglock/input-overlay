use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use evdev::{Device, InputEventKind, Key, RelativeAxisType};
use tokio::sync::mpsc::UnboundedSender;

use crate::services::consts::{hid_to_vk, now_ms};
use crate::ws_server::InputEvent;

fn evdev_btn_to_overlay(code: u16) -> Option<u8> {
    match code {
        0x110 => Some(1), //BTN_LEFT
        0x111 => Some(2), //BTN_RIGHT
        0x112 => Some(3), //BTN_MIDDLE
        0x113 => Some(4), //BTN_SIDE
        0x114 => Some(5), //BTN_EXTRA
        _ => None,
    }
}

fn evdev_to_hid(code: u16) -> Option<u16> {
    Some(match code {
        1 => 0x29,   //KEY_ESC
        2 => 0x1E,   //KEY_1
        3 => 0x1F,   //KEY_2
        4 => 0x20,   //KEY_3
        5 => 0x21,   //KEY_4
        6 => 0x22,   //KEY_5
        7 => 0x23,   //KEY_6
        8 => 0x24,   //KEY_7
        9 => 0x25,   //KEY_8
        10 => 0x26,  //KEY_9
        11 => 0x27,  //KEY_0
        12 => 0x2D,  //KEY_MINUS
        13 => 0x2E,  //KEY_EQUAL
        14 => 0x2A,  //KEY_BACKSPACE
        15 => 0x2B,  //KEY_TAB
        16 => 0x14,  //KEY_Q
        17 => 0x1A,  //KEY_W
        18 => 0x08,  //KEY_E
        19 => 0x15,  //KEY_R
        20 => 0x17,  //KEY_T
        21 => 0x1C,  //KEY_Y
        22 => 0x18,  //KEY_U
        23 => 0x0C,  //KEY_I
        24 => 0x12,  //KEY_O
        25 => 0x13,  //KEY_P
        26 => 0x2F,  //KEY_LEFTBRACE
        27 => 0x30,  //KEY_RIGHTBRACE
        28 => 0x28,  //KEY_ENTER
        29 => 0xE0,  //KEY_LEFTCTRL
        30 => 0x04,  //KEY_A
        31 => 0x16,  //KEY_S
        32 => 0x07,  //KEY_D
        33 => 0x09,  //KEY_F
        34 => 0x0A,  //KEY_G
        35 => 0x0B,  //KEY_H
        36 => 0x0D,  //KEY_J
        37 => 0x0E,  //KEY_K
        38 => 0x0F,  //KEY_L
        39 => 0x33,  //KEY_SEMICOLON
        40 => 0x34,  //KEY_APOSTROPHE
        41 => 0x35,  //KEY_GRAVE
        42 => 0xE1,  //KEY_LEFTSHIFT
        43 => 0x31,  //KEY_BACKSLASH
        44 => 0x1D,  //KEY_Z
        45 => 0x1B,  //KEY_X
        46 => 0x06,  //KEY_C
        47 => 0x19,  //KEY_V
        48 => 0x05,  //KEY_B
        49 => 0x11,  //KEY_N
        50 => 0x10,  //KEY_M
        51 => 0x36,  //KEY_COMMA
        52 => 0x37,  //KEY_DOT
        53 => 0x38,  //KEY_SLASH
        54 => 0xE5,  //KEY_RIGHTSHIFT
        55 => 0x55,  //KEY_KPASTERISK
        56 => 0xE2,  //KEY_LEFTALT
        57 => 0x2C,  //KEY_SPACE
        58 => 0x39,  //KEY_CAPSLOCK
        59 => 0x3A,  //KEY_F1
        60 => 0x3B,  //KEY_F2
        61 => 0x3C,  //KEY_F3
        62 => 0x3D,  //KEY_F4
        63 => 0x3E,  //KEY_F5
        64 => 0x3F,  //KEY_F6
        65 => 0x40,  //KEY_F7
        66 => 0x41,  //KEY_F8
        67 => 0x42,  //KEY_F9
        68 => 0x43,  //KEY_F10
        69 => 0x53,  //KEY_NUMLOCK
        70 => 0x47,  //KEY_SCROLLLOCK
        71 => 0x5F,  //KEY_KP7
        72 => 0x60,  //KEY_KP8
        73 => 0x61,  //KEY_KP9
        74 => 0x56,  //KEY_KPMINUS
        75 => 0x5C,  //KEY_KP4
        76 => 0x5D,  //KEY_KP5
        77 => 0x5E,  //KEY_KP6
        78 => 0x57,  //KEY_KPPLUS
        79 => 0x59,  //KEY_KP1
        80 => 0x5A,  //KEY_KP2
        81 => 0x5B,  //KEY_KP3
        82 => 0x62,  //KEY_KP0
        83 => 0x63,  //KEY_KPDOT
        85 => 0x64,  //KEY_ZENKAKUHANKAKU / KEY_102ND (ISO backslash)
        86 => 0x64,  //KEY_102ND
        87 => 0x44,  //KEY_F11
        88 => 0x45,  //KEY_F12
        183 => 0x68, //KEY_F13
        184 => 0x69, //KEY_F14
        185 => 0x6A, //KEY_F15
        186 => 0x6B, //KEY_F16
        187 => 0x6C, //KEY_F17
        188 => 0x6D, //KEY_F18
        189 => 0x6E, //KEY_F19
        190 => 0x6F, //KEY_F20
        191 => 0x70, //KEY_F21
        192 => 0x71, //KEY_F22
        193 => 0x72, //KEY_F23
        194 => 0x73, //KEY_F24
        96 => 0x58,  //KEY_KPENTER
        97 => 0xE4,  //KEY_RIGHTCTRL
        98 => 0x54,  //KEY_KPSLASH
        99 => 0x46,  //KEY_SYSRQ (print screen)
        100 => 0xE6, //KEY_RIGHTALT
        102 => 0x4A, //KEY_HOME
        103 => 0x52, //KEY_UP
        104 => 0x4B, //KEY_PAGEUP
        105 => 0x50, //KEY_LEFT
        106 => 0x4F, //KEY_RIGHT
        107 => 0x4D, //KEY_END
        108 => 0x51, //KEY_DOWN
        109 => 0x4E, //KEY_PAGEDOWN
        110 => 0x49, //KEY_INSERT
        111 => 0x4C, //KEY_DELETE
        125 => 0xE3, //KEY_LEFTMETA
        126 => 0xE7, //KEY_RIGHTMETA
        127 => 0x65, //KEY_COMPOSE (menu)
        _ => return None,
    })
}

pub fn enum_keyboards() -> Vec<(String, String)> {
    let mut results = Vec::new();
    for (path, dev) in evdev::enumerate() {
        if dev
            .supported_keys()
            .map_or(false, |k| k.contains(Key::KEY_A))
        {
            results.push((
                path.to_string_lossy().into_owned(),
                dev.name().unwrap_or("Unknown").to_string(),
            ));
        }
    }
    results
}

pub fn enum_mice() -> Vec<(String, String)> {
    let mut results = Vec::new();
    for (path, dev) in evdev::enumerate() {
        let has_xy = dev.supported_relative_axes().map_or(false, |a| {
            a.contains(RelativeAxisType::REL_X) && a.contains(RelativeAxisType::REL_Y)
        });
        if has_xy {
            results.push((
                path.to_string_lossy().into_owned(),
                dev.name().unwrap_or("Unknown").to_string(),
            ));
        }
    }
    results
}

pub fn check_permissions() -> Vec<String> {
    let mut missing = Vec::new();
    if !is_in_input_group() {
        missing.push("you are not in the 'input' group".to_string());
        missing.push("keyboard and mouse capture won't work".to_string());
        missing.push("run: sudo usermod -aG input $USER (then reboot)".to_string());
        missing.push("see gh release page for more info".to_string());
    }
    missing
}

fn is_in_input_group() -> bool {
    std::process::Command::new("id")
        .arg("-Gn")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .any(|g| g == "input")
        })
        .unwrap_or(true)
}

pub struct EvdevInputThread {
    stop: Arc<AtomicBool>,
    handles: Vec<thread::JoinHandle<()>>,
}

impl EvdevInputThread {
    pub fn start(
        tx: UnboundedSender<InputEvent>,
        kbd_path: &str,
        mouse_path: &str,
        min_delta: i32,
        flush_hz: u32,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();

        if !kbd_path.is_empty() {
            match Device::open(kbd_path) {
                Ok(dev) => {
                    let tx2 = tx.clone();
                    let stop2 = Arc::clone(&stop);
                    let h = thread::Builder::new()
                        .name("EvdevKeyboard".into())
                        .spawn(move || run_keyboard(dev, tx2, stop2))
                        .expect("spawn evdev keyboard thread");
                    handles.push(h);
                }
                Err(e) => tracing::error!("evdev: cannot open keyboard {kbd_path}: {e}"),
            }
        }

        if !mouse_path.is_empty() {
            match Device::open(mouse_path) {
                Ok(dev) => {
                    let tx2 = tx.clone();
                    let stop2 = Arc::clone(&stop);
                    let h = thread::Builder::new()
                        .name("EvdevMouse".into())
                        .spawn(move || run_mouse(dev, tx2, stop2, min_delta, flush_hz))
                        .expect("spawn evdev mouse thread");
                    handles.push(h);
                }
                Err(e) => tracing::error!("evdev: cannot open mouse {mouse_path}: {e}"),
            }
        }

        if handles.is_empty() {
            if kbd_path.is_empty() && mouse_path.is_empty() {
                tracing::info!("evdev: no devices configured");
            } else {
                tracing::warn!("evdev: no device threads started... check paths and permissions");
            }
        }

        EvdevInputThread { stop, handles }
    }
}

impl Drop for EvdevInputThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}

fn run_keyboard(mut dev: Device, tx: UnboundedSender<InputEvent>, stop: Arc<AtomicBool>) {
    unsafe {
        let fd = dev.as_raw_fd();
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    tracing::info!(
        "evdev: keyboard thread started ({})",
        dev.name().unwrap_or("?")
    );

    while !stop.load(Ordering::Relaxed) {
        match dev.fetch_events() {
            Ok(events) => {
                for event in events {
                    match event.kind() {
                        InputEventKind::Key(key) if evdev_btn_to_overlay(key.0).is_some() => {
                            let btn = evdev_btn_to_overlay(key.0).unwrap();
                            let timestamp = now_ms();
                            match event.value() {
                                1 => {
                                    let _ = tx.send(InputEvent::MouseButton {
                                        button: btn,
                                        pressed: true,
                                        timestamp,
                                    });
                                }
                                0 => {
                                    let _ = tx.send(InputEvent::MouseButton {
                                        button: btn,
                                        pressed: false,
                                        timestamp,
                                    });
                                }
                                _ => {}
                            }
                        }
                        InputEventKind::Key(key) => {
                            let Some(hid) = evdev_to_hid(key.0) else {
                                continue;
                            };
                            let Some(vk) = hid_to_vk(hid) else { continue };
                            let timestamp = now_ms();
                            match event.value() {
                                1 | 2 => {
                                    let _ = tx.send(InputEvent::KeyPress {
                                        rawcode: vk,
                                        timestamp,
                                    });
                                }
                                0 => {
                                    let _ = tx.send(InputEvent::KeyRelease {
                                        rawcode: vk,
                                        timestamp,
                                    });
                                }
                                _ => {}
                            }
                        }
                        InputEventKind::RelAxis(axis) if axis == RelativeAxisType::REL_WHEEL => {
                            let rotation: i8 = match event.value().cmp(&0) {
                                std::cmp::Ordering::Greater => -1,
                                std::cmp::Ordering::Less => 1,
                                std::cmp::Ordering::Equal => 0,
                            };
                            if rotation != 0 {
                                let _ = tx.send(InputEvent::MouseScroll {
                                    rotation,
                                    timestamp: now_ms(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(4));
            }
            Err(e) => {
                tracing::error!("evdev: keyboard error: {e}");
                break;
            }
        }
    }

    tracing::info!("evdev: keyboard thread stopped");
}

fn run_mouse(
    mut dev: Device,
    tx: UnboundedSender<InputEvent>,
    stop: Arc<AtomicBool>,
    min_delta: i32,
    flush_hz: u32,
) {
    unsafe {
        let fd = dev.as_raw_fd();
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    tracing::info!(
        "evdev: mouse thread started ({})",
        dev.name().unwrap_or("?")
    );

    let accum_dx: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let accum_dy: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let accum_ts: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));

    {
        let stop2 = Arc::clone(&stop);
        let tx2 = tx.clone();
        let adx = Arc::clone(&accum_dx);
        let ady = Arc::clone(&accum_dy);
        let ats = Arc::clone(&accum_ts);
        thread::Builder::new()
            .name("EvdevMouseFlush".into())
            .spawn(move || {
                let interval = Duration::from_micros(1_000_000 / flush_hz as u64);
                while !stop2.load(Ordering::Relaxed) {
                    thread::sleep(interval);
                    let dx = std::mem::take(&mut *adx.lock().unwrap());
                    let dy = std::mem::take(&mut *ady.lock().unwrap());
                    if dx != 0 || dy != 0 {
                        let timestamp = {
                            let mut g = ats.lock().unwrap();
                            std::mem::take(&mut *g).unwrap_or_else(now_ms)
                        };
                        let _ = tx2.send(InputEvent::MouseMove { dx, dy, timestamp });
                    } else {
                        *ats.lock().unwrap() = None;
                    }
                }
            })
            .ok();
    }

    while !stop.load(Ordering::Relaxed) {
        match dev.fetch_events() {
            Ok(events) => {
                for event in events {
                    match event.kind() {
                        InputEventKind::RelAxis(axis) if axis == RelativeAxisType::REL_X => {
                            let dx = event.value();
                            if min_delta <= 0 || dx.abs() >= min_delta {
                                *accum_dx.lock().unwrap() += dx;
                                let mut ts = accum_ts.lock().unwrap();
                                if ts.is_none() {
                                    *ts = Some(now_ms());
                                }
                            }
                        }
                        InputEventKind::RelAxis(axis) if axis == RelativeAxisType::REL_Y => {
                            let dy = event.value();
                            if min_delta <= 0 || dy.abs() >= min_delta {
                                *accum_dy.lock().unwrap() += dy;
                                let mut ts = accum_ts.lock().unwrap();
                                if ts.is_none() {
                                    *ts = Some(now_ms());
                                }
                            }
                        }
                        InputEventKind::RelAxis(axis) if axis == RelativeAxisType::REL_WHEEL => {
                            let rotation: i8 = match event.value().cmp(&0) {
                                std::cmp::Ordering::Greater => -1,
                                std::cmp::Ordering::Less => 1,
                                std::cmp::Ordering::Equal => 0,
                            };
                            if rotation != 0 {
                                let _ = tx.send(InputEvent::MouseScroll {
                                    rotation,
                                    timestamp: now_ms(),
                                });
                            }
                        }
                        InputEventKind::Key(key) => {
                            if let Some(btn) = evdev_btn_to_overlay(key.0) {
                                let timestamp = now_ms();
                                match event.value() {
                                    1 => {
                                        let _ = tx.send(InputEvent::MouseButton {
                                            button: btn,
                                            pressed: true,
                                            timestamp,
                                        });
                                    }
                                    0 => {
                                        let _ = tx.send(InputEvent::MouseButton {
                                            button: btn,
                                            pressed: false,
                                            timestamp,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(4));
            }
            Err(e) => {
                tracing::error!("evdev: mouse error: {e}");
                break;
            }
        }
    }

    tracing::info!("evdev: mouse thread stopped");
}
