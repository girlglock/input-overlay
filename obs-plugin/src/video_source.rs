use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tiny_skia::{Color, Pixmap};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;

use crate::obs::{
    ObsData, ObsEffect, ObsProperties, ObsProperty, ObsSource, ObsSourceInfo, ObsTexture,
    GS_BLEND_INVSRCALPHA, GS_BLEND_ONE, GS_COLOR_FORMAT_RGBA, GS_TEXTURE_DYNAMIC,
    OBS_SOURCE_TYPE_INPUT, OBS_SOURCE_VIDEO, OBS_TEXT_DEFAULT,
};
use crate::obs_helpers::{
    checkbox, file_path, group, info, list, list_item, read_str, set_visible, text,
};
use crate::overlay_link::{
    decode_json_config, decode_link, default_layout, default_settings, DecodedLink,
};
use crate::render::Renderer;
use io_ws_common::services::config::Config;
use io_ws_common::input_event::InputEvent;

const SOURCE_ID: &[u8] = b"input-overlay-ws-video\0";
const DEFAULT_WIDTH: u32 = 512;
const DEFAULT_HEIGHT: u32 = 512;
const MIN_DIMENSION: i32 = 64;
const MAX_DIMENSION: i32 = 7680;

const RENDER_POLL_INTERVAL: Duration = Duration::from_millis(2);

struct RenderedFrame {
    width: u32,
    height: u32,
    pixmap: Pixmap,
}

enum RenderCommand {
    Reconfigure(Option<Box<DecodedLink>>),
    Resize(u32, u32),
}

type FrameSlot = Arc<Mutex<Option<RenderedFrame>>>;

struct PluginConfigState {
    config: Arc<Mutex<Config>>,
    config_path: PathBuf,
}

static EVENT_BUS: OnceLock<broadcast::Sender<InputEvent>> = OnceLock::new();
static PLUGIN_CONFIG: Mutex<Option<PluginConfigState>> = Mutex::new(None);

struct VideoSourceData {
    width: u32,
    height: u32,
    texture: Option<*mut ObsTexture>,
    texture_size: Option<(u32, u32)>,
    pixmap: Pixmap,
    dirty: bool,
    link: String,
    json_file: String,
    use_link: bool,
    configured: bool,
    frame_slot: FrameSlot,
    cmd_tx: Option<std::sync::mpsc::Sender<RenderCommand>>,
    render_thread: Option<std::thread::JoinHandle<()>>,
    shown: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
}

pub fn init(
    bus: broadcast::Sender<InputEvent>,
    config: Arc<Mutex<Config>>,
    config_path: PathBuf,
) -> bool {
    let Some(api) = crate::obs::api() else {
        tracing::error!("video_source: OBS API not loaded");
        return false;
    };

    let _ = EVENT_BUS.set(bus);
    *PLUGIN_CONFIG.lock().unwrap() = Some(PluginConfigState {
        config,
        config_path,
    });

    let info_struct = ObsSourceInfo {
        id: SOURCE_ID.as_ptr() as *const _,
        type_: OBS_SOURCE_TYPE_INPUT,
        output_flags: OBS_SOURCE_VIDEO,
        get_name: Some(source_get_name),
        create: Some(source_create),
        destroy: Some(source_destroy),
        get_width: Some(source_get_width),
        get_height: Some(source_get_height),
        get_defaults: Some(source_get_defaults),
        get_properties: Some(source_get_properties),
        update: Some(source_update),
        activate: Some(source_activate),
        deactivate: Some(source_deactivate),
        show: Some(source_show),
        hide: Some(source_hide),
        video_tick: Some(source_video_tick),
        video_render: Some(source_video_render),
    };
    unsafe {
        (api.register_source_s)(&info_struct, std::mem::size_of::<ObsSourceInfo>());
    }

    tracing::info!("native overlay video source registered");
    true
}

fn idle_pixmap(width: u32, height: u32) -> Pixmap {
    let mut pm = Pixmap::new(width.max(1), height.max(1)).expect("pixmap dims are always >= 1");
    pm.fill(Color::from_rgba8(40, 40, 40, 255));
    pm
}

unsafe extern "C" fn source_get_name(_type_data: *mut c_void) -> *const std::ffi::c_char {
    c"Input Overlay".as_ptr()
}

