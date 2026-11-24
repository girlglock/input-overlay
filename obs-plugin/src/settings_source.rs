use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use io_ws_common::services::config::Config;

use crate::obs::{
    ObsData, ObsProperties, ObsSource, ObsSourceInfo, OBS_SOURCE_CAP_DISABLED, OBS_SOURCE_TYPE_INPUT,
};
use crate::obs_helpers::{group, info, list, list_header_str, list_item, list_item_str, read_str};

const SOURCE_ID: &std::ffi::CStr = c"input-overlay-gg-settings";
const WINDOW_NAME: &std::ffi::CStr = c"Input Overlay Settings (GG)";

struct PluginConfigState {
    config: Arc<Mutex<Config>>,
    config_path: PathBuf,
}

static PLUGIN_CONFIG: Mutex<Option<PluginConfigState>> = Mutex::new(None);

struct SourcePtr(*mut ObsSource);
unsafe impl Send for SourcePtr {}

//created on the first menu click and released in obs_module_unload
static SETTINGS_SOURCE: Mutex<SourcePtr> = Mutex::new(SourcePtr(std::ptr::null_mut()));

pub fn init(config: Arc<Mutex<Config>>, config_path: PathBuf) -> bool {
    let Some(api) = crate::obs::api() else {
        return false;
    };
    *PLUGIN_CONFIG.lock().unwrap() = Some(PluginConfigState { config, config_path });

    let info_struct = ObsSourceInfo {
        id: SOURCE_ID.as_ptr(),
        type_: OBS_SOURCE_TYPE_INPUT,
        output_flags: OBS_SOURCE_CAP_DISABLED,
        get_name: Some(source_get_name),
        create: Some(source_create),
        destroy: Some(source_destroy),
        get_width: None,
        get_height: None,
        get_defaults: Some(source_get_defaults),
        get_properties: Some(source_get_properties),
        update: Some(source_update),
        activate: None,
        deactivate: None,
        show: None,
        hide: None,
        video_tick: None,
        video_render: None,
    };
    
    unsafe {
        (api.register_source_s)(&info_struct, std::mem::size_of::<ObsSourceInfo>());
    }

    match crate::obs::frontend() {
        Some(fe) => unsafe {
            (fe.add_tools_menu_item)(WINDOW_NAME.as_ptr(), Some(open_settings), std::ptr::null_mut());
        },
        None => tracing::warn!("settings: obs frontend api not available, no Tools menu entry"),
    }
    
    true
}

pub fn shutdown() {
    let source = std::mem::replace(&mut SETTINGS_SOURCE.lock().unwrap().0, std::ptr::null_mut());
    
    if !source.is_null() {
        if let Some(api) = crate::obs::api() {
            unsafe { (api.source_release)(source) };
        }
    }
}

unsafe extern "C" fn open_settings(_data: *mut c_void) {
    let (Some(api), Some(fe)) = (crate::obs::api(), crate::obs::frontend()) else {
        return;
    };
    
    let source = {
        let mut slot = SETTINGS_SOURCE.lock().unwrap();
        if slot.0.is_null() {
            slot.0 = (api.source_create_private)(SOURCE_ID.as_ptr(), WINDOW_NAME.as_ptr(), std::ptr::null_mut());
        }
        slot.0
    };
    
    if source.is_null() {
        tracing::error!("settings: couldn't create the settings source");
        return;
    }
    
    (fe.open_source_properties)(source);
}

fn current_config() -> Option<Config> {
    PLUGIN_CONFIG
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.config.lock().unwrap().clone())
}

unsafe extern "C" fn source_get_name(_type_data: *mut c_void) -> *const std::ffi::c_char {
    WINDOW_NAME.as_ptr()
}

unsafe extern "C" fn source_create(settings: *mut ObsData, _source: *mut ObsSource) -> *mut c_void {
    if let (Some(api), Some(cfg)) = (crate::obs::api(), current_config()) {
        let set = |key: &std::ffi::CStr, value: &str| {
            if let Ok(value) = std::ffi::CString::new(value) {
                (api.data_set_string)(settings, key.as_ptr(), value.as_ptr());
            }
        };
        
        set(c"analog_keyboard", &cfg.analog_keyboard);
        set(c"gamepad_device", &cfg.gamepad_device);
        
        #[cfg(target_os = "linux")]
        {
            set(c"linux_kbd_device", &cfg.linux_evdev_keyboard_device);
            set(c"linux_mouse_device", &cfg.linux_raw_mouse_device);
        }
    }

    std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
}

