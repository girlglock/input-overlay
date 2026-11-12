#![cfg(feature = "gamepad")]

use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::sync::{Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

use sdl3_sys::everything::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::input_event::InputEvent;
use crate::services::consts::now_ms;

pub const ALL_GAMEPADS: &str = "all";
const TRIGGER_PRESS_THRESHOLD: f32 = 0.12;
const IDLE_WAIT_MS: i32 = 500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GamepadInfo {
    //with a #n suffix for dupes
    pub id: String,
    pub name: String,
}

struct Output {
    tx: UnboundedSender<InputEvent>,
    filter: String,
    flush: Duration,
}

#[derive(Default)]
struct Shared {
    devices: Vec<GamepadInfo>,
    revision: u64,
    output: Option<Output>,
}

fn shared() -> &'static Mutex<Shared> {
    static SHARED: OnceLock<Mutex<Shared>> = OnceLock::new();
    SHARED.get_or_init(|| Mutex::new(Shared::default()))
}

pub fn ensure_running() {
    static START: Once = Once::new();
    START.call_once(|| {
        let spawned = std::thread::Builder::new()
            .name("Gamepad".into())
            .spawn(service_thread);
        if let Err(e) = spawned {
            tracing::error!("gamepad: failed to spawn thread: {e}");
        }
    });
}

pub fn devices() -> (u64, Vec<GamepadInfo>) {
    let s = shared().lock().unwrap();
    (s.revision, s.devices.clone())
}

//filter is "all" or a gamepad id
pub fn set_output(tx: UnboundedSender<InputEvent>, filter: &str, flush_hz: u32) {
    if filter.is_empty() {
        clear_output();
        return;
    }
    ensure_running();
    shared().lock().unwrap().output = Some(Output {
        tx,
        filter: filter.to_string(),
        flush: Duration::from_micros(1_000_000 / flush_hz.max(1) as u64),
    });
}

pub fn clear_output() {
    shared().lock().unwrap().output = None;
}

struct Pad {
    handle: *mut SDL_Gamepad,
    id: String,
    held: Vec<u8>,
}

#[derive(Default)]
struct AxisState {
    pending: Option<f32>,
    sent: f32,
}

fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

//sdl button -> w3c
fn map_button(button: u8) -> Option<u8> {
    let b = SDL_GamepadButton(button as i32);
    Some(match b {
        SDL_GAMEPAD_BUTTON_SOUTH => 0,
        SDL_GAMEPAD_BUTTON_EAST => 1,
        SDL_GAMEPAD_BUTTON_WEST => 2,
        SDL_GAMEPAD_BUTTON_NORTH => 3,
        SDL_GAMEPAD_BUTTON_LEFT_SHOULDER => 4,
        SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER => 5,
        SDL_GAMEPAD_BUTTON_BACK => 8,
        SDL_GAMEPAD_BUTTON_START => 9,
        SDL_GAMEPAD_BUTTON_LEFT_STICK => 10,
        SDL_GAMEPAD_BUTTON_RIGHT_STICK => 11,
        SDL_GAMEPAD_BUTTON_DPAD_UP => 12,
        SDL_GAMEPAD_BUTTON_DPAD_DOWN => 13,
        SDL_GAMEPAD_BUTTON_DPAD_LEFT => 14,
        SDL_GAMEPAD_BUTTON_DPAD_RIGHT => 15,
        SDL_GAMEPAD_BUTTON_GUIDE => 16,
        _ => return None,
    })
}

enum MappedAxis {
    Stick(u8),
    Trigger(u8),
}

fn map_axis(axis: u8) -> Option<MappedAxis> {
    let a = SDL_GamepadAxis(axis as i32);
    Some(match a {
        SDL_GAMEPAD_AXIS_LEFTX => MappedAxis::Stick(0),
        SDL_GAMEPAD_AXIS_LEFTY => MappedAxis::Stick(1),
        SDL_GAMEPAD_AXIS_RIGHTX => MappedAxis::Stick(2),
        SDL_GAMEPAD_AXIS_RIGHTY => MappedAxis::Stick(3),
        SDL_GAMEPAD_AXIS_LEFT_TRIGGER => MappedAxis::Trigger(6),
        SDL_GAMEPAD_AXIS_RIGHT_TRIGGER => MappedAxis::Trigger(7),
        _ => return None,
    })
}

