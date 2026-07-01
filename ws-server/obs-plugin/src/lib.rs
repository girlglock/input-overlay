mod obs;
mod settings_ui;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;

use tokio::sync::watch;

use io_ws_common::ws_server::{InputEvent, ServerStatus, WsState};
use io_ws_common::services;

static OBS_MODULE_PTR: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

static PLUGIN_STATE: Mutex<Option<PluginState>> = Mutex::new(None);

struct PluginState {
    _runtime: tokio::runtime::Runtime,
    ws_task:  tokio::task::JoinHandle<()>,
    _ws_state: Arc<WsState>,
    #[cfg(windows)]
    raw_input: Option<services::windows::raw_input::RawInputThread>,
    #[cfg(target_os = "linux")]
    _evdev: Option<services::linux::evdev_input::EvdevInputThread>,
    _analog:   Option<services::analog::AnalogThread>,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}

#[no_mangle]
pub unsafe extern "C" fn obs_module_set_pointer(module: *mut std::ffi::c_void) {
    OBS_MODULE_PTR.store(module, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn obs_module_ver() -> u32 {
    30 << 24
}

#[no_mangle]
pub extern "C" fn obs_module_name() -> *const std::os::raw::c_char {
    b"Input Overlay WS Server\0".as_ptr() as *const _
}

#[no_mangle]
pub extern "C" fn obs_module_description() -> *const std::os::raw::c_char {
    b"WebSocket input server for input-overlay\0".as_ptr() as *const _
}

#[no_mangle]
pub extern "C" fn obs_module_load() -> bool {
    let log_guard = setup_logging();

    let config_dir = plugin_config_dir();
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        eprintln!("input-overlay-ws: failed to create config dir: {e}");
        return false;
    }

    let config_path = config_dir.join("config.json");
    let mut cfg = services::config::load(&config_path);

    if cfg.auth_token.is_empty() {
        cfg.auth_token = gen_token(32);
        let _ = services::config::save(&config_path, &cfg);
        tracing::info!("generated auth token");
    }

    let analog_kb = cfg.analog_keyboard.clone();
    #[cfg(windows)]
    let (min_delta, flush_hz) = (cfg.raw_mouse_min_delta, cfg.flush_hz);
    #[cfg(target_os = "linux")]
    let (kbd_dev, mouse_dev, min_delta, flush_hz) = (
        cfg.linux_evdev_keyboard_device.clone(),
        cfg.linux_raw_mouse_device.clone(),
        cfg.raw_mouse_min_delta,
        cfg.flush_hz,
    );

    let config = Arc::new(Mutex::new(cfg));
    let status = Arc::new(Mutex::new(ServerStatus::default()));
    let (rebind_tx, rebind_rx) = watch::channel(());
    let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel::<InputEvent>();

    let ws_state = Arc::new(WsState {
        rebind_tx,
        input_tx: input_tx.clone(),
    });

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("failed to create tokio runtime: {e}");
            return false;
        }
    };

    let ws_task = runtime.spawn({
        let cfg = Arc::clone(&config);
        let st  = Arc::clone(&status);
        async move { io_ws_common::ws_server::run(cfg, input_rx, st, rebind_rx, |_| {}).await }
    });

    #[cfg(windows)]
    let raw_input = Some(services::windows::raw_input::RawInputThread::start(
        input_tx.clone(),
        min_delta,
        64,
        false,
    ));

    #[cfg(target_os = "linux")]
    let evdev = Some(services::linux::evdev_input::EvdevInputThread::start(
        input_tx.clone(),
        &kbd_dev,
        &mouse_dev,
        min_delta,
        64,
    ));

    let analog = if !analog_kb.is_empty() {
        Some(services::analog::AnalogThread::start(input_tx, &analog_kb))
    } else {
        None
    };

    #[cfg(any(windows, target_os = "linux"))]
    if obs::init() {
        settings_ui::init(Arc::clone(&config), config_path.clone(), Arc::clone(&ws_state));
    }

    *PLUGIN_STATE.lock().unwrap() = Some(PluginState {
        _runtime: runtime,
        ws_task,
        _ws_state: ws_state,
        #[cfg(windows)]
        raw_input,
        #[cfg(target_os = "linux")]
        _evdev: evdev,
        _analog: analog,
        _log_guard: log_guard,
    });

    tracing::info!("input-overlay ws server plugin loaded");
    true
}

#[no_mangle]
pub extern "C" fn obs_module_unload() {
    #[cfg(any(windows, target_os = "linux"))]
    settings_ui::release_singleton();

    let state = PLUGIN_STATE.lock().unwrap().take();
    if let Some(mut s) = state {
        s.ws_task.abort();

        #[cfg(windows)]
        if let Some(mut ri) = s.raw_input.take() {
            ri.stop();
        }

        drop(s._analog);
        #[cfg(target_os = "linux")]
        drop(s._evdev);
        drop(s._ws_state);
        drop(s._runtime);
    }
    tracing::info!("input-overlay ws server plugin unloaded");
}

#[cfg(windows)]
pub fn plugin_config_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("obs-studio")
        .join("plugin_config")
        .join("input-overlay-ws-server")
}

#[cfg(target_os = "linux")]
pub fn plugin_config_dir() -> PathBuf {
    let base = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join(".config")
        .join("obs-studio")
        .join("plugin_config")
        .join("input-overlay-ws-server")
}

fn setup_logging() -> tracing_appender::non_blocking::WorkerGuard {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let log_dir = plugin_config_dir().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_name = format!("{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
    let appender = tracing_appender::rolling::never(&log_dir, &log_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .try_init();

    guard
}

pub(crate) fn gen_token(len: usize) -> String {
    use std::sync::atomic::AtomicU64;
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    (0..len as u64)
        .map(|i| {
            let c = COUNTER.fetch_add(1, Ordering::Relaxed);
            let v = t
                .wrapping_mul(6364136223846793005)
                .wrapping_add(c.wrapping_mul(1442695040888963407))
                .wrapping_add(i.wrapping_mul(2891336453));
            CHARS[((v >> 33) as usize) % CHARS.len()] as char
        })
        .collect()
}
