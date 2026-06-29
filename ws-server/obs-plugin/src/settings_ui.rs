use std::ffi::{c_void, CString};
use std::path::PathBuf;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, Mutex};

use crate::obs::{
    ObsApi, ObsData, ObsProperties, ObsProperty, ObsPropertyClickedFn, ObsSource, ObsSourceInfo,
    OBS_COMBO_FORMAT_STRING, OBS_COMBO_TYPE_LIST, OBS_GROUP_NORMAL, OBS_SOURCE_CAP_DISABLED,
    OBS_SOURCE_TYPE_INPUT, OBS_TEXT_DEFAULT, OBS_TEXT_INFO, OBS_TEXT_PASSWORD,
};
use io_ws_common::services::config::Config;
use io_ws_common::ws_server::WsState;

struct SourceCallbackState {
    config: Arc<Mutex<Config>>,
    config_path: PathBuf,
    ws_state: Arc<WsState>,
}

static SOURCE_STATE: Mutex<Option<SourceCallbackState>> = Mutex::new(None);
static SINGLETON_SOURCE: AtomicPtr<ObsSource> = AtomicPtr::new(std::ptr::null_mut());

const SOURCE_ID: &[u8] = b"input-overlay-ws-server-settings\0";

pub fn init(config: Arc<Mutex<Config>>, config_path: PathBuf, ws_state: Arc<WsState>) -> bool {
    let Some(api) = crate::obs::api() else {
        tracing::error!("settings_ui: OBS API not loaded");
        return false;
    };

    *SOURCE_STATE.lock().unwrap() = Some(SourceCallbackState {
        config: config.clone(),
        config_path: config_path.clone(),
        ws_state: ws_state.clone(),
    });

    let info = ObsSourceInfo {
        id: SOURCE_ID.as_ptr() as *const _,
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
    };
    unsafe {
        (api.register_source_s)(&info, std::mem::size_of::<ObsSourceInfo>());
    }

    let data = unsafe { (api.data_create)() };
    unsafe {
        let cfg = config.lock().unwrap();
        populate_obs_data(api, data, &cfg);
    }
    let source = unsafe {
        (api.source_create_private)(
            SOURCE_ID.as_ptr() as *const _,
            b"Input Overlay WS Server\0".as_ptr() as *const _,
            data,
        )
    };
    unsafe { (api.data_release)(data) };

    if source.is_null() {
        tracing::error!("settings_ui: failed to create singleton source");
        return false;
    }
    SINGLETON_SOURCE.store(source, Ordering::Relaxed);

    unsafe {
        (api.add_tools_menu_item)(
            b"Input Overlay WS Settings\0".as_ptr() as *const _,
            on_tools_menu_click,
            std::ptr::null_mut(),
        );
    }

    tracing::info!("settings UI registered");
    true
}

pub fn release_singleton() {
    let source = SINGLETON_SOURCE.swap(std::ptr::null_mut(), Ordering::Relaxed);
    if !source.is_null() {
        if let Some(api) = crate::obs::api() {
            unsafe {
                (api.source_release)(source);
            }
        }
    }
    drop(SOURCE_STATE.lock().unwrap().take());
}

unsafe extern "C" fn source_get_name(_type_data: *mut c_void) -> *const std::ffi::c_char {
    b"Input Overlay WS Server\0".as_ptr() as *const _
}

unsafe extern "C" fn source_create(
    _settings: *mut ObsData,
    _source: *mut ObsSource,
) -> *mut c_void {
    Box::into_raw(Box::new(())) as *mut c_void
}

unsafe extern "C" fn source_destroy(data: *mut c_void) {
    if !data.is_null() {
        drop(Box::from_raw(data as *mut ()));
    }
}