fn guid_string(instance: SDL_JoystickID) -> String {
    let mut buf = [0 as c_char; 33];
    unsafe {
        let guid = SDL_GetGamepadGUIDForID(instance);
        SDL_GUIDToString(guid, buf.as_mut_ptr(), buf.len() as i32);
        cstr(buf.as_ptr())
    }
}

struct Service {
    pads: HashMap<u32, Pad>,
    axes: HashMap<(u32, u8, bool), AxisState>,
    last_flush: Instant,
}

impl Service {
    fn publish_devices(&self) {
        let mut list: Vec<GamepadInfo> = self
            .pads
            .values()
            .map(|p| GamepadInfo {
                id: p.id.clone(),
                name: cstr(unsafe { SDL_GetGamepadName(p.handle) }),
            })
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        let mut s = shared().lock().unwrap();
        if s.devices != list {
            s.devices = list;
            s.revision += 1;
        }
    }

    fn send(&self, event: InputEvent, pad_id: &str) {
        let s = shared().lock().unwrap();
        if let Some(out) = &s.output {
            if out.filter == ALL_GAMEPADS || out.filter == pad_id {
                let _ = out.tx.send(event);
            }
        }
    }

    fn on_added(&mut self, instance: SDL_JoystickID) {
        if self.pads.contains_key(&instance.0) {
            return;
        }
        let handle = unsafe { SDL_OpenGamepad(instance) };
        if handle.is_null() {
            tracing::warn!(
                "gamepad: failed to open #{}: {}",
                instance.0,
                cstr(SDL_GetError())
            );
            return;
        }
        let base = guid_string(instance);
        let mut id = base.clone();
        let mut n = 2;
        while self.pads.values().any(|p| p.id == id) {
            id = format!("{base}#{n}");
            n += 1;
        }
        tracing::info!(
            "gamepad: connected \"{}\" ({id})",
            cstr(unsafe { SDL_GetGamepadName(handle) })
        );
        self.pads.insert(
            instance.0,
            Pad {
                handle,
                id,
                held: Vec::new(),
            },
        );
        self.publish_devices();
    }

    fn on_removed(&mut self, instance: SDL_JoystickID) {
        let Some(pad) = self.pads.remove(&instance.0) else {
            return;
        };
        let timestamp = now_ms();
        for button in &pad.held {
            self.send(
                InputEvent::GamepadButton {
                    pad: instance.0,
                    button: *button,
                    pressed: false,
                    value: 0.0,
                    timestamp,
                },
                &pad.id,
            );
        }
        for ((p, idx, trigger), state) in self.axes.iter() {
            if *p != instance.0 || state.sent == 0.0 {
                continue;
            }
            let event = if *trigger {
                InputEvent::GamepadButton {
                    pad: *p,
                    button: *idx,
                    pressed: false,
                    value: 0.0,
                    timestamp,
                }
            } else {
                InputEvent::GamepadAxis {
                    pad: *p,
                    axis: *idx,
                    value: 0.0,
                    timestamp,
                }
            };
            self.send(event, &pad.id);
        }
        self.axes.retain(|(p, _, _), _| *p != instance.0);
        unsafe { SDL_CloseGamepad(pad.handle) };
        tracing::info!("gamepad: disconnected ({})", pad.id);
        self.publish_devices();
    }

    fn on_button(&mut self, instance: SDL_JoystickID, button: u8, down: bool) {
        let Some(mapped) = map_button(button) else {
            return;
        };
        let Some(pad) = self.pads.get_mut(&instance.0) else {
            return;
        };
        if down {
            if !pad.held.contains(&mapped) {
                pad.held.push(mapped);
            }
        } else {
            pad.held.retain(|b| *b != mapped);
        }
        let id = pad.id.clone();
        self.send(
            InputEvent::GamepadButton {
                pad: instance.0,
                button: mapped,
                pressed: down,
                value: if down { 1.0 } else { 0.0 },
                timestamp: now_ms(),
            },
            &id,
        );
    }

