mod services;
mod ws_server;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tokio::sync::{watch, RwLock};

use services::config::Config;
use ws_server::{InputEvent, ServerStatus, WsState};
#[cfg(any(windows, target_os = "linux"))]
use tauri::Emitter;

struct AppState {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
    status: Arc<Mutex<ServerStatus>>,
    ws_state: Arc<WsState>,
    #[cfg(windows)]
    _raw_input: Mutex<Option<services::windows::raw_input::RawInputThread>>,
    #[cfg(windows)]
    update_cache: Mutex<Option<services::windows::updater::UpdateInfo>>,
    #[cfg(target_os = "linux")]
    update_cache: Mutex<Option<services::linux::updater::UpdateInfo>>,
    #[cfg(target_os = "linux")]
    _evdev: Mutex<Option<services::linux::evdev_input::EvdevInputThread>>,
    analog: Mutex<Option<services::analog::AnalogThread>>,
    http: Mutex<Option<services::http_server::HttpServer>>,
}

//tairi io-----------------------------------------------------------------------
#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
async fn save_config(
    new_cfg: Config,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let (need_rebind, need_analog_restart, need_http_restart, need_evdev_restart) = {
        let old = state.config.read().await;
        (
            old.host != new_cfg.host || old.port != new_cfg.port,
            old.analog_keyboard != new_cfg.analog_keyboard,
            old.http_enabled != new_cfg.http_enabled
                || old.http_port != new_cfg.http_port
                || old.host != new_cfg.host,
            old.linux_evdev_keyboard_device != new_cfg.linux_evdev_keyboard_device
                || old.linux_raw_mouse_device != new_cfg.linux_raw_mouse_device,
        )
    };

    services::config::save(&state.config_path, &new_cfg).map_err(|e| e.to_string())?;

    let new_analog_kb  = new_cfg.analog_keyboard.clone();
    let new_http_host  = new_cfg.host.clone();
    let new_http_port  = new_cfg.http_port;
    let new_http_on    = new_cfg.http_enabled;
    #[cfg(target_os = "linux")]
    let (new_kbd_dev, new_mouse_dev, new_min_delta) = (
        new_cfg.linux_evdev_keyboard_device.clone(),
        new_cfg.linux_raw_mouse_device.clone(),
        new_cfg.raw_mouse_min_delta,
    );
    *state.config.write().await = new_cfg;

    if need_rebind {
        let _ = state.ws_state.rebind_tx.send(());
    }

    if need_analog_restart {
        let new_thread = if !new_analog_kb.is_empty() {
            Some(services::analog::AnalogThread::start(
                state.ws_state.input_tx.clone(),
                &new_analog_kb,
            ))
        } else {
            None
        };
        *state.analog.lock().unwrap() = new_thread;
    }

    if need_http_restart {
        let new_server = if new_http_on {
            services::http_server::HttpServer::start(&new_http_host, new_http_port)
        } else {
            None
        };
        *state.http.lock().unwrap() = new_server;
    }

    #[cfg(target_os = "linux")]
    if need_evdev_restart {
        let new_thread = services::linux::evdev_input::EvdevInputThread::start(
            state.ws_state.input_tx.clone(),
            &new_kbd_dev,
            &new_mouse_dev,
            new_min_delta,
        );
        *state._evdev.lock().unwrap() = Some(new_thread);
    }
    #[cfg(windows)]
    let _ = need_evdev_restart;

    Ok(())
}

#[tauri::command]
async fn toggle_http(
    enabled: bool,
    host: String,
    port: u16,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let new_server = if enabled {
        services::http_server::HttpServer::start(&host, port)
    } else {
        None
    };
    *state.http.lock().unwrap() = new_server;
    let mut cfg = state.config.write().await;
    cfg.http_enabled = enabled;
    cfg.http_port = port;
    services::config::save(&state.config_path, &cfg).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_status(state: tauri::State<'_, AppState>) -> Result<ServerStatus, String> {
    Ok(state.status.lock().unwrap().clone())
}

#[tauri::command]
fn get_autostart() -> bool {
    #[cfg(target_os = "linux")]
    { return services::linux::autostart::is_enabled(); }
    #[cfg(windows)]
    services::windows::autostart::is_enabled()
}

#[tauri::command]
fn set_autostart(enabled: bool) -> bool {
    let exe = std::env::current_exe().unwrap_or_default();
    #[cfg(target_os = "linux")]
    { return services::linux::autostart::set_enabled(enabled, &exe); }
    #[cfg(windows)]
    services::windows::autostart::set_enabled(enabled, &exe)
}

#[tauri::command]
fn minimize_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn close_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
fn is_admin() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
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
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elev.TokenIsElevated != 0
    }
    #[cfg(not(windows))]
    {
        true
    }
}