unsafe extern "C" fn source_create(settings: *mut ObsData, _source: *mut ObsSource) -> *mut c_void {
    let shown = Arc::new(AtomicBool::new(true));
    let active = Arc::new(AtomicBool::new(true));
    let frame_slot: FrameSlot = Arc::new(Mutex::new(None));
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
    let render_thread = spawn_render_worker(
        EVENT_BUS.get().map(|bus| bus.subscribe()),
        cmd_rx,
        frame_slot.clone(),
        shown.clone(),
        active.clone(),
        DEFAULT_WIDTH,
        DEFAULT_HEIGHT,
    );
    if render_thread.is_none() {
        tracing::error!("video_source: failed to spawn render worker thread");
    }

    let mut data = VideoSourceData {
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        texture: None,
        texture_size: None,
        pixmap: idle_pixmap(DEFAULT_WIDTH, DEFAULT_HEIGHT),
        dirty: true,
        link: String::new(),
        json_file: String::new(),
        use_link: false,
        configured: false,
        frame_slot,
        cmd_tx: Some(cmd_tx),
        render_thread,
        shown,
        active,
    };
    update_from_settings(&mut data, settings);
    Box::into_raw(Box::new(data)) as *mut c_void
}

unsafe extern "C" fn source_destroy(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    let mut boxed = Box::from_raw(data as *mut VideoSourceData);
    drop(boxed.cmd_tx.take());
    if let Some(handle) = boxed.render_thread.take() {
        let _ = handle.join();
    }
    if let Some(tex) = boxed.texture.take() {
        if let Some(api) = crate::obs::api() {
            (api.enter_graphics)();
            (api.texture_destroy)(tex);
            (api.leave_graphics)();
        }
    }
}

unsafe extern "C" fn source_get_width(data: *mut c_void) -> u32 {
    if data.is_null() {
        return 0;
    }
    (*(data as *mut VideoSourceData)).width
}

unsafe extern "C" fn source_get_height(data: *mut c_void) -> u32 {
    if data.is_null() {
        return 0;
    }
    (*(data as *mut VideoSourceData)).height
}

unsafe extern "C" fn source_show(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    (*(data as *mut VideoSourceData))
        .shown
        .store(true, Ordering::Relaxed);
}

unsafe extern "C" fn source_hide(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    (*(data as *mut VideoSourceData))
        .shown
        .store(false, Ordering::Relaxed);
}

unsafe extern "C" fn source_activate(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    (*(data as *mut VideoSourceData))
        .active
        .store(true, Ordering::Relaxed);
}

unsafe extern "C" fn source_deactivate(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    (*(data as *mut VideoSourceData))
        .active
        .store(false, Ordering::Relaxed);
}

unsafe extern "C" fn source_get_defaults(settings: *mut ObsData) {
    let Some(api) = crate::obs::api() else {
        return;
    };
    (api.data_set_default_string)(settings, c"link".as_ptr(), c"".as_ptr());
    (api.data_set_default_string)(settings, c"json_file".as_ptr(), c"".as_ptr());
    (api.data_set_default_bool)(settings, c"use_link".as_ptr(), false);
    (api.data_set_default_string)(settings, c"resolution".as_ptr(), c"512x512".as_ptr());
    (api.data_set_default_string)(settings, c"analog_keyboard".as_ptr(), c"".as_ptr());
    #[cfg(target_os = "linux")]
    {
        (api.data_set_default_string)(settings, c"linux_kbd_device".as_ptr(), c"".as_ptr());
        (api.data_set_default_string)(settings, c"linux_mouse_device".as_ptr(), c"".as_ptr());
    }
}

unsafe extern "C" fn on_use_link_toggled(
    props: *mut ObsProperties,
    _property: *mut ObsProperty,
    settings: *mut ObsData,
) -> bool {
    let Some(api) = crate::obs::api() else {
        return false;
    };
    let use_link = (api.data_get_bool)(settings, c"use_link".as_ptr());
    let json_prop = (api.properties_get)(props, c"json_file".as_ptr());
    let link_prop = (api.properties_get)(props, c"link".as_ptr());
    set_visible(api, json_prop, !use_link);
    set_visible(api, link_prop, use_link);
    true
}