unsafe extern "C" fn source_get_defaults(settings: *mut ObsData) {
    let Some(api) = crate::obs::api() else {
        return;
    };
    (api.data_set_default_string)(
        settings,
        b"host\0".as_ptr() as _,
        b"localhost\0".as_ptr() as _,
    );
    (api.data_set_default_int)(settings, b"port\0".as_ptr() as _, 4455);
    (api.data_set_default_string)(settings, b"auth_token\0".as_ptr() as _, b"\0".as_ptr() as _);
    (api.data_set_default_bool)(settings, b"send_mouse_move\0".as_ptr() as _, true);
    (api.data_set_default_string)(
        settings,
        b"key_whitelist\0".as_ptr() as _,
        b"\0".as_ptr() as _,
    );
    (api.data_set_default_string)(
        settings,
        b"analog_keyboard\0".as_ptr() as _,
        b"\0".as_ptr() as _,
    );
    #[cfg(target_os = "linux")]
    {
        (api.data_set_default_string)(
            settings,
            b"linux_kbd_device\0".as_ptr() as _,
            b"\0".as_ptr() as _,
        );
        (api.data_set_default_string)(
            settings,
            b"linux_mouse_device\0".as_ptr() as _,
            b"\0".as_ptr() as _,
        );
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
            b"\xE2\x9A\xA0\xEF\xB8\x8F Not running as admin, WM_INPUT won't reach \
this process while an elevated app is in focus. Blease restart OBS as administrator.\0",
        );
    }

    group(api, p, b"grp_server\0", b"WS Server\0", |g| {
        text(api, g, b"host\0", b"Host\0", OBS_TEXT_DEFAULT);
        int(api, g, b"port\0", b"Port\0", 1, 65535);
    });

    group(api, p, b"grp_auth\0", b"Authentication\0", |g| {
        text(api, g, b"auth_token\0", b"Auth Token\0", OBS_TEXT_PASSWORD);
        btn(api, g, b"copy_token\0", b"Copy Token\0", copy_token_clicked);
        btn(
            api,
            g,
            b"regen_token\0",
            b"Regenerate Token\0",
            regen_token_clicked,
        );
    });

    group(api, p, b"grp_input\0", b"Input\0", |g| {
        text(
            api,
            g,
            b"key_whitelist\0",
            b"Key Whitelist\0",
            OBS_TEXT_DEFAULT,
        );
        info(
            api,
            g,
            b"wl_note\0",
            b"Comma-separated key names. Leave empty to allow all keys.\0",
        );
        bool(api, g, b"send_mouse_move\0", b"Send mouse movement\0");

        let ak = list(api, g, b"analog_keyboard\0", b"Analog Keyboard\0");
        list_item(api, ak, b"Disabled\0", b"\0");
        list_item(api, ak, b"Auto detect\0", b"auto\0");
        list_item(api, ak, b"Wooting\0", b"wooting\0");
        list_item(api, ak, b"Razer Huntsman Analog\0", b"razer\0");
        list_item(api, ak, b"DrunkDeer\0", b"drunkdeer\0");
        list_item(api, ak, b"Keychron / Lemokey HE\0", b"keychron\0");
        list_item(api, ak, b"NuPhy\0", b"nuphy\0");
        list_item(api, ak, b"Madlions\0", b"madlions\0");
        list_item(api, ak, b"Redragon\0", b"bytech\0");
        info(
            api,
            g,
            b"ak_note\0",
            b"Analog keyboard change requires restarting OBS.\0",
        );

        #[cfg(target_os = "linux")]
        {
            text(
                api,
                g,
                b"linux_kbd_device\0",
                b"Keyboard Device\0",
                OBS_TEXT_DEFAULT,
            );
            text(
                api,
                g,
                b"linux_mouse_device\0",
                b"Mouse Device\0",
                OBS_TEXT_DEFAULT,
            );
            info(
                api,
                g,
                b"linux_note\0",
                b"e.g. /dev/input/event0 changes require restarting OBS.\0",
            );
        }
    });

    group(api, p, b"grp_about\0", b"About\0", |g| {
        info(
            api,
            g,
            b"about_ver\0",
            concat!("input-overlay-ws-server v", env!("CARGO_PKG_VERSION"), "\0").as_bytes(),
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
            b"<a href=\"https://github.com/girlglock/input-overlay\">GitHub</a> | \
<a href=\"https://twitter.com/girlglock_\">Twitter</a> | \
<a href=\"https://girlglock.com\">girlglock.com</a>\0",
        );
    });

    p
}