    fn on_axis(&mut self, instance: SDL_JoystickID, axis: u8, raw: i16) {
        let Some(mapped) = map_axis(axis) else {
            return;
        };
        let (idx, trigger, value) = match mapped {
            MappedAxis::Stick(i) => (i, false, (raw as f32 / 32767.0).clamp(-1.0, 1.0)),
            MappedAxis::Trigger(b) => (b, true, (raw as f32 / 32767.0).clamp(0.0, 1.0)),
        };
        self.axes
            .entry((instance.0, idx, trigger))
            .or_default()
            .pending = Some(value);
    }

    fn flush_axes(&mut self) {
        let timestamp = now_ms();
        let mut out = Vec::new();
        for ((pad, idx, trigger), state) in self.axes.iter_mut() {
            let Some(value) = state.pending.take() else {
                continue;
            };
            if value == state.sent {
                continue;
            }
            state.sent = value;
            let event = if *trigger {
                InputEvent::GamepadButton {
                    pad: *pad,
                    button: *idx,
                    pressed: value > TRIGGER_PRESS_THRESHOLD,
                    value,
                    timestamp,
                }
            } else {
                InputEvent::GamepadAxis {
                    pad: *pad,
                    axis: *idx,
                    value,
                    timestamp,
                }
            };
            out.push((*pad, event));
        }
        for (pad, event) in out {
            if let Some(id) = self.pads.get(&pad).map(|p| p.id.clone()) {
                self.send(event, &id);
            }
        }
        self.last_flush = Instant::now();
    }

    fn handle(&mut self, ev: &SDL_Event) {
        unsafe {
            let kind = SDL_EventType(ev.r#type);
            if kind == SDL_EVENT_GAMEPAD_ADDED {
                self.on_added(ev.gdevice.which);
            } else if kind == SDL_EVENT_GAMEPAD_REMOVED {
                self.on_removed(ev.gdevice.which);
            } else if kind == SDL_EVENT_GAMEPAD_BUTTON_DOWN || kind == SDL_EVENT_GAMEPAD_BUTTON_UP {
                self.on_button(ev.gbutton.which, ev.gbutton.button, ev.gbutton.down);
            } else if kind == SDL_EVENT_GAMEPAD_AXIS_MOTION {
                self.on_axis(ev.gaxis.which, ev.gaxis.axis, ev.gaxis.value);
            }
        }
    }
}

fn service_thread() {
    unsafe {
        let one = c"1";
        SDL_SetHint(SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS, one.as_ptr());
        SDL_SetHint(SDL_HINT_JOYSTICK_THREAD, one.as_ptr());
        if !SDL_Init(SDL_INIT_GAMEPAD) {
            tracing::error!("gamepad: SDL_Init failed: {}", cstr(SDL_GetError()));
            return;
        }
    }
    tracing::info!("gamepad: sdl started");

    let mut service = Service {
        pads: HashMap::new(),
        axes: HashMap::new(),
        last_flush: Instant::now(),
    };
    let mut ev: SDL_Event = unsafe { std::mem::zeroed() };
    loop {
        let flush = shared().lock().unwrap().output.as_ref().map(|o| o.flush);
        let wait_ms = match flush {
            Some(f) => f
                .saturating_sub(service.last_flush.elapsed())
                .as_millis()
                .clamp(1, 100) as i32,
            None => IDLE_WAIT_MS,
        };
        unsafe {
            if SDL_WaitEventTimeout(&mut ev, wait_ms) {
                service.handle(&ev);
                while SDL_PollEvent(&mut ev) {
                    service.handle(&ev);
                }
            }
        }
        match flush {
            Some(f) if service.last_flush.elapsed() >= f => service.flush_axes(),
            None => {
                for state in service.axes.values_mut() {
                    state.pending = None;
                }
                service.last_flush = Instant::now();
            }
            _ => {}
        }
    }
}