#[tauri::command]
async fn set_theme(theme: Option<String>, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut cfg = state.config.write().await;
    cfg.theme = theme;
    services::config::save(&state.config_path, &cfg).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_url(url: String) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", &url])
            .creation_flags(0x08000000)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    }
}

#[cfg(target_os = "linux")]
#[tauri::command]
fn enum_keyboards() -> Vec<(String, String)> {
    services::linux::evdev_input::enum_keyboards()
}

#[cfg(target_os = "linux")]
#[tauri::command]
fn enum_mice() -> Vec<(String, String)> {
    services::linux::evdev_input::enum_mice()
}

#[cfg(target_os = "linux")]
#[tauri::command]
fn check_linux_perms() -> Vec<String> {
    services::linux::evdev_input::check_permissions()
}

//updater-----------------------------------------------------------------------
#[tauri::command]
fn can_auto_update() -> bool {
    cfg!(windows)
}

#[cfg(windows)]
#[tauri::command]
fn check_update(
    state: tauri::State<'_, AppState>,
) -> Option<services::windows::updater::UpdateInfo> {
    state.update_cache.lock().unwrap().clone()
}

#[cfg(target_os = "linux")]
#[tauri::command]
fn check_update(
    state: tauri::State<'_, AppState>,
) -> Option<services::linux::updater::UpdateInfo> {
    state.update_cache.lock().unwrap().clone()
}