unsafe extern "C" fn source_update(_data: *mut c_void, settings: *mut ObsData) {
    let Some(api) = crate::obs::api() else {
        return;
    };
    let guard = match SOURCE_STATE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let Some(state) = guard.as_ref() else {
        return;
    };

    let host = read_str(api, settings, b"host\0");
    let port = (api.data_get_int)(settings, b"port\0".as_ptr() as _) as u16;
    let auth_token = read_str(api, settings, b"auth_token\0");
    let send_mouse_move = (api.data_get_bool)(settings, b"send_mouse_move\0".as_ptr() as _);
    let whitelist_str = read_str(api, settings, b"key_whitelist\0");
    let analog_keyboard = read_str(api, settings, b"analog_keyboard\0");
    #[cfg(target_os = "linux")]
    let linux_kbd = read_str(api, settings, b"linux_kbd_device\0");
    #[cfg(target_os = "linux")]
    let linux_mouse = read_str(api, settings, b"linux_mouse_device\0");

    let key_whitelist: Vec<String> = whitelist_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let (host_changed, port_changed) = {
        let cfg = state.config.lock().unwrap();
        (cfg.host != host, cfg.port != port)
    };

    {
        let mut cfg = state.config.lock().unwrap();
        cfg.host = host;
        cfg.port = port;
        cfg.auth_token = auth_token;
        cfg.send_mouse_move = send_mouse_move;
        cfg.key_whitelist = key_whitelist;
        cfg.analog_keyboard = analog_keyboard;
        #[cfg(target_os = "linux")]
        {
            cfg.linux_evdev_keyboard_device = linux_kbd;
            cfg.linux_raw_mouse_device = linux_mouse;
        }
    }

    {
        let cfg = state.config.lock().unwrap();
        if let Err(e) = io_ws_common::services::config::save(&state.config_path, &cfg) {
            tracing::error!("failed to save config: {e}");
        }
    }

    if host_changed || port_changed {
        let _ = state.ws_state.rebind_tx.send(());
    }
}

unsafe fn text(api: &ObsApi, p: *mut ObsProperties, id: &[u8], label: &[u8], kind: i32) {
    (api.properties_add_text)(p, id.as_ptr() as _, label.as_ptr() as _, kind);
}

unsafe fn int(api: &ObsApi, p: *mut ObsProperties, id: &[u8], label: &[u8], min: i32, max: i32) {
    (api.properties_add_int)(p, id.as_ptr() as _, label.as_ptr() as _, min, max, 1);
}

unsafe fn bool(api: &ObsApi, p: *mut ObsProperties, id: &[u8], label: &[u8]) {
    (api.properties_add_bool)(p, id.as_ptr() as _, label.as_ptr() as _);
}

unsafe fn info(api: &ObsApi, p: *mut ObsProperties, id: &[u8], msg: &[u8]) {
    (api.properties_add_text)(p, id.as_ptr() as _, msg.as_ptr() as _, OBS_TEXT_INFO);
}

unsafe fn btn(
    api: &ObsApi,
    p: *mut ObsProperties,
    id: &[u8],
    label: &[u8],
    cb: ObsPropertyClickedFn,
) {
    (api.properties_add_button)(p, id.as_ptr() as _, label.as_ptr() as _, cb);
}

unsafe fn list(api: &ObsApi, p: *mut ObsProperties, id: &[u8], label: &[u8]) -> *mut ObsProperty {
    (api.properties_add_list)(
        p,
        id.as_ptr() as _,
        label.as_ptr() as _,
        OBS_COMBO_TYPE_LIST,
        OBS_COMBO_FORMAT_STRING,
    )
}

unsafe fn list_item(api: &ObsApi, prop: *mut ObsProperty, label: &[u8], val: &[u8]) {
    (api.property_list_add_string)(prop, label.as_ptr() as _, val.as_ptr() as _);
}

unsafe fn group(
    api: &ObsApi,
    parent: *mut ObsProperties,
    id: &[u8],
    label: &[u8],
    f: impl FnOnce(*mut ObsProperties),
) {
    let g = (api.properties_create)();
    f(g);
    (api.properties_add_group)(
        parent,
        id.as_ptr() as _,
        label.as_ptr() as _,
        OBS_GROUP_NORMAL,
        g,
    );
}

