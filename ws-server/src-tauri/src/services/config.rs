use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub auth_token: String,
    pub key_whitelist: Vec<String>,
    pub raw_mouse_min_delta: i32,
    pub send_mouse_move: bool,
    pub cpu_affinity: Vec<u32>,
    pub analog_keyboard: String,
    pub http_enabled: bool,
    pub http_port: u16,
    pub linux_evdev_keyboard_device: String,
    pub linux_raw_mouse_device: String,
    pub dismissed_update_versions: Vec<String>,
    pub theme: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 4455,
            auth_token: String::new(),
            key_whitelist: Vec::new(),
            raw_mouse_min_delta: 0,
            send_mouse_move: true,
            cpu_affinity: vec![0, 1],
            analog_keyboard: String::new(),
            http_enabled: false,
            http_port: 4456,
            linux_evdev_keyboard_device: String::new(),
            linux_raw_mouse_device: String::new(),
            dismissed_update_versions: Vec::new(),
            theme: None,
        }
    }
}

pub fn load(path: &Path) -> Config {
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(s) => match serde_json::from_str::<Config>(&s) {
                Ok(cfg) => return cfg,
                Err(e) => tracing::error!("config parse error: {e}"),
            },
            Err(e) => tracing::error!("config read error: {e}"),
        }
    }
    Config::default()
}

pub fn save(path: &Path, config: &Config) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(config).expect("config serialize failed");
    std::fs::write(path, json)
}
