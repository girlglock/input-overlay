#![cfg(windows)]

use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::Path;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/girlglock/input-overlay/releases/latest";
const ASSET_NAME: &str = "input-overlay-ws-windows.zip";
const EXE_NAME: &str = "input-overlay-ws.exe";

#[derive(Debug, Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub release_url: String,
    pub download_url: String,
    pub body: String,
}

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub percent: u32,
    pub status: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

fn parse_ver(v: &str) -> (u32, u32, u32) {
    let s = v.trim_start_matches('v');
    let mut p = s.splitn(3, '.');
    let n = |x: Option<&str>| x.and_then(|s| s.parse().ok()).unwrap_or(0u32);
    (n(p.next()), n(p.next()), n(p.next()))
}

pub async fn check(current_version: &str, dismissed: &[String]) -> Option<UpdateInfo> {
    let client = reqwest::Client::builder()
        .user_agent("input-overlay-ws")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let release: GithubRelease = client
        .get(GITHUB_API_URL)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;

    let version = release.tag_name.trim_start_matches('v').to_string();

    if parse_ver(&version) <= parse_ver(current_version) {
        return None;
    }
    if dismissed.iter().any(|d| d == &version) {
        return None;
    }

    let download_url = release
        .assets
        .iter()
        .find(|a| a.name == ASSET_NAME)?
        .browser_download_url
        .clone();

    Some(UpdateInfo {
        version,
        release_url: release.html_url,
        download_url,
        body: release.body.unwrap_or_default(),
    })
}

pub async fn download_and_schedule(
    download_url: &str,
    current_exe: &Path,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    let emit = |pct: u32, msg: &str| {
        let _ = app.emit(
            "update-progress",
            ProgressPayload { percent: pct, status: msg.to_string() },
        );
    };

    emit(5, "connecting...");

    let client = reqwest::Client::builder()
        .user_agent("input-overlay-ws")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut zip_bytes = Vec::with_capacity(total as usize);

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        zip_bytes.extend_from_slice(&chunk);
        if total > 0 {
            let pct = 5 + (downloaded * 65 / total) as u32;
            emit(pct, &format!("downloading... {}kb / {}kb", downloaded / 1024, total / 1024));
        }
    }

    emit(72, "extracting...");
    let new_exe = extract_exe_from_zip(&zip_bytes)?;

    emit(90, "scheduling update...");
    schedule_replace(current_exe, &new_exe)?;

    emit(100, "restarting...");
    Ok(())
}

fn extract_exe_from_zip(zip_bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    let tmp_dir = std::env::temp_dir()
        .join(format!("iov_update_{}", std::process::id()));
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

fn schedule_replace(current_exe: &Path, new_exe: &Path) -> Result<(), String> {
    let ps = |p: &Path| p.to_string_lossy().replace('\\', "/");

    let old_exe  = current_exe.with_extension("old");
    let tmp_ps1  = std::env::temp_dir()
        .join(format!("iov_upd_{}.ps1", std::process::id()));

    let script = format!(
        "$ErrorActionPreference = 'Stop'\n\
         trap {{ Add-Type -AssemblyName System.Windows.Forms; \
         [System.Windows.Forms.MessageBox]::Show(\"Update failed:`n$_\",\"Error\",0,16); exit 1 }}\n\
         Add-Type -AssemblyName System.Windows.Forms\n\
         Start-Sleep -Milliseconds 1200\n\
         $cur = \"{cur}\"\n\
         $new = \"{new}\"\n\
         $old = \"{old}\"\n\
         if (Test-Path $old) {{ Remove-Item $old -Force }}\n\
         Rename-Item -Path $cur -NewName $old -Force\n\
         Copy-Item -Path $new -Destination $cur -Force\n\
         Remove-Item $old -Force -ErrorAction SilentlyContinue\n\
         Remove-Item $new -Force -ErrorAction SilentlyContinue\n\
         [System.Windows.Forms.MessageBox]::Show(\
         \"Update finished! Reopen input-overlay-ws.exe\",\
         \"Update Finished\",0,64)\n",
        cur = ps(current_exe),
        new = ps(new_exe),
        old = ps(&old_exe),
    );

    std::fs::write(&tmp_ps1, &script).map_err(|e| e.to_string())?;

    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
               tmp_ps1.to_str().unwrap_or("")])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}
