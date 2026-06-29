use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

const GITHUB_API_URL: &str = "https://api.github.com/repos/girlglock/input-overlay/releases/latest";

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

pub fn parse_ver(v: &str) -> (u32, u32, u32) {
    let s = v.trim_start_matches('v');
    let mut p = s.splitn(3, '.');
    let n = |x: Option<&str>| x.and_then(|s| s.parse().ok()).unwrap_or(0u32);
    (n(p.next()), n(p.next()), n(p.next()))
}

pub async fn check(
    current_version: &str,
    dismissed: &[String],
    asset_name: &str,
) -> Option<UpdateInfo> {
    let client = reqwest::Client::builder()
        .user_agent("input-overlay-ws")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let release: GithubRelease = client
        .get(GITHUB_API_URL)
        .send()
        .await
        .inspect_err(|e| tracing::warn!("update check: request failed.. {e}"))
        .ok()?
        .error_for_status()
        .inspect_err(|e| tracing::warn!("update check: bad status.. {e}"))
        .ok()?
        .json()
        .await
        .inspect_err(|e| tracing::warn!("update check: bad response.. {e}"))
        .ok()?;

    let version = release.tag_name.trim_start_matches('v').to_string();

    if parse_ver(&version) <= parse_ver(current_version) {
        return None;
    }
    if dismissed.iter().any(|d| d == &version) {
        return None;
    }

    let Some(asset) = release.assets.iter().find(|a| a.name == asset_name) else {
        tracing::warn!("update check: asset '{asset_name}' not in release");
        return None;
    };
    let download_url = asset.browser_download_url.clone();

    Some(UpdateInfo {
        version,
        release_url: release.html_url,
        download_url,
        body: release.body.unwrap_or_default(),
    })
}

pub async fn download_bytes<F: Fn(u32, &str)>(url: &str, emit: F) -> Result<Vec<u8>, String> {
    emit(5, "connecting...");

    let client = reqwest::Client::builder()
        .user_agent("input-overlay-ws")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut bytes = Vec::with_capacity(total as usize);

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        if let Some(div) = (downloaded * 65).checked_div(total) {
            emit(
                5 + div as u32,
                &format!(
                    "downloading... {}kb / {}kb",
                    downloaded / 1024,
                    total / 1024
                ),
            );
        }
    }

    Ok(bytes)
}