unsafe extern "C" fn source_get_properties(data: *mut c_void) -> *mut ObsProperties {
    let Some(api) = crate::obs::api() else {
        return std::ptr::null_mut();
    };
    let use_link_now = if data.is_null() {
        false
    } else {
        (*(data as *mut VideoSourceData)).use_link
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

    group(api, p, b"grp_overlay\0", b"Overlay\0", |g| {
        let json_prop = file_path(
            api,
            g,
            b"json_file\0",
            b"Config File\0",
            b"JSON Files (*.json)\0",
        );
        let link_prop = text(api, g, b"link\0", b"Overlay Link\0", OBS_TEXT_DEFAULT);
        let use_link_prop = checkbox(api, g, b"use_link\0", b"Use Web link instead\0");
        set_visible(api, json_prop, !use_link_now);
        set_visible(api, link_prop, use_link_now);
        (api.property_set_modified_callback)(use_link_prop, Some(on_use_link_toggled));
        text(
            api,
            g,
            b"resolution\0",
            b"Resolution (WxH)\0",
            OBS_TEXT_DEFAULT,
        );
    });

    group(api, p, b"grp_global\0", b"Global Settings\0", |g| {
        group(api, g, b"grp_capture\0", b"Capture\0", |g2| {
            let ak = list(api, g2, b"analog_keyboard\0", b"Analog Keyboard\0");
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
                g2,
                b"analogsense_note\0",
                b"Analog support mirrors the <a href=\"https://github.com/AnalogSense/JavaScript-SDK\">AnalogSense SDK</a> ported to Rust. Amazing project, please show them some love and PR new keyboards if you can!\0",
            );

            #[cfg(target_os = "linux")]
            {
                text(
                    api,
                    g2,
                    b"linux_kbd_device\0",
                    b"Keyboard Device\0",
                    OBS_TEXT_DEFAULT,
                );
                text(
                    api,
                    g2,
                    b"linux_mouse_device\0",
                    b"Mouse Device\0",
                    OBS_TEXT_DEFAULT,
                );
                info(
                    api,
                    g2,
                    b"linux_note\0",
                    b"e.g. /dev/input/event0 - changes require restarting OBS.\0",
                );
            }
            #[cfg(windows)]
            info(
                api,
                g2,
                b"ak_note\0",
                b"Analog keyboard change needs restarting OBS to take effect\0",
            );
        });

        group(api, g, b"grp_about\0", b"About\0", |g2| {
            info(
                api,
                g2,
                b"about_ver\0",
                concat!("input-overlay v", env!("CARGO_PKG_VERSION"), "\0").as_bytes(),
            );
            info(
                api,
                g2,
                b"about_configurator\0",
                b"<a href=\"https://overlay.girlglock.com\">Open Overlay Configurator</a>\0",
            );
            info(
                api,
                g2,
                b"about_links\0",
                b"<a href=\"https://github.com/girlglock/input-overlay\">GitHub</a> | <a href=\"https://twitter.com/girlglock_\">Twitter</a> | <a href=\"https://girlglock.com\">girlglock.com</a>\0",
            );
        });
    });

    p
}

unsafe extern "C" fn source_update(data: *mut c_void, settings: *mut ObsData) {
    if data.is_null() {
        return;
    }
    update_from_settings(&mut *(data as *mut VideoSourceData), settings);
}

fn load_from_link(link: &str) -> Option<DecodedLink> {
    if link.is_empty() {
        return None;
    }
    let decoded = decode_link(link);
    tracing::info!(
        "video_source: decoded link with {} layout element(s)",
        decoded.layout.len()
    );
    Some(decoded)
}

fn load_from_json_file(path: &str) -> Option<DecodedLink> {
    if path.is_empty() {
        return None;
    }
    let json = match std::fs::read_to_string(path) {
        Ok(json) => json,
        Err(e) => {
            tracing::error!("video_source: failed to read JSON config file {path}: {e}");
            return None;
        }
    };
    match decode_json_config(&json) {
        Some(decoded) => {
            tracing::info!(
                "video_source: decoded JSON config with {} layout element(s)",
                decoded.layout.len()
            );
            Some(decoded)
        }
        None => {
            tracing::error!("video_source: failed to parse JSON config file {path}");
            None
        }
    }
}

fn build_renderer(decoded: Option<DecodedLink>) -> Renderer {
    match decoded {
        None => Renderer::new(&default_layout(), &default_settings()),
        Some(decoded) => Renderer::new(&decoded.layout, &decoded.settings),
    }
}

fn send_command(cmd_tx: &Option<std::sync::mpsc::Sender<RenderCommand>>, cmd: RenderCommand) {
    if let Some(tx) = cmd_tx {
        let _ = tx.send(cmd);
    }
}

