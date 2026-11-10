#![cfg(target_os = "linux")]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::services::updater::{download_bytes, UpdateInfo};

const ASSET_NAME: &str = "input-overlay-ws.AppImage";

pub fn appimage_path() -> Option<PathBuf> {
    std::env::var("APPIMAGE").ok().map(PathBuf::from)
}

pub async fn check(current_version: &str, dismissed: &[String]) -> Option<UpdateInfo> {
    appimage_path()?;
    crate::services::updater::check(current_version, dismissed, ASSET_NAME).await
}

pub async fn download_and_apply(
    download_url: &str,
    version: &str,
    emit: &(dyn Fn(u32, &str) + Send + Sync),
) -> Result<(), String> {

    let target_path = appimage_path().ok_or("updates are only supported for the appimage")?;

    let bytes = download_bytes(download_url, &emit).await?;

    emit(72, "installing...");

    let tmp_path = target_path.with_extension("update_tmp");
    std::fs::write(&tmp_path, &bytes).map_err(|e| e.to_string())?;
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
