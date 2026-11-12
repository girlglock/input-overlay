use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use io_ws_common::input_event::InputEvent;
use tokio::sync::watch;

use crate::services;
use crate::services::config::Config;
use crate::services::updater::UpdateInfo;
use crate::ui;
use crate::views::{PostUpdateView, SettingsView, UpdateView};
use crate::ws_server::{self, ServerStatus, WsState};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to start tokio runtime")
    })
}

pub enum AppMsg {
    Status(ServerStatus),
    UpdateAvailable(UpdateInfo),
    UpdateProgress { percent: u32, status: String },
    UpdateFailed(String),
    UpdateApplied,
    ToggleSettings,
    FocusSettings,
    Quit,
}

pub struct App {
    pub config: Arc<Mutex<Config>>,
    pub config_path: PathBuf,
    pub status: Arc<Mutex<ServerStatus>>,
    ws_state: Arc<WsState>,
    #[cfg(windows)]
    raw_input: Option<services::windows::raw_input::RawInputThread>,
    #[cfg(target_os = "linux")]
    evdev: Option<services::linux::evdev_input::EvdevInputThread>,
    analog: Option<services::analog::AnalogThread>,
    http: Option<services::http_server::HttpServer>,
    pub update_info: Option<UpdateInfo>,
    pub tx: Sender<AppMsg>,
    quit: bool,
}

impl App {
    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    pub fn server_status(&self) -> ServerStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn dark_theme(&self) -> bool {
        match self.config.lock().unwrap().theme.as_deref() {
            Some("dark") => true,
            Some("light") => false,
            _ => crate::system::system_prefers_dark(),
        }
    }

    pub fn set_theme(&mut self, dark: bool) -> Result<(), String> {
        ui::set_dark_theme(dark);
        let mut cfg = self.config.lock().unwrap();
        cfg.theme = Some(if dark { "dark" } else { "light" }.to_string());
        services::config::save(&self.config_path, &cfg).map_err(|e| e.to_string())
    }

    pub fn save_config(&mut self, new_cfg: Config) -> Result<(), String> {
        let (need_rebind, need_analog, need_http, need_evdev, need_raw, need_affinity, need_gamepad) = {
            let old = self.config.lock().unwrap();
            (
                old.host != new_cfg.host || old.port != new_cfg.port,
                old.analog_keyboard != new_cfg.analog_keyboard || old.flush_hz != new_cfg.flush_hz,
                old.http_enabled != new_cfg.http_enabled || old.http_port != new_cfg.http_port || old.host != new_cfg.host,
                old.linux_evdev_keyboard_device != new_cfg.linux_evdev_keyboard_device
                    || old.linux_raw_mouse_device != new_cfg.linux_raw_mouse_device
                    || old.flush_hz != new_cfg.flush_hz,
                old.flush_hz != new_cfg.flush_hz,
                old.cpu_affinity != new_cfg.cpu_affinity,
                old.gamepad_device != new_cfg.gamepad_device || old.flush_hz != new_cfg.flush_hz,
            )
        };

        services::config::save(&self.config_path, &new_cfg).map_err(|e| e.to_string())?;
        *self.config.lock().unwrap() = new_cfg.clone();

        if need_affinity {
            apply_cpu_affinity(&new_cfg.cpu_affinity);
        }
        if need_rebind {
            let _ = self.ws_state.rebind_tx.send(());
        }
        if need_analog {
            self.analog = None;
            if !new_cfg.analog_keyboard.is_empty() {
                self.analog = Some(services::analog::AnalogThread::start(
                    self.ws_state.input_tx.clone(),
                    &new_cfg.analog_keyboard,
                    new_cfg.flush_hz,
                ));
            }
        }
        if need_gamepad {
            services::gamepad::set_output(self.ws_state.input_tx.clone(), &new_cfg.gamepad_device, new_cfg.flush_hz);
        }
        if need_http {
            self.http = None;
            if new_cfg.http_enabled {
                self.http = services::http_server::HttpServer::start(&new_cfg.host, new_cfg.http_port);
            }
        }
        #[cfg(target_os = "linux")]
        {
            let _ = need_raw;
            if need_evdev {
                self.evdev = None;
                self.evdev = Some(services::linux::evdev_input::EvdevInputThread::start(
                    self.ws_state.input_tx.clone(),
                    &new_cfg.linux_evdev_keyboard_device,
                    &new_cfg.linux_raw_mouse_device,
                    new_cfg.raw_mouse_min_delta,
                    new_cfg.flush_hz,
                ));
            }
        }
        #[cfg(windows)]
        {
            let _ = need_evdev;
            if need_raw {
                self.raw_input = None;
                self.raw_input = Some(services::windows::raw_input::RawInputThread::start(
                    self.ws_state.input_tx.clone(),
                    new_cfg.raw_mouse_min_delta,
                    new_cfg.flush_hz,
                    true,
                ));
            }
        }
        Ok(())
    }

