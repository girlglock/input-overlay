use std::time::{Duration, Instant};

use crate::app::{App, VERSION};
use crate::services::config::Config;
use crate::services::updater::UpdateInfo;
use crate::system::{self, open_url};
use crate::ui::{self, escape, Event, EventKind, Win};
use crate::ws_server::ServerStatus;

const TABS: [&str; 4] = ["server", "input", "advanced", "about"];
const LINKS: [(&str, &str); 4] = [
    ("link-github", "https://github.com/girlglock/input-overlay"),
    ("link-twitter", "https://twitter.com/girlglock_"),
    ("link-website", "https://girlglock.com"),
    ("link-analogsense", "https://github.com/AnalogSense/JavaScript-SDK"),
];
const TOKEN_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub struct SettingsView {
    pub win: Win,
    original: Config,
    whitelist: Vec<String>,
    dirty: bool,
    listening: bool,
    token_visible: bool,
    autostart: bool,
    http_enabled: bool,
    clients: Vec<String>,
    clients_text: String,
    notice_until: Option<Instant>,
    msg_until: Option<Instant>,
    update_info: Option<UpdateInfo>,
    updating: bool,
    keep_open_on_save: bool,
}

impl SettingsView {
    pub fn open(app: &App, keep_open_on_save: bool) -> Option<Self> {
        let win = Win::open("Input Overlay WS - Settings", 420, 580, true, "settings.rml")?;
        let cfg = app.config();
        let mut view = Self {
            win,
            original: cfg.clone(),
            whitelist: Vec::new(),
            dirty: false,
            listening: false,
            token_visible: false,
            autostart: autostart_enabled(),
            http_enabled: cfg.http_enabled,
            clients: Vec::new(),
            clients_text: String::new(),
            notice_until: None,
            msg_until: None,
            update_info: None,
            updating: false,
            keep_open_on_save,
        };

        win.set_inner_rml("version-label", &format!("Input-Overlay WebSocket Server | Version: {VERSION}<br/>(latest)"));
        win.set_text("status-version", &format!("v{VERSION}"));
        view.apply_theme_label(app.dark_theme());
        view.apply_config(&cfg);
        view.apply_status(&app.server_status());
        win.set_checked("autostart", view.autostart);

        #[cfg(windows)]
        win.set_hidden("cpu-affinity-row", false);
        if !system::is_admin() {
            win.set_hidden("admin-warning", false);
            win.set_disabled("autostart", true);
        }
        #[cfg(target_os = "linux")]
        view.init_linux_devices(&cfg);

        if let Some(info) = &app.update_info {
            view.apply_update_info(info);
        }
        Some(view)
    }

    fn apply_config(&mut self, cfg: &Config) {
        let w = self.win;
        self.original = cfg.clone();
        w.set_value("host", &cfg.host);
        w.set_value("port", &cfg.port.to_string());
        w.set_value("auth-token", &cfg.auth_token);
        w.set_value("auth-token-shown", &cfg.auth_token);
        w.set_checked("send-mouse-move", cfg.send_mouse_move);
        w.set_value("flush-hz", &cfg.flush_hz.to_string());
        w.set_value(
            "cpu-affinity",
            &cfg.cpu_affinity.iter().map(u32::to_string).collect::<Vec<_>>().join(", "),
        );
        w.set_value("analog-keyboard", &cfg.analog_keyboard);
        w.set_checked("http-enabled", cfg.http_enabled);
        w.set_value("http-host", &cfg.host);
        w.set_value("http-port", &cfg.http_port.to_string());
        self.http_enabled = cfg.http_enabled;
        self.apply_http_enabled(cfg.http_enabled);
        w.set_value("kbd-device", &cfg.linux_evdev_keyboard_device);
        w.set_value("mouse-device", &cfg.linux_raw_mouse_device);
        self.whitelist = cfg.key_whitelist.clone();
        self.render_key_list();
        self.set_dirty(false);
    }

