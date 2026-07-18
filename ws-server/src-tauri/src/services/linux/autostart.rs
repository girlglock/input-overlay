pub fn is_enabled() -> bool {
    autostart_path().map_or(false, |p| p.exists())
}

pub fn set_enabled(enabled: bool, exe_path: &std::path::Path) -> bool {
    let Some(path) = autostart_path() else {
        return false;
    };
    if enabled {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let exe = exe_path.to_string_lossy();
        let work = exe_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Input Overlay WS\nExec={exe}\nPath={work}\nX-GNOME-Autostart-enabled=true\n"
        );
        std::fs::write(&path, content).is_ok()
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => true,
            Err(_) if !path.exists() => true,
            Err(e) => {
                tracing::warn!("autostart remove failed: {e}");
                false
            }
        }
    }
}

fn autostart_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".config/autostart/input-overlay-ws.desktop"))
}
