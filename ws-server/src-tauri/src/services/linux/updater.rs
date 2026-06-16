#![cfg(target_os = "linux")]

use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use tauri::Emitter;

use crate::services::updater::{download_bytes, ProgressPayload, UpdateInfo};

const ASSET_APPIMAGE: &str = "input-overlay-ws-linux.AppImage";
const ASSET_BINARY: &str = "input-overlay-ws-linux-binary.zip";
const BINARY_NAME: &str = "input-overlay-ws";

fn running_as_appimage() -> bool {
    std::env::var("APPIMAGE").is_ok()
}

fn asset_name() -> &'static str {
    if running_as_appimage() {
        ASSET_APPIMAGE
    } else {
        ASSET_BINARY
    }
}

pub async fn check(current_version: &str, dismissed: &[String]) -> Option<UpdateInfo> {
    crate::services::updater::check(current_version, dismissed, asset_name()).await
}

pub async fn download_and_apply(download_url: &str, version: &str, app: &tauri::AppHandle) -> Result<(), String> {
    let emit = |pct: u32, msg: &str| {
        let _ = app.emit(
            "update-progress",
            ProgressPayload {
                percent: pct,
                status: msg.to_string(),
            },
        );
    };

    let bytes = download_bytes(download_url, &emit).await?;

    emit(72, "installing...");

    let target_path: PathBuf = if running_as_appimage() {
        std::env::var("APPIMAGE")
            .map(PathBuf::from)
            .map_err(|_| "APPIMAGE env var missing".to_string())?
    } else {
        std::env::current_exe().map_err(|e| e.to_string())?
    };

    let new_bytes = if running_as_appimage() {
        bytes
    } else {
        extract_binary_from_zip(&bytes)?
    };

    let tmp_path = target_path.with_extension("update_tmp");
    std::fs::write(&tmp_path, &new_bytes).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &target_path).map_err(|e| e.to_string())?;

    emit(90, "restarting...");

    std::process::Command::new(&target_path)
        .args(["--post-update", version])
        .spawn()
        .map_err(|e| e.to_string())?;

    emit(100, "done");
    Ok(())
}

fn extract_binary_from_zip(zip_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if std::path::Path::new(entry.name())
            .file_name()
            .and_then(|n| n.to_str())
            == Some(BINARY_NAME)
        {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            return Ok(buf);
        }
    }

    Err(format!("'{BINARY_NAME}' not found in zip"))
}