    fn read_config(&self, app: &App) -> Config {
        let w = self.win;
        let host = w.value("host").trim().to_string();
        let cpu = w.value("cpu-affinity");
        Config {
            host: if host.is_empty() { "localhost".into() } else { host },
            port: w.value("port").trim().parse().unwrap_or(4455),
            auth_token: w.value(self.token_field()).trim().to_string(),
            send_mouse_move: w.checked("send-mouse-move"),
            key_whitelist: self.whitelist.clone(),
            analog_keyboard: w.value("analog-keyboard"),
            http_enabled: w.checked("http-enabled"),
            http_port: w.value("http-port").trim().parse().unwrap_or(4456),
            flush_hz: w.value("flush-hz").trim().parse().unwrap_or(125),
            cpu_affinity: cpu.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
            raw_mouse_min_delta: self.original.raw_mouse_min_delta,
            #[cfg(target_os = "linux")]
            linux_evdev_keyboard_device: w.value("kbd-device"),
            #[cfg(not(target_os = "linux"))]
            linux_evdev_keyboard_device: self.original.linux_evdev_keyboard_device.clone(),
            #[cfg(target_os = "linux")]
            linux_raw_mouse_device: w.value("mouse-device"),
            #[cfg(not(target_os = "linux"))]
            linux_raw_mouse_device: self.original.linux_raw_mouse_device.clone(),
            dismissed_update_versions: self.original.dismissed_update_versions.clone(),
            theme: Some(if app.dark_theme() { "dark" } else { "light" }.into()),
        }
    }

    fn refresh_dirty(&mut self, app: &App) {
        let mut current = self.read_config(app);
        current.theme = self.original.theme.clone();
        let mut original = self.original.clone();
        original.host = if original.host.trim().is_empty() { "localhost".into() } else { original.host.trim().into() };
        let dirty = !configs_equal(&current, &original);
        self.set_dirty(dirty);
    }

    fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
        let w = self.win;
        w.set_class("save-btn", "dirty", dirty);
        w.set_text("save-btn", if dirty { "\u{25CF} Save" } else { "Save" });
        if self.msg_until.is_none() {
            self.show_dirty_msg();
        }
    }

    fn show_dirty_msg(&self) {
        let w = self.win;
        w.remove_property("save-msg", "color");
        w.set_class("save-msg", "dirty-msg", self.dirty);
        w.set_text("save-msg", if self.dirty { "you have unsaved changes pending" } else { "" });
    }

    fn show_msg(&mut self, text: &str, is_error: bool) {
        let w = self.win;
        w.set_class("save-msg", "dirty-msg", false);
        w.set_text("save-msg", text);
        w.set_property("save-msg", "color", if is_error { "#bf5e5e" } else { "#003c74" });
        self.msg_until = Some(Instant::now() + Duration::from_secs(3));
    }

    fn notice(&mut self, text: &str) {
        self.win.set_text("clients-count", text);
        self.notice_until = Some(Instant::now() + Duration::from_secs(5));
    }

    pub fn apply_status(&mut self, s: &ServerStatus) {
        let w = self.win;
        if s.running {
            w.set_hidden("port-error", true);
        } else {
            let text = match s.bind_error.as_deref() {
                Some("inuse") => format!("Port {} is already in use - try a different port", s.port),
                Some("denied") => format!("Access to {}:{} was denied", s.host, s.port),
                _ => format!("Could not bind to {}:{}", s.host, s.port),
            };
            w.set_text("port-error", &text);
            w.set_hidden("port-error", false);
        }

        self.clients = s.clients.clone();
        let n = self.clients.len();
        self.clients_text = match n {
            0 => "no clients connected".into(),
            1 => "1 client connected".into(),
            _ => format!("{n} clients connected"),
        };
        w.set_class("clients-count", "has-clients", n > 0);
        let items: String = self.clients.iter().map(|a| format!("<li>{}</li>", escape(a))).collect();
        w.set_inner_rml("clients-list", &items);
        if self.notice_until.is_none() {
            w.set_text("clients-count", &self.clients_text);
        }
    }

    fn render_key_list(&self) {
        let rml = if self.whitelist.is_empty() {
            "<span class=\"key-empty\">all keys allowed</span>".to_string()
        } else {
            self.whitelist
                .iter()
                .enumerate()
                .map(|(i, k)| format!("<button class=\"key-tag\" id=\"key-tag-{i}\">{}</button>", escape(&k.to_uppercase())))
                .collect()
        };
        self.win.set_inner_rml("key-list", &rml);
    }

    fn add_to_whitelist(&mut self, app: &App, name: &str) {
        if !self.whitelist.iter().any(|k| k == name) {
            self.whitelist.push(name.to_string());
            self.render_key_list();
            self.refresh_dirty(app);
        }
    }

    fn set_listening(&mut self, listening: bool) {
        self.listening = listening;
        self.win.set_text("add-key-btn", if listening { "LISTENING..." } else { "ADD KEY" });
        self.win.set_class("add-key-btn", "listening", listening);
        if listening {
            self.win.blur();
        }
    }

    fn token_field(&self) -> &'static str {
        if self.token_visible { "auth-token-shown" } else { "auth-token" }
    }

    fn apply_http_enabled(&self, enabled: bool) {
        for id in ["http-host", "http-port", "open-http-btn"] {
            self.win.set_disabled(id, !enabled);
        }
    }

    fn apply_theme_label(&self, dark: bool) {
        self.win.set_text("theme-toggle", if dark { "dark" } else { "light" });
        self.win.set_class("theme-toggle", "active", dark);
    }

    fn select_tab(&self, tab: &str) {
        for t in TABS {
            self.win.set_attr(&format!("tab-btn-{t}"), "aria-selected", if t == tab { "true" } else { "false" });
            self.win.set_hidden(&format!("tab-{t}"), t != tab);
        }
    }

    fn apply_bind_if_valid(&mut self, app: &mut App, notice: String) {
        let host = self.win.value("host").trim().to_string();
        let host = if host.is_empty() { "localhost".to_string() } else { host };
        if let Ok(port) = self.win.value("port").trim().parse::<u16>() {
            if port >= 1 {
                self.notice(&notice);
                app.apply_bind(host, port);
            }
        }
    }

    fn request_close(&mut self) {
        if self.dirty {
            self.win.set_hidden("discard-modal", false);
        } else {
            self.win.close();
        }
    }

    fn regenerate_token(&mut self, app: &App) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let token: String = (0..32).map(|_| TOKEN_CHARS[rng.gen_range(0..TOKEN_CHARS.len())] as char).collect();
        self.win.set_value("auth-token", &token);
        self.win.set_value("auth-token-shown", &token);
        self.refresh_dirty(app);
    }

    pub fn apply_update_info(&mut self, info: &UpdateInfo) {
        if self.update_info.as_ref().is_some_and(|i| i.version == info.version) {
            return;
        }
        self.update_info = Some(info.clone());
        let w = self.win;
        w.set_text("update-text", &format!("update available: v{}", info.version));
        w.set_hidden("update-bar", false);
        w.set_inner_rml(
            "version-label",
            &format!(
                "Input-Overlay WebSocket Server | Version: {VERSION}<br/><a class=\"accent-link\" id=\"update-version-link\">\u{2192} v{} available</a>",
                escape(&info.version)
            ),
        );
    }

    pub fn update_progress(&mut self, percent: u32, status: &str) {
        if self.updating {
            self.win.set_attr("update-progress-fill", "value", &percent.to_string());
            self.win.set_text("update-progress-status", status);
        }
    }

    pub fn update_failed(&mut self, err: &str) {
        if self.updating {
            self.updating = false;
            self.win.set_hidden("update-progress-modal", true);
            self.show_msg(&format!("update failed: {err}"), true);
        }
    }

    #[cfg(target_os = "linux")]
    fn init_linux_devices(&self, cfg: &Config) {
        use crate::services::linux::evdev_input;
        let w = self.win;
        w.set_text("autostart-label", "Start on login");
        w.set_hidden("linux-devices", false);
        for (id, devices, current) in [
            ("kbd-device", evdev_input::enum_keyboards(), &cfg.linux_evdev_keyboard_device),
            ("mouse-device", evdev_input::enum_mice(), &cfg.linux_raw_mouse_device),
        ] {
            for (path, name) in devices {
                w.select_add(id, &name, &path);
            }
            w.set_value(id, current);
        }
        let perms = evdev_input::check_permissions();
        if !perms.is_empty() {
            let rml: String = perms
                .iter()
                .map(|m| {
                    format!(
                        "<span class=\"warn-line\"><span class=\"warn-icon\">\u{26A0}</span><span class=\"warn-text\">{}</span></span>",
                        escape(m)
                    )
                })
                .collect();
            w.set_inner_rml("linux-perm-warning", &rml);
            w.set_hidden("linux-perm-warning", false);
        }
    }

    pub fn handle(&mut self, app: &mut App, ev: &Event) {
        if self.listening {
            match ev.kind {
                EventKind::KeyDown(key) => {
                    if let Some(name) = system::key_name(key) {
                        self.add_to_whitelist(app, name);
                    }
                    self.set_listening(false);
                    return;
                }
                EventKind::MouseDown(button) if ev.id != "add-key-btn" => {
                    if let Some(name) = system::mouse_button_name(button) {
                        self.add_to_whitelist(app, name);
                    }
                    self.set_listening(false);
                    return;
                }
                EventKind::Wheel => {
                    self.add_to_whitelist(app, "mouse_wheel");
                    self.set_listening(false);
                    return;
                }
                EventKind::Click if ev.id != "add-key-btn" => return,
                _ => {}
            }
        }

        match ev.kind {
            EventKind::Click => self.on_click(app, &ev.id),
            EventKind::Change => self.on_change(app, &ev.id),
            EventKind::Blur => self.on_blur(app, &ev.id),
            EventKind::Close => self.request_close(),
            _ => {}
        }
    }

    fn on_click(&mut self, app: &mut App, id: &str) {
        if let Some(tab) = id.strip_prefix("tab-btn-") {
            self.select_tab(tab);
            return;
        }
        if let Some(idx) = id.strip_prefix("key-tag-").and_then(|i| i.parse::<usize>().ok()) {
            if idx < self.whitelist.len() {
                self.whitelist.remove(idx);
                self.render_key_list();
                self.refresh_dirty(app);
            }
            return;
        }
        if let Some((_, url)) = LINKS.iter().find(|(link, _)| *link == id) {
            open_url(url);
            return;
        }
        match id {
            "min-btn" => self.win.minimize(),
            "close-title-btn" | "cancel-btn" => self.request_close(),
            "clients-count" if !self.clients.is_empty() => self.win.set_hidden("clients-modal", false),
            "clients-modal-close" | "clients-modal-ok" => self.win.set_hidden("clients-modal", true),
            "theme-toggle" => {
                let dark = !app.dark_theme();
                let _ = app.set_theme(dark);
                self.apply_theme_label(dark);
            }
            "show-token" => {
                let token = self.win.value(self.token_field());
                self.token_visible = !self.token_visible;
                self.win.set_value(self.token_field(), &token);
                self.win.set_hidden("auth-token", self.token_visible);
                self.win.set_hidden("auth-token-shown", !self.token_visible);
            }
            "copy-token" => {
                ui::set_clipboard(&self.win.value(self.token_field()));
                self.show_msg("token copied", false);
            }
            "regen-token" => self.regenerate_token(app),
            "add-key-btn" => self.set_listening(!self.listening),
            "open-http-btn" if self.http_enabled => {
                let host = self.win.value("http-host");
                let host = if host.is_empty() { "localhost".to_string() } else { host };
                let port = self.win.value("http-port").trim().parse::<u16>().unwrap_or(4456);
                open_url(&format!("http://{host}:{port}"));
            }
            "save-btn" => match app.save_config(self.read_config(app)) {
                Ok(()) => {
                    self.original = app.config();
                    self.set_dirty(false);
                    if self.keep_open_on_save {
                        self.show_msg("settings saved", false);
                    } else {
                        self.win.close();
                    }
                }
                Err(e) => self.show_msg(&e, true),
            },
            "modal-keep-btn" => self.win.set_hidden("discard-modal", true),
            "modal-discard-btn" => {
                self.win.set_hidden("discard-modal", true);
                let original = self.original.clone();
                app.apply_bind(original.host.clone(), original.port);
                self.apply_config(&original);
                self.win.close();
            }
            "update-now-btn" => {
                if let Some(info) = self.update_info.clone() {
                    self.updating = true;
                    self.win.set_hidden("update-progress-modal", false);
                    self.win.set_text("update-progress-status", "starting...");
                    self.win.set_attr("update-progress-fill", "value", "0");
                    app.apply_update(&info);
                }
            }
            "update-later-btn" => {
                self.win.set_hidden("update-bar", true);
                self.update_info = None;
            }
            "update-notes-link" | "update-version-link" => {
                if let Some(info) = &app.update_info {
                    open_url(&info.release_url);
                }
            }
            _ => {}
        }
    }

    fn on_change(&mut self, app: &mut App, id: &str) {
        match id {
            "autostart" => {
                let enabled = self.win.checked("autostart");
                if enabled != self.autostart {
                    self.autostart = enabled;
                    set_autostart(enabled, self.keep_open_on_save);
                }
            }
            "http-enabled" => {
                let enabled = self.win.checked("http-enabled");
                if enabled != self.http_enabled {
                    self.http_enabled = enabled;
                    self.apply_http_enabled(enabled);
                    let host = self.win.value("host").trim().to_string();
                    let host = if host.is_empty() { "localhost".to_string() } else { host };
                    let port = self.win.value("http-port").trim().parse().unwrap_or(4456);
                    if let Err(e) = app.toggle_http(enabled, &host, port) {
                        self.show_msg(&e, true);
                    }
                    self.original.http_enabled = enabled;
                    self.original.http_port = port;
                }
            }
            "host" => {
                let host = self.win.value("host");
                self.win.set_value("http-host", &host);
                self.refresh_dirty(app);
            }
            _ => self.refresh_dirty(app),
        }
    }

    fn on_blur(&mut self, app: &mut App, id: &str) {
        match id {
            "host" => {
                let host = self.win.value("host").trim().to_string();
                let host = if host.is_empty() { "localhost".to_string() } else { host };
                self.apply_bind_if_valid(app, format!("host changed to: {host}"));
            }
            "port" => {
                let port = self.win.value("port");
                self.apply_bind_if_valid(app, format!("port changed to: {port}"));
            }
            "http-port" => {
                if let Ok(port) = self.win.value("http-port").trim().parse::<u16>() {
                    if port >= 1 {
                        self.notice(&format!("http port changed to: {port}"));
                        let host = self.win.value("host").trim().to_string();
                        let host = if host.is_empty() { "localhost".to_string() } else { host };
                        let _ = app.toggle_http(self.http_enabled, &host, port);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        [self.notice_until, self.msg_until].into_iter().flatten().min()
    }

    pub fn tick(&mut self, now: Instant) {
        if self.notice_until.is_some_and(|t| now >= t) {
            self.notice_until = None;
            self.win.set_text("clients-count", &self.clients_text);
        }
        if self.msg_until.is_some_and(|t| now >= t) {
            self.msg_until = None;
            self.show_dirty_msg();
        }
    }
}

fn configs_equal(a: &Config, b: &Config) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}

fn autostart_enabled() -> bool {
    #[cfg(target_os = "linux")]
    {
        crate::services::linux::autostart::is_enabled()
    }
    #[cfg(windows)]
    {
        crate::services::windows::autostart::is_enabled()
    }
}

//notray: started with --notray, so start on login the same way
fn set_autostart(enabled: bool, notray: bool) -> bool {
    let args: &[&str] = if notray { &["--notray"] } else { &[] };
    #[cfg(target_os = "linux")]
    {
        //inside an appimage current_exe is the temporary mount, autostart the appimage itself
        let exe = crate::services::linux::updater::appimage_path()
            .or_else(|| std::env::current_exe().ok())
            .unwrap_or_default();
        crate::services::linux::autostart::set_enabled(enabled, &exe, args)
    }
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().unwrap_or_default();
        crate::services::windows::autostart::set_enabled(enabled, &exe, args)
    }
}