    pub fn apply_bind(&mut self, host: String, port: u16) {
        {
            let mut cfg = self.config.lock().unwrap();
            cfg.host = host;
            cfg.port = port;
        }
        let _ = self.ws_state.rebind_tx.send(());
    }

    pub fn toggle_http(&mut self, enabled: bool, host: &str, port: u16) -> Result<(), String> {
        self.http = None;
        if enabled {
            self.http = services::http_server::HttpServer::start(host, port);
        }
        let mut cfg = self.config.lock().unwrap();
        cfg.http_enabled = enabled;
        cfg.http_port = port;
        services::config::save(&self.config_path, &cfg).map_err(|e| e.to_string())
    }

    //updater-----------------------------------------------------------------------
    pub fn dismiss_update(&mut self, version: &str) {
        let mut cfg = self.config.lock().unwrap();
        if !cfg.dismissed_update_versions.iter().any(|v| v == version) {
            cfg.dismissed_update_versions.push(version.to_string());
            let _ = services::config::save(&self.config_path, &cfg);
        }
    }

    pub fn apply_update(&self, info: &UpdateInfo) {
        let tx = self.tx.clone();
        let (url, version) = (info.download_url.clone(), info.version.clone());
        runtime().spawn(async move {
            let progress_tx = tx.clone();
            let progress = move |percent: u32, status: &str| {
                let _ = progress_tx.send(AppMsg::UpdateProgress { percent, status: status.to_string() });
                ui::wake();
            };
            #[cfg(windows)]
            let result = match std::env::current_exe() {
                Ok(exe) => services::windows::updater::download_and_apply(&url, &version, &exe, &progress).await,
                Err(e) => Err(e.to_string()),
            };
            #[cfg(target_os = "linux")]
            let result = services::linux::updater::download_and_apply(&url, &version, &progress).await;
            let _ = tx.send(match result {
                Ok(()) => AppMsg::UpdateApplied,
                Err(e) => AppMsg::UpdateFailed(e),
            });
            ui::wake();
        });
    }

    pub fn quit(&mut self) {
        tracing::info!("shutting down");
        self.quit = true;
    }
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
        if let Err(e) = SetProcessAffinityMask(GetCurrentProcess(), mask) {
            tracing::warn!("SetProcessAffinityMask failed: {e}");
        } else {
            tracing::info!("cpu affinity set: cores {cores:?} mask=0x{mask:x}");
        }
    }
}

#[cfg(target_os = "linux")]
fn apply_cpu_affinity(_cores: &[u32]) {}

fn data_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .expect("failed to get exe path")
        .parent()
        .expect("exe has no parent dir")
        .to_path_buf();
    //for appimage  use the dir where the appimage is in, not the temp dir it runs in :p
    #[cfg(target_os = "linux")]
    let exe_dir = std::env::var("APPIMAGE")
        .ok()
        .and_then(|p| PathBuf::from(p).parent().map(|p| p.to_path_buf()))
        .unwrap_or(exe_dir);
    exe_dir
}

fn init_logging(data_dir: &std::path::Path) -> tracing_appender::non_blocking::WorkerGuard {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    let log_dir = data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).ok();
    let log_filename = format!("{}.log", chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
    let file_appender = tracing_appender::rolling::never(&log_dir, &log_filename);
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(non_blocking_file))
        .with(tracing_subscriber::fmt::layer())
        .init();
    guard
}

