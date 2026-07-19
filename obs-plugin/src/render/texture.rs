use std::io::Read;
use std::time::Duration;

use tiny_skia::{Color, Pixmap};

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "ogv", "ogg", "mov", "mkv"];
const MAX_BYTES: u64 = 20_000_000;
const FETCH_TIMEOUT: Duration = Duration::from_secs(8);

pub fn load(url: &str, tint: Color) -> Option<Pixmap> {
    let url = url.trim();
    if url.is_empty() || is_video(url) {
        if is_video(url) {
            tracing::info!("mouse_pad: video textures aren't supported natively yet, falling back to checkerboard");
        }
        return None;
    }

    let bytes = fetch(url)?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    rgba_to_tinted_pixmap(&img, tint)
}

pub(super) fn load_plain(url: &str) -> Option<Pixmap> {
    let url = url.trim();
    if url.is_empty() || is_video(url) {
        return None;
    }
    let bytes = fetch(url)?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    rgba_to_tinted_pixmap(&img, Color::WHITE)
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
    if url.starts_with("http://") || url.starts_with("https://") {
        let response = ureq::get(url).timeout(FETCH_TIMEOUT).call().ok()?;
        let mut bytes = Vec::new();
        response.into_reader().take(MAX_BYTES).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    } else {
        std::fs::read(url).ok()
    }
}