fn spawn_render_worker(
    mut event_rx: Option<broadcast::Receiver<InputEvent>>,
    cmd_rx: std::sync::mpsc::Receiver<RenderCommand>,
    frame_slot: FrameSlot,
    shown: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    initial_width: u32,
    initial_height: u32,
) -> Option<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("OverlayRender".into())
        .spawn(move || {
            let mut renderer: Option<Renderer> = None;
            let mut width = initial_width;
            let mut height = initial_height;

            loop {
                let mut shutdown = false;
                loop {
                    match cmd_rx.try_recv() {
                        Ok(RenderCommand::Reconfigure(decoded)) => {
                            renderer = Some(build_renderer(decoded.map(|b| *b)))
                        }
                        Ok(RenderCommand::Resize(w, h)) => {
                            width = w;
                            height = h;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            shutdown = true;
                            break;
                        }
                    }
                }
                if shutdown {
                    break;
                }

                if !shown.load(Ordering::Relaxed) || !active.load(Ordering::Relaxed) {
                    std::thread::sleep(RENDER_POLL_INTERVAL.max(Duration::from_millis(50)));
                    continue;
                }

                #[cfg(feature = "profiler")]
                let mut profiler = crate::render::profiler::Profiler::start();

                let mut changed = false;
                if let Some(rx) = event_rx.as_mut() {
                    loop {
                        match rx.try_recv() {
                            Ok(event) => {
                                if let Some(renderer) = renderer.as_mut() {
                                    if renderer.apply_event(&event) {
                                        changed = true;
                                    }
                                }
                            }
                            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => break,
                            Err(TryRecvError::Lagged(_)) => continue,
                        }
                    }
                }

                changed = changed || renderer.as_ref().is_some_and(|r| r.needs_constant_redraw());

                #[cfg(feature = "profiler")]
                profiler.mark("POLL");

                if changed {
                    if let Some(renderer) = renderer.as_mut() {
                        if let Some(mut pixmap) = Pixmap::new(width.max(1), height.max(1)) {
                            #[cfg(feature = "profiler")]
                            renderer.draw(&mut pixmap, |label, duration| match duration {
                                Some(d) => profiler.record(label, d),
                                None => profiler.mark(label),
                            });
                            #[cfg(not(feature = "profiler"))]
                            renderer.draw(&mut pixmap, |_, _| {});

                            #[cfg(feature = "profiler")]
                            profiler.draw_overlay(&mut pixmap);

                            *frame_slot.lock().unwrap() = Some(RenderedFrame {
                                width,
                                height,
                                pixmap,
                            });
                        }
                    }
                }

                std::thread::sleep(RENDER_POLL_INTERVAL);
            }
        })
        .ok()
}

unsafe fn update_from_settings(data: &mut VideoSourceData, settings: *mut ObsData) {
    let Some(api) = crate::obs::api() else {
        return;
    };

    let (width, height) = parse_resolution(&read_str(api, settings, b"resolution\0"));
    let size_changed = (width, height) != (data.width, data.height);
    data.width = width;
    data.height = height;

    let link = read_str(api, settings, b"link\0");
    let json_file = read_str(api, settings, b"json_file\0");
    let use_link = (api.data_get_bool)(settings, c"use_link".as_ptr());
    let is_first_configure = !data.configured;
    let source_changed = is_first_configure
        || link != data.link
        || json_file != data.json_file
        || use_link != data.use_link;
    if source_changed {
        data.link = link;
        data.json_file = json_file;
        data.use_link = use_link;
        data.configured = true;

        if data.use_link {
            send_command(
                &data.cmd_tx,
                RenderCommand::Reconfigure(load_from_link(&data.link).map(Box::new)),
            );
        } else {
            let path = data.json_file.clone();
            let cmd_tx = data.cmd_tx.clone();
            let spawned = std::thread::Builder::new()
                .name("OverlayJsonConfig".into())
                .spawn(move || {
                    send_command(
                        &cmd_tx,
                        RenderCommand::Reconfigure(load_from_json_file(&path).map(Box::new)),
                    );
                })
                .is_ok();
            if !spawned {
                tracing::error!("video_source: failed to spawn JSON config load thread");
            }
            if is_first_configure {
                send_command(&data.cmd_tx, RenderCommand::Reconfigure(None));
            }
        }
    }

    if size_changed {
        send_command(&data.cmd_tx, RenderCommand::Resize(data.width, data.height));
        data.pixmap = idle_pixmap(data.width, data.height);
        data.dirty = true;
    }

    let analog_keyboard = read_str(api, settings, b"analog_keyboard\0");
    #[cfg(target_os = "linux")]
    let linux_kbd = read_str(api, settings, b"linux_kbd_device\0");
    #[cfg(target_os = "linux")]
    let linux_mouse = read_str(api, settings, b"linux_mouse_device\0");

    persist_capture_settings(
        analog_keyboard,
        #[cfg(target_os = "linux")]
        linux_kbd,
        #[cfg(target_os = "linux")]
        linux_mouse,
    );
}