fn load_fonts() {
    for (path, family, weight) in [
        ("fonts/ms_sans_serif.ttf", "Pixelated MS Sans Serif", 400),
        ("fonts/ms_sans_serif_bold.ttf", "Pixelated MS Sans Serif", 700),
        ("fonts/PerfectDOSVGA437Win.ttf", "Perfect DOS VGA 437 Win", 400),
    ] {
        if !ui::load_font(path, family, weight, false) {
            tracing::warn!("ui: failed to load font {path}");
        }
    }
    for (family, fallback) in [("Arial", true), ("Trebuchet MS", false)] {
        for (bold, weight) in [(false, 400), (true, 700)] {
            match crate::system::system_font(family, bold) {
                Some(path) => {
                    ui::load_font(&path.to_string_lossy(), family, weight, fallback && !bold);
                }
                None => tracing::warn!("ui: no system font for {family}"),
            }
        }
    }
    #[cfg(windows)]
    let symbols = Some(PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into())).join("Fonts\\seguisym.ttf"));
    #[cfg(target_os = "linux")]
    let symbols = crate::system::system_font("DejaVu Sans", false);
    if let Some(path) = symbols.filter(|p| p.is_file()) {
        ui::load_font(&path.to_string_lossy(), "Symbols", 400, true);
    }
}

struct Views {
    settings: Option<SettingsView>,
    update: Option<UpdateView>,
    post_update: Option<PostUpdateView>,
}

impl Views {
    fn any_open(&self) -> bool {
        self.settings.is_some() || self.update.is_some() || self.post_update.is_some()
    }

    fn toggle_settings(&mut self, app: &App, notray: bool) {
        match &self.settings {
            Some(v) if v.win.exists() => v.win.focus(),
            _ => {
                if ensure_ui(app) {
                    self.settings = SettingsView::open(app, notray);
                }
            }
        }
    }