unsafe fn read_str(api: &ObsApi, data: *mut ObsData, name: &[u8]) -> String {
    let ptr = (api.data_get_string)(data, name.as_ptr() as *const _);
    if ptr.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(ptr)
        .to_str()
        .unwrap_or("")
        .to_string()
}

unsafe fn populate_obs_data(api: &ObsApi, data: *mut ObsData, cfg: &Config) {
    let host = CString::new(cfg.host.as_str()).unwrap_or_default();
    let token = CString::new(cfg.auth_token.as_str()).unwrap_or_default();
    let wl = CString::new(cfg.key_whitelist.join(", ").as_str()).unwrap_or_default();
    let ak = CString::new(cfg.analog_keyboard.as_str()).unwrap_or_default();

    (api.data_set_string)(data, b"host\0".as_ptr() as _, host.as_ptr());
    (api.data_set_int)(data, b"port\0".as_ptr() as _, cfg.port as i64);
    (api.data_set_string)(data, b"auth_token\0".as_ptr() as _, token.as_ptr());
    (api.data_set_bool)(
        data,
        b"send_mouse_move\0".as_ptr() as _,
        cfg.send_mouse_move,
    );
    (api.data_set_string)(data, b"key_whitelist\0".as_ptr() as _, wl.as_ptr());
    (api.data_set_string)(data, b"analog_keyboard\0".as_ptr() as _, ak.as_ptr());

    #[cfg(target_os = "linux")]
    {
        let kbd = CString::new(cfg.linux_evdev_keyboard_device.as_str()).unwrap_or_default();
        let mouse = CString::new(cfg.linux_raw_mouse_device.as_str()).unwrap_or_default();
        (api.data_set_string)(data, b"linux_kbd_device\0".as_ptr() as _, kbd.as_ptr());
        (api.data_set_string)(data, b"linux_mouse_device\0".as_ptr() as _, mouse.as_ptr());
    }
}

unsafe extern "C" fn on_tools_menu_click(_data: *mut c_void) {
    let source = SINGLETON_SOURCE.load(Ordering::Relaxed);
    if source.is_null() {
        return;
    }
    let Some(api) = crate::obs::api() else {
        return;
    };
    (api.open_source_properties)(source);
}

unsafe extern "C" fn copy_token_clicked(
    _props: *mut ObsProperties,
    _prop: *mut ObsProperty,
    _data: *mut c_void,
) -> bool {
    let token = SOURCE_STATE
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.config.lock().unwrap().auth_token.clone())
        .unwrap_or_default();
    copy_to_clipboard(&token);
    false
}

fn copy_to_clipboard(text: &str) {
    #[cfg(windows)]
    {
        use std::io::Write;
        use std::os::windows::process::CommandExt;
        if let Ok(mut child) = std::process::Command::new("clip")
            .stdin(std::process::Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let try_pipe = |cmd: &str, args: &[&str]| -> bool {
            if let Ok(mut child) = Command::new(cmd).args(args).stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                true
            } else {
                false
            }
        };
        if !try_pipe("xclip", &["-selection", "clipboard"]) {
            if !try_pipe("xsel", &["--clipboard", "--input"]) {
                let _ = Command::new("wl-copy").arg(text).spawn();
            }
        }
    }
}

unsafe extern "C" fn regen_token_clicked(
    _props: *mut ObsProperties,
    _prop: *mut ObsProperty,
    _data: *mut c_void,
) -> bool {
    let Some(api) = crate::obs::api() else {
        return false;
    };
    let source = SINGLETON_SOURCE.load(Ordering::Relaxed);
    if source.is_null() {
        return false;
    }

    let new_token = crate::gen_token(32);
    let data = (api.data_create)();
    let token_cstr = CString::new(new_token.as_str()).unwrap_or_default();
    (api.data_set_string)(data, b"auth_token\0".as_ptr() as _, token_cstr.as_ptr());
    (api.source_update)(source, data);
    (api.data_release)(data);
    true
}


#[cfg(windows)]
fn is_admin() -> bool {
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