fn persist_capture_settings(
    analog_keyboard: String,
    #[cfg(target_os = "linux")] linux_kbd: String,
    #[cfg(target_os = "linux")] linux_mouse: String,
) {
    let guard = PLUGIN_CONFIG.lock().unwrap();
    let Some(state) = guard.as_ref() else {
        return;
    };

    let cfg_snapshot = {
        let mut cfg = state.config.lock().unwrap();
        cfg.analog_keyboard = analog_keyboard;
        #[cfg(target_os = "linux")]
        {
            cfg.linux_evdev_keyboard_device = linux_kbd;
            cfg.linux_raw_mouse_device = linux_mouse;
        }
        cfg.clone()
    };
    let config_path = state.config_path.clone();
    drop(guard);

    let spawned = std::thread::Builder::new()
        .name("OverlayConfigSave".into())
        .spawn(move || {
            if let Err(e) = io_ws_common::services::config::save(&config_path, &cfg_snapshot) {
                tracing::error!("failed to save config: {e}");
            }
        })
        .is_ok();
    if !spawned {
        tracing::error!("video_source: failed to spawn config save thread");
    }
}

unsafe extern "C" fn source_video_tick(data: *mut c_void, _seconds: f32) {
    if data.is_null() {
        return;
    }
    let data = &mut *(data as *mut VideoSourceData);
    let Some(api) = crate::obs::api() else {
        return;
    };

    if !data.shown.load(Ordering::Relaxed) || !data.active.load(Ordering::Relaxed) {
        return;
    }

    if let Some(frame) = data.frame_slot.lock().unwrap().take() {
        if frame.width == data.width && frame.height == data.height {
            data.pixmap = frame.pixmap;
            data.dirty = true;
        }
    }

    if data.texture_size != Some((data.width, data.height)) {
        (api.enter_graphics)();
        if let Some(tex) = data.texture.take() {
            (api.texture_destroy)(tex);
        }
        let tex = (api.texture_create)(
            data.width,
            data.height,
            GS_COLOR_FORMAT_RGBA,
            1,
            std::ptr::null(),
            GS_TEXTURE_DYNAMIC,
        );
        if tex.is_null() {
            tracing::error!("video_source: gs_texture_create failed");
            data.texture_size = None;
        } else {
            data.texture = Some(tex);
            data.texture_size = Some((data.width, data.height));
            data.dirty = true;
        }
        (api.leave_graphics)();
    }

    if data.dirty {
        if let Some(tex) = data.texture {
            (api.enter_graphics)();
            (api.texture_set_image)(tex, data.pixmap.data().as_ptr(), data.width * 4, false);
            (api.leave_graphics)();
        }
        data.dirty = false;
    }
}

unsafe extern "C" fn source_video_render(data: *mut c_void, effect: *mut ObsEffect) {
    if data.is_null() {
        return;
    }
    let data = &*(data as *mut VideoSourceData);
    if !data.shown.load(Ordering::Relaxed) || !data.active.load(Ordering::Relaxed) {
        return;
    }
    let Some(api) = crate::obs::api() else {
        return;
    };
    let Some(tex) = data.texture else {
        return;
    };

    let param = (api.effect_get_param_by_name)(effect, c"image".as_ptr());
    if param.is_null() {
        return;
    }
    (api.effect_set_texture)(param, tex);

    (api.blend_state_push)();
    (api.blend_function)(GS_BLEND_ONE, GS_BLEND_INVSRCALPHA);
    (api.draw_sprite)(tex, 0, data.width, data.height);
    (api.blend_state_pop)();
}

fn parse_resolution(s: &str) -> (u32, u32) {
    let s = s.trim().to_lowercase();
    if let Some((w, h)) = s.split_once('x') {
        if let (Ok(w), Ok(h)) = (w.trim().parse::<i64>(), h.trim().parse::<i64>()) {
            let w = w.clamp(MIN_DIMENSION as i64, MAX_DIMENSION as i64) as u32;
            let h = h.clamp(MIN_DIMENSION as i64, MAX_DIMENSION as i64) as u32;
            return (w, h);
        }
    }
    (DEFAULT_WIDTH, DEFAULT_HEIGHT)
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