unsafe extern "C" fn source_destroy(_data: *mut c_void) {}

unsafe extern "C" fn source_get_defaults(settings: *mut ObsData) {
    let Some(api) = crate::obs::api() else {
        return;
    };
    
    (api.data_set_default_string)(settings, c"analog_keyboard".as_ptr(), c"".as_ptr());
    (api.data_set_default_string)(settings, c"gamepad_device".as_ptr(), c"".as_ptr());
    
    #[cfg(target_os = "linux")]
    {
        (api.data_set_default_string)(settings, c"linux_kbd_device".as_ptr(), c"".as_ptr());
        (api.data_set_default_string)(settings, c"linux_mouse_device".as_ptr(), c"".as_ptr());
    }
}

unsafe extern "C" fn source_get_properties(_data: *mut c_void) -> *mut ObsProperties {
    let Some(api) = crate::obs::api() else {
        return std::ptr::null_mut();
    };
    
    let p = (api.properties_create)();

    #[cfg(windows)]
    if !is_admin() {
        info(
            api,
            p,
            b"admin_warn\0",
            b"\xE2\x9A\xA0\xEF\xB8\x8F OBS is not running as admin, WM_INPUT wont post messages to the OBS process while an app with admin rights is in focus.. blease consider restarting as admin :3c\0",
        );
    }

    group(api, p, b"grp_capture\0", b"Capture\0", |g2| {
        let ak = list(api, g2, b"analog_keyboard\0", b"Analog Keyboard\0");
    
        list_item(api, ak, b"Disabled\0", b"\0");
        list_item(api, ak, b"Auto detect\0", b"auto\0");
    
        for brand in io_ws_common::services::analog::ANALOG_BRANDS {
            list_header_str(api, ak, &format!("\u{2500}\u{2500} {} \u{2500}\u{2500}", brand.label()));
            
            if let Some(all) = brand.all_label {
                list_item_str(api, ak, &format!("    {all}"), brand.id);
            }
            
            for model in brand.models {
                list_item_str(api, ak, &format!("    {}", model.name), model.id);
            }
        }

        info(
            api,
            g2,
            b"analogsense_note\0",
            b"(R) Razer Synapse needs to be installed and running for analogue inputs to be received from this keyboard.<br>(P) The official firmware only supports polling, which can lead to lag and missed inputs.<br>(F) <a href=\"https://analogsense.org/firmware/\">Custom firmware with full analog report functionality is available.</a><br><br>Analog support mirrors the <a href=\"https://github.com/AnalogSense/JavaScript-SDK\">AnalogSense SDK</a> ported to Rust. Amazing project, please show them some love and PR new keyboards if you can!\0",
        );

        let gp = list(api, g2, b"gamepad_device\0", b"Gamepad\0");
        
        list_item(api, gp, b"Disabled\0", b"\0");
        list_item_str(api, gp, "All gamepads", io_ws_common::services::gamepad::ALL_GAMEPADS);
        
        let (_, pads) = io_ws_common::services::gamepad::devices();
        
        for pad in &pads {
            let suffix = pad.id.split_once('#').map(|(_, n)| format!(" ({n})")).unwrap_or_default();
            list_item_str(api, gp, &format!("{}{suffix}", pad.name), &pad.id);
        }
        
        let saved = current_config().map(|c| c.gamepad_device).unwrap_or_default();
        let known = saved.is_empty()
            || saved == io_ws_common::services::gamepad::ALL_GAMEPADS
            || pads.iter().any(|p| p.id == saved);
        
        if !known {
            list_item_str(api, gp, "Last used (disconnected)", &saved);
        }

        #[cfg(target_os = "linux")]
        {
            use io_ws_common::services::linux::evdev_input;

            for (id, label, devices) in [
                (&b"linux_kbd_device\0"[..], &b"Keyboard Device\0"[..], evdev_input::enum_keyboards()),
                (&b"linux_mouse_device\0"[..], &b"Mouse Device\0"[..], evdev_input::enum_mice()),
            ] {
                let prop = list(api, g2, id, label);
                list_item(api, prop, b"None\0", b"\0");
                for (path, name) in devices {
                    let node = path.rsplit('/').next().unwrap_or(&path);
                    list_item_str(api, prop, &format!("{name} ({node})"), &path);
                }
            }

            let perms = evdev_input::check_permissions();
        
            if !perms.is_empty() {
                let msg = format!("\u{26A0}\u{FE0F} {}", perms.join("<br>"));
                if let Ok(msg) = std::ffi::CString::new(msg) {
                    info(api, g2, b"linux_perm_warn\0", msg.as_bytes_with_nul());
                }
            }
        }
    });

    group(api, p, b"grp_about\0", b"About\0", |g| {
        info(
            api,
            g,
            b"about_ver\0",
            concat!("input-overlay v", env!("CARGO_PKG_VERSION"), "\0").as_bytes(),
        );
        
        info(
            api,
            g,
            b"about_configurator\0",
            b"<a href=\"https://overlay.girlglock.com\">Open Overlay Configurator</a>\0",
        );
        
        info(
            api,
            g,
            b"about_links\0",
            b"<a href=\"https://github.com/girlglock/input-overlay\">GitHub</a> | <a href=\"https://twitter.com/girlglock_\">Twitter</a> | <a href=\"https://girlglock.com\">girlglock.com</a>\0",
        );
    });

    p
}

