use std::io::{Cursor, Read};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::OnceLock;
use std::time::Duration;

use tiny_skia::{Color, Pixmap};

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "ogv", "ogg", "mov", "mkv"];
const MAX_BYTES: u64 = 20_000_000;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const FETCH_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_REDIRECTS: u32 = 3;
const MAX_IMAGE_SIDE: u32 = 8192;
const MAX_DECODE_ALLOC: u64 = 256 * 1024 * 1024;

pub fn load(url: &str, tint: Color) -> Option<Pixmap> {
    let url = url.trim();
    if url.is_empty() || is_video(url) {
        if is_video(url) {
            tracing::info!("mouse_pad: video textures aren't supported natively yet, falling back to checkerboard");
        }
        return None;
    }

    let bytes = fetch(url)?;
    let img = decode(&bytes, url)?;
    rgba_to_tinted_pixmap(&img, tint)
}

pub(super) fn load_plain(url: &str) -> Option<Pixmap> {
    let url = url.trim();
    if url.is_empty() || is_video(url) {
        return None;
    }
    let bytes = fetch(url)?;
    let img = decode(&bytes, url)?;
    rgba_to_tinted_pixmap(&img, Color::WHITE)
}

pub(super) fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SIDE);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    limits
}

fn decode(bytes: &[u8], url: &str) -> Option<image::RgbaImage> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(decode_limits());
    reader
        .decode()
        .inspect_err(|e| tracing::warn!("image: failed to decode {url}: {e}"))
        .ok()
        .map(|img| img.to_rgba8())
}

pub(super) fn rgba_to_tinted_pixmap(img: &image::RgbaImage, tint: Color) -> Option<Pixmap> {
    let (w, h) = img.dimensions();
    let mut pixmap = Pixmap::new(w, h)?;

    let t = tint.to_color_u8();
    let factor = |channel: u8| 0.87 + 0.13 * (channel as f32 / 255.0);
    let (fr, fg, fb) = (factor(t.red()), factor(t.green()), factor(t.blue()));

    let dst = pixmap.data_mut();
    for (i, px) in img.pixels().enumerate() {
        let [r, g, b, a] = px.0;
        let af = a as f32 / 255.0;
        let idx = i * 4;
        dst[idx] = (r as f32 * fr * af) as u8;
        dst[idx + 1] = (g as f32 * fg * af) as u8;
        dst[idx + 2] = (b as f32 * fb * af) as u8;
        dst[idx + 3] = a;
    }

    Some(pixmap)
}

fn is_video(url: &str) -> bool {
    has_extension(url, VIDEO_EXTENSIONS)
}

pub(super) fn is_gif(url: &str) -> bool {
    has_extension(url, &["gif"])
}

fn has_extension(url: &str, exts: &[&str]) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_lowercase();
    exts.iter().any(|ext| path.ends_with(&format!(".{ext}")))
}

pub(super) fn fetch(url: &str) -> Option<Vec<u8>> {
    let lower = url.to_ascii_lowercase();
    
    if lower.starts_with("https://") {
        return fetch_remote(url);
    }

    if lower.starts_with("http://") {
        tracing::warn!("image: refusing insecure http:// url {url} - use https://");
        return None;
    }

    if is_network_path(url) {
        tracing::warn!("image: refusing network path {url} - only local files or https:// urls");
        return None;
    }

    std::fs::read(url).ok()
}

pub(crate) fn is_network_path(path: &str) -> bool {
    let p = path.trim();
    p.starts_with("\\\\") || p.starts_with("//") || p.starts_with("\\/") || p.starts_with("/\\")
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .https_only(true)
            .redirects(MAX_REDIRECTS)
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout(FETCH_TIMEOUT)
            .resolver(public_only_resolve)
            .user_agent(concat!("input-overlay-obs-plugin/", env!("CARGO_PKG_VERSION")))
            .build()
    })
}

fn public_only_resolve(netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
    let addrs: Vec<SocketAddr> = netloc.to_socket_addrs()?.collect();
    let public: Vec<SocketAddr> = addrs.iter().copied().filter(|a| is_public_ip(a.ip())).collect();
    if public.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("{netloc} resolves only to non-public addresses"),
        ));
    }
    Ok(public)
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, c, _] = v4.octets();
            !(v4.is_unspecified()
                || v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_multicast()
                || a == 0
                || (a == 100 && (64..128).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 198 && (18..20).contains(&b))
                || a >= 240) //reserved
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_unspecified()
                || v6.is_loopback()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
                || first == 0x2001 && v6.segments()[1] == 0x0db8)
        }
    }
}

fn fetch_remote(url: &str) -> Option<Vec<u8>> {
    let response = agent()
        .get(url)
        .call()
        .inspect_err(|e| tracing::warn!("image: request for {url} failed: {e}"))
        .ok()?;
    let declared: Option<u64> = response.header("Content-Length").and_then(|v| v.parse().ok());
    
    if declared.is_some_and(|len| len > MAX_BYTES) {
        tracing::warn!("image: {url} is larger than {MAX_BYTES} bytes, skipped");
        return None;
    }

    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_BYTES {
        tracing::warn!("image: {url} is larger than {MAX_BYTES} bytes, skipped");
        return None;
    }
    Some(bytes)
}