    fn focus_settings(&self) {
        if let Some(v) = &self.settings {
            v.win.focus();
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.settings.as_ref().and_then(|v| v.next_deadline())
    }

    fn drop_closed(&mut self) {
        if self.settings.as_ref().is_some_and(|v| !v.win.exists()) {
            self.settings = None;
        }
        if self.update.as_ref().is_some_and(|v| !v.win.exists()) {
            self.update = None;
        }
        if self.post_update.as_ref().is_some_and(|v| !v.win.exists()) {
            self.post_update = None;
        }
    }
}

fn ensure_ui(app: &App) -> bool {
    if ui::is_active() {
        return true;
    }
    if !ui::init() {
        tracing::error!("failed to initialise the UI (GLFW/OpenGL)");
        return false;
    }
    load_fonts();
    ui::set_dark_theme(app.dark_theme());
    true
}

fn release_ui() {
    ui::shutdown();
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
        let _ = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

//app init-----------------------------------------------------------------------
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let post_update_version: Option<String> =
        args.iter().position(|a| a == "--post-update").and_then(|i| args.get(i + 1).cloned());
    let notray = args.iter().any(|a| a == "--notray" || a == "-notray");

    let data_dir = data_dir();
    let _log_guard = init_logging(&data_dir);

    let (tx, rx): (Sender<AppMsg>, Receiver<AppMsg>) = std::sync::mpsc::channel();
    {
        let tx = tx.clone();
        if !crate::single_instance::init(move |_argv, _cwd| {
            let _ = tx.send(AppMsg::FocusSettings);
            ui::wake();
        }) {
            return;
        }
    }

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
    apply_cpu_affinity(&cfg.cpu_affinity);

    let config = Arc::new(Mutex::new(cfg.clone()));
    let status = Arc::new(Mutex::new(ServerStatus::default()));
    let (rebind_tx, rebind_rx) = watch::channel(());
    let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel::<InputEvent>();
    let ws_state = Arc::new(WsState { rebind_tx, input_tx: input_tx.clone() });

    {
        let (cfg, st, tx) = (Arc::clone(&config), Arc::clone(&status), tx.clone());
        runtime().spawn(async move {
            ws_server::run(cfg, input_rx, st, rebind_rx, move |s| {
                let _ = tx.send(AppMsg::Status(s.clone()));
                ui::wake();
            })
            .await;
        });
    }

    #[cfg(windows)]
    let raw_input = Some(services::windows::raw_input::RawInputThread::start(
        input_tx.clone(),
        cfg.raw_mouse_min_delta,
        cfg.flush_hz,
        true,
    ));
    #[cfg(target_os = "linux")]
    let evdev = Some(services::linux::evdev_input::EvdevInputThread::start(
        input_tx.clone(),
        &cfg.linux_evdev_keyboard_device,
        &cfg.linux_raw_mouse_device,
        cfg.raw_mouse_min_delta,
        cfg.flush_hz,
    ));
    if !cfg.gamepad_device.is_empty() {
        services::gamepad::set_output(input_tx.clone(), &cfg.gamepad_device, cfg.flush_hz);
    }
    let analog = (!cfg.analog_keyboard.is_empty())
        .then(|| services::analog::AnalogThread::start(input_tx, &cfg.analog_keyboard, cfg.flush_hz));
    let http = if cfg.http_enabled {
        services::http_server::HttpServer::start(&cfg.host, cfg.http_port)
    } else {
        None
    };

    let mut app = App {
        config,
        config_path,
        status,
        ws_state,
        #[cfg(windows)]
        raw_input,
        #[cfg(target_os = "linux")]
        evdev,
        analog,
        http,
        update_info: None,
        tx: tx.clone(),
        quit: false,
    };

    let tray = (!notray).then(|| crate::tray::Tray::spawn(tx.clone()));

    let mut views = Views { settings: None, update: None, post_update: None };
    if notray {
        views.toggle_settings(&app, true);
        if views.settings.is_none() {
            return;
        }
    }
    if let Some(version) = post_update_version {
        if ensure_ui(&app) {
            views.post_update = PostUpdateView::open(&version);
        }
    }

    {
        let (dismissed, tx) = (cfg.dismissed_update_versions.clone(), tx.clone());
        runtime().spawn(async move {
            #[cfg(windows)]
            let check = services::windows::updater::check(VERSION, &dismissed).await;
            #[cfg(target_os = "linux")]
            let check = services::linux::updater::check(VERSION, &dismissed).await;
            if let Some(info) = check {
                let _ = tx.send(AppMsg::UpdateAvailable(info));
                ui::wake();
            }
        });
    }

    while !app.quit {
        let now = Instant::now();
        let wait = views
            .next_deadline()
            .map(|d| d.saturating_duration_since(now))
            .unwrap_or(Duration::from_secs(10))
            .min(Duration::from_secs(10));

        let mut msgs = Vec::new();
        if ui::is_active() {
            ui::frame(wait);
        } else if let Ok(msg) = rx.recv_timeout(wait) {
            msgs.push(msg);
        }
        msgs.extend(rx.try_iter());

        for msg in msgs {
            match msg {
                AppMsg::Status(s) => {
                    if let Some(v) = views.settings.as_mut() {
                        v.apply_status(&s);
                    }
                }
                AppMsg::UpdateAvailable(info) => {
                    app.update_info = Some(info.clone());
                    if let Some(v) = views.settings.as_mut() {
                        v.apply_update_info(&info);
                    }
                    if views.update.is_none() && ensure_ui(&app) {
                        views.update = UpdateView::open(&info);
                    }
                }
                AppMsg::UpdateProgress { percent, status } => {
                    if let Some(v) = views.update.as_mut() {
                        v.progress(percent, &status);
                    }
                    if let Some(v) = views.settings.as_mut() {
                        v.update_progress(percent, &status);
                    }
                }
                AppMsg::UpdateFailed(err) => {
                    if let Some(v) = views.update.as_mut() {
                        v.failed(&err);
                    }
                    if let Some(v) = views.settings.as_mut() {
                        v.update_failed(&err);
                    }
                }
                AppMsg::UpdateApplied => app.quit(),
                AppMsg::ToggleSettings => views.toggle_settings(&app, notray),
                AppMsg::FocusSettings => views.focus_settings(),
                AppMsg::Quit => app.quit(),
            }
        }

        if ui::is_active() {
            for ev in ui::poll_events() {
                if let Some(v) = views.settings.as_mut().filter(|v| v.win == ev.window) {
                    v.handle(&mut app, &ev);
                } else if let Some(v) = views.update.as_mut().filter(|v| v.win == ev.window) {
                    v.handle(&mut app, &ev);
                } else if let Some(v) = views.post_update.as_mut().filter(|v| v.win == ev.window) {
                    v.handle(&ev);
                }
            }
            if let Some(v) = views.settings.as_mut() {
                v.tick(Instant::now());
            }
            views.drop_closed();
            if notray && views.settings.is_none() {
                app.quit();
            }
            if !views.any_open() {
                release_ui();
            }
        }
    }

    ui::shutdown();
    crate::single_instance::destroy();
    drop(tray);
}
