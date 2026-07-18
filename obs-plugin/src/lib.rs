mod obs;
mod obs_helpers;
mod overlay_link;
mod render;
mod video_source;

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use io_ws_common::services;
use io_ws_common::input_event::InputEvent;

static OBS_MODULE_PTR: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

static PLUGIN_STATE: Mutex<Option<PluginState>> = Mutex::new(None);

struct PluginState {
    _runtime: tokio::runtime::Runtime,
    _relay_task: tokio::task::JoinHandle<()>,
    #[cfg(windows)]
    raw_input: Option<services::windows::raw_input::RawInputThread>,
    #[cfg(target_os = "linux")]
    _evdev: Option<services::linux::evdev_input::EvdevInputThread>,
    _analog: Option<services::analog::AnalogThread>,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}

/// # Safety
///
/// called by obs with a module pointer
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
    c"Input Overlay".as_ptr()
}

#[no_mangle]
pub extern "C" fn obs_module_description() -> *const std::os::raw::c_char {
    c"Input Overlay (overlay.girlglock.com)".as_ptr()
}

#[no_mangle]
pub extern "C" fn obs_module_load() -> bool {
    let log_guard = setup_logging();

    let config_dir = plugin_config_dir();
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        eprintln!("input-overlay: failed to create config dir: {e}");
        return false;
    }

    let config_path = config_dir.join("config.json");
    let cfg = services::config::load(&config_path);

    let analog_kb = cfg.analog_keyboard.clone();
    #[cfg(target_os = "linux")]
    let (kbd_dev, mouse_dev, min_delta) = (
        cfg.linux_evdev_keyboard_device.clone(),
        cfg.linux_raw_mouse_device.clone(),
        cfg.raw_mouse_min_delta,
    );
    #[cfg(windows)]
    let min_delta = cfg.raw_mouse_min_delta;

    let config = Arc::new(Mutex::new(cfg));

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

    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<InputEvent>();
    let (bus_tx, _bus_rx0) = tokio::sync::broadcast::channel::<InputEvent>(1024);

    let relay_bus = bus_tx.clone();
    let relay_task = runtime.spawn(async move {
        while let Some(event) = input_rx.recv().await {
            let _ = relay_bus.send(event);
        }
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
        Some(services::analog::AnalogThread::start(
            input_tx, &analog_kb, 64,
        ))
    } else {
        None
    };

    #[cfg(any(windows, target_os = "linux"))]
    if obs::init() {
        video_source::init(bus_tx, Arc::clone(&config), config_path.clone());
    }

    *PLUGIN_STATE.lock().unwrap() = Some(PluginState {
        _runtime: runtime,
        _relay_task: relay_task,
        #[cfg(windows)]
        raw_input,
        #[cfg(target_os = "linux")]
        _evdev: evdev,
        _analog: analog,
        _log_guard: log_guard,
    });

    tracing::info!("input-overlay plugin loaded");
    true
}

#[no_mangle]
pub extern "C" fn obs_module_unload() {
    let state = PLUGIN_STATE.lock().unwrap().take();
    if let Some(mut s) = state {
        s._relay_task.abort();

        #[cfg(windows)]
        if let Some(mut ri) = s.raw_input.take() {
            ri.stop();
        }

        drop(s._analog);
        #[cfg(target_os = "linux")]
        drop(s._evdev);
        drop(s._runtime);
    }
    tracing::info!("input-overlay plugin unloaded");
}

#[cfg(windows)]
pub fn plugin_config_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("obs-studio")
        .join("plugin_config")
        .join("input-overlay")
}

#[cfg(target_os = "linux")]
pub fn plugin_config_dir() -> PathBuf {
    let base = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join(".config")
        .join("obs-studio")
        .join("plugin_config")
        .join("input-overlay")
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