unsafe extern "C" fn source_update(_data: *mut c_void, settings: *mut ObsData) {
    let Some(api) = crate::obs::api() else {
        return;
    };
    let analog_keyboard = read_str(api, settings, b"analog_keyboard\0");
    let gamepad_device = read_str(api, settings, b"gamepad_device\0");
    #[cfg(target_os = "linux")]
    let linux_kbd = read_str(api, settings, b"linux_kbd_device\0");
    #[cfg(target_os = "linux")]
    let linux_mouse = read_str(api, settings, b"linux_mouse_device\0");

    persist_capture_settings(
        analog_keyboard,
        gamepad_device,
        #[cfg(target_os = "linux")]
        linux_kbd,
        #[cfg(target_os = "linux")]
        linux_mouse,
    );
}

fn persist_capture_settings(
    analog_keyboard: String,
    gamepad_device: String,
    
    #[cfg(target_os = "linux")] linux_kbd: String,
    #[cfg(target_os = "linux")] linux_mouse: String,
) {
    let guard = PLUGIN_CONFIG.lock().unwrap();
    
    let Some(state) = guard.as_ref() else {
        return;
    };

    let (cfg_snapshot, analog_changed, evdev_changed) = {
        let mut cfg = state.config.lock().unwrap();
        let analog_changed = cfg.analog_keyboard != analog_keyboard;
        cfg.analog_keyboard = analog_keyboard;
        
        if cfg.gamepad_device != gamepad_device {
            if let Some(tx) = crate::INPUT_TX.get() {
                io_ws_common::services::gamepad::set_output(tx.clone(), &gamepad_device, crate::GAMEPAD_FLUSH_HZ);
            }
            cfg.gamepad_device = gamepad_device;
        }
        
        #[cfg(target_os = "linux")]
        let evdev_changed = cfg.linux_evdev_keyboard_device != linux_kbd || cfg.linux_raw_mouse_device != linux_mouse;
        #[cfg(not(target_os = "linux"))]
        let evdev_changed = false;
        #[cfg(target_os = "linux")]
        {
            cfg.linux_evdev_keyboard_device = linux_kbd;
            cfg.linux_raw_mouse_device = linux_mouse;
        }
        
        (cfg.clone(), analog_changed, evdev_changed)
    };
    
    let config_path = state.config_path.clone();
    drop(guard);

    if analog_changed {
        crate::restart_analog(&cfg_snapshot.analog_keyboard);
    }
    #[cfg(target_os = "linux")]
    if evdev_changed {
        crate::restart_evdev(
            &cfg_snapshot.linux_evdev_keyboard_device,
            &cfg_snapshot.linux_raw_mouse_device,
            cfg_snapshot.raw_mouse_min_delta,
        );
    }
    #[cfg(not(target_os = "linux"))]
    let _ = evdev_changed;

    let spawned = std::thread::Builder::new()
        .name("OverlayConfigSave".into())
        .spawn(move || {
            if let Err(e) = io_ws_common::services::config::save(&config_path, &cfg_snapshot) {
                tracing::error!("failed to save config: {e}");
            }
        })
        .is_ok();
    
    if !spawned {
        tracing::error!("settings: failed to spawn config save thread");
    }
}

#[cfg(windows)]
pub fn is_admin() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    
    unsafe {
        let mut token = HANDLE::default();
        
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        
        let mut elev = TOKEN_ELEVATION::default();
        let mut cb = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elev as *mut _ as *mut _),
            cb,
            &mut cb,
        );
        
        let _ = CloseHandle(token);
        
        ok.is_ok() && elev.TokenIsElevated != 0
    }
}