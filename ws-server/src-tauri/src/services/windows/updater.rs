#![cfg(windows)]

use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::Path;

use tauri::Emitter;

use crate::services::updater::{download_bytes, ProgressPayload, UpdateInfo};

const ASSET_NAME: &str = "input-overlay-ws-windows.zip";
const EXE_NAME: &str = "input-overlay-ws.exe";

pub async fn check(current_version: &str, dismissed: &[String]) -> Option<UpdateInfo> {
    crate::services::updater::check(current_version, dismissed, ASSET_NAME).await
}

pub async fn download_and_apply(
    download_url: &str,
    version: &str,
    current_exe: &Path,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    let emit = |pct: u32, msg: &str| {
        let _ = app.emit(
            "update-progress",
            ProgressPayload {
                percent: pct,
                status: msg.to_string(),
            },
        );
    };

    let zip_bytes = download_bytes(download_url, &emit).await?;

    emit(72, "extracting...");
    let new_exe = extract_exe_from_zip(&zip_bytes)?;

    emit(90, "scheduling restart...");
    schedule_replace(current_exe, &new_exe, version)?;

    emit(100, "restarting...");
    Ok(())
}

fn extract_exe_from_zip(zip_bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    let tmp_dir = std::env::temp_dir().join(format!("iov_update_{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let out_path = tmp_dir.join(EXE_NAME);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if Path::new(entry.name()).file_name().and_then(|n| n.to_str()) == Some(EXE_NAME) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            std::fs::write(&out_path, buf).map_err(|e| e.to_string())?;
            return Ok(out_path);
        }
    }

    Err(format!("'{EXE_NAME}' not found in zip"))
}

fn schedule_replace(current_exe: &Path, new_exe: &Path, version: &str) -> Result<(), String> {
    let ps = |p: &Path| p.to_string_lossy().replace('\\', "/");

    let old_exe = current_exe.with_extension("old");
    let tmp_ps1 = std::env::temp_dir().join(format!("iov_upd_{}.ps1", std::process::id()));

    let script = format!(
        "$ErrorActionPreference = 'Stop'\n\
         trap {{ \
           Add-Type -AssemblyName System.Windows.Forms; \
           [System.Windows.Forms.MessageBox]::Show(\"Update failed:`n$_\",\"Update Error\",0,16); \
           exit 1 \
         }}\n\
         Start-Sleep -Milliseconds 1200\n\
         $cur = \"{cur}\"\n\
         $new = \"{new}\"\n\
         $old = \"{old}\"\n\
         if (Test-Path $old) {{ Remove-Item $old -Force }}\n\
         Rename-Item -Path $cur -NewName $old -Force\n\
         Copy-Item -Path $new -Destination $cur -Force\n\
         Remove-Item $old -Force -ErrorAction SilentlyContinue\n\
         Remove-Item $new -Force -ErrorAction SilentlyContinue\n\
         Start-Process -FilePath $cur -ArgumentList \"--post-update\",\"{ver}\"\n",
        cur = ps(current_exe),
        new = ps(new_exe),
        old = ps(&old_exe),
        ver = version,
    );

    std::fs::write(&tmp_ps1, &script).map_err(|e| e.to_string())?;

    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            tmp_ps1.to_str().unwrap_or(""),
        ])
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}