#[cfg(any(windows, target_os = "linux"))]
#[tauri::command]
async fn dismiss_update(version: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut cfg = state.config.write().await;
    if !cfg.dismissed_update_versions.contains(&version) {
        cfg.dismissed_update_versions.push(version);
        services::config::save(&state.config_path, &cfg).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
#[tauri::command]
async fn apply_update(
    download_url: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    services::windows::updater::download_and_schedule(&download_url, &current_exe, &app).await?;
    app.exit(0);
    Ok(())
}

//cpu affinity-----------------------------------------------------------------------
#[cfg(windows)]
fn apply_cpu_affinity(cores: &[u32]) {
    if cores.is_empty() {
        return;
    }
    use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessAffinityMask};
    let mask: usize = cores.iter().fold(0usize, |acc, &c| acc | (1 << c));
    unsafe {
        let proc = GetCurrentProcess();
        if let Err(e) = SetProcessAffinityMask(proc, mask) {
            tracing::warn!("SetProcessAffinityMask failed: {e}");
        } else {
            tracing::info!("cpu affinity set: cores {cores:?} mask=0x{mask:x}");
        }
    }
}

#[cfg(not(windows))]
fn apply_cpu_affinity(_cores: &[u32]) {}

//app init-----------------------------------------------------------------------
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    //webkit2gtk might fail gpu accel or dmabuf on fedora with nvidia-open.. im just gonna disable it entirely for now
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    let data_dir = std::env::current_exe()
        .expect("failed to get exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();

    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).ok();
    let log_filename = format!("{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
    let file_appender = tracing_appender::rolling::never(&log_dir, &log_filename);
    let (non_blocking_file, _log_guard) = tracing_appender::non_blocking(file_appender);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(non_blocking_file))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(move |app| {
            let config_path = data_dir.join("config.json");
            let mut cfg = services::config::load(&config_path);
            if cfg.auth_token.is_empty() {
                use rand::Rng;
                cfg.auth_token = rand::thread_rng()
                    .sample_iter(rand::distributions::Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect();
                let _ = services::config::save(&config_path, &cfg);
                tracing::info!("generated auth token: {}****", &cfg.auth_token[..4]);
            }

            apply_cpu_affinity(&cfg.cpu_affinity.clone());

            let analog_kb    = cfg.analog_keyboard.clone();
            let http_enabled = cfg.http_enabled;
            let http_host    = cfg.host.clone();
            let http_port    = cfg.http_port;
            #[cfg(target_os = "linux")]
            let (evdev_kbd, evdev_mouse, evdev_min_delta) = (
                cfg.linux_evdev_keyboard_device.clone(),
                cfg.linux_raw_mouse_device.clone(),
                cfg.raw_mouse_min_delta,
            );
            let config = Arc::new(RwLock::new(cfg));
            let status = Arc::new(Mutex::new(ServerStatus::default()));
            let (rebind_tx, rebind_rx) = watch::channel(());
            let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel::<InputEvent>();

            let ws_state = Arc::new(WsState {
                rebind_tx,
                input_tx: input_tx.clone(),
            });

            {
                let cfg = Arc::clone(&config);
                let st = Arc::clone(&status);
                let ah = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    ws_server::run(cfg, input_rx, st, rebind_rx, ah).await;
                });
            }

            #[cfg(windows)]
            let raw_input_handle = {
                let min_delta = config.try_read().map(|c| c.raw_mouse_min_delta).unwrap_or(0);
                Some(services::windows::raw_input::RawInputThread::start(input_tx.clone(), min_delta))
            };

            #[cfg(target_os = "linux")]
            let evdev_handle = Some(services::linux::evdev_input::EvdevInputThread::start(
                input_tx.clone(),
                &evdev_kbd,
                &evdev_mouse,
                evdev_min_delta,
            ));

            let analog_handle = if !analog_kb.is_empty() {
                Some(services::analog::AnalogThread::start(input_tx, &analog_kb))
            } else {
                None
            };

            let http_handle = if http_enabled {
                services::http_server::HttpServer::start(&http_host, http_port)
            } else {
                None
            };

            app.manage(AppState {
                config,
                config_path,
                status,
                ws_state,
                #[cfg(windows)]
                _raw_input: Mutex::new(raw_input_handle),
                #[cfg(windows)]
                update_cache: Mutex::new(None),
                #[cfg(target_os = "linux")]
                update_cache: Mutex::new(None),
                #[cfg(target_os = "linux")]
                _evdev: Mutex::new(evdev_handle),
                analog: Mutex::new(analog_handle),
                http: Mutex::new(http_handle),
            });

            #[cfg(windows)]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = handle.state::<AppState>();
                    let dismissed = state.config.read().await.dismissed_update_versions.clone();
                    let current   = handle.package_info().version.to_string();
                    if let Some(info) = services::windows::updater::check(&current, &dismissed).await {
                        *state.update_cache.lock().unwrap() = Some(info.clone());
                        let _ = handle.emit("update-available", &info);
                        if handle.get_webview_window("update").is_none() {
                            let _ = open_update_window(&handle);
                        }
                    }
                });
            }

            #[cfg(target_os = "linux")]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = handle.state::<AppState>();
                    let dismissed = state.config.read().await.dismissed_update_versions.clone();
                    let current   = handle.package_info().version.to_string();
                    if let Some(info) = services::linux::updater::check(&current, &dismissed).await {
                        *state.update_cache.lock().unwrap() = Some(info.clone());
                        let _ = handle.emit("update-available", &info);
                        if handle.get_webview_window("update").is_none() {
                            let _ = open_update_window(&handle);
                        }
                    }
                });
            }

            build_tray(app)?;
            Ok(())
        })
        .invoke_handler({
            #[cfg(windows)]
            { tauri::generate_handler![
                get_config, save_config, get_status,
                get_autostart, set_autostart, toggle_http,
                minimize_window, close_window, open_url, is_admin, set_theme,
                can_auto_update, check_update, dismiss_update, apply_update,
            ] }
            #[cfg(target_os = "linux")]
            { tauri::generate_handler![
                get_config, save_config, get_status,
                get_autostart, set_autostart, toggle_http,
                minimize_window, close_window, open_url, is_admin, set_theme,
                can_auto_update, check_update, dismiss_update,
                enum_keyboards, enum_mice, check_linux_perms,
            ] }
            #[cfg(not(any(windows, target_os = "linux")))]
            { tauri::generate_handler![
                get_config, save_config, get_status,
                get_autostart, set_autostart, toggle_http,
                minimize_window, close_window, open_url, is_admin, set_theme,
                can_auto_update,
            ] }
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

    let Some(icon) = app.default_window_icon() else {
        tracing::warn!("no window icon available for tray");
        return Ok(());
    };

    let _tray = TrayIconBuilder::new()
        .icon(icon.clone())
        .tooltip("Input Overlay")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => toggle_window(app),
            "quit" => {
                tracing::info!("shutting down");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let _ = open_main_window(app);
}

fn open_update_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "update",
        tauri::WebviewUrl::App("update.html".into()),
    )
    .title("Update Available")
    .inner_size(560.0, 600.0)
    .decorations(false)
    .resizable(false)
    .center()
    .focused(true);

    #[cfg(windows)]
    {
        builder = builder.transparent(true).shadow(false);
    }

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone())?;
    }

    builder.build()?;
    Ok(())
}

fn open_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "main",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Input Overlay WS - Settings")
    .inner_size(420.0, 580.0)
    .resizable(true)
    .decorations(false)
    .skip_taskbar(false)
    .center();

    #[cfg(windows)]
    {
        builder = builder.transparent(true).shadow(false);
    }

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone())?;
    }

    builder.build()?;
    Ok(())
}
