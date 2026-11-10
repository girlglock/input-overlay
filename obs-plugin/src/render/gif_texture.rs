use std::io::Cursor;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageDecoder};
use tiny_skia::{Color, Pixmap};

use super::texture::{decode_limits, fetch, rgba_to_tinted_pixmap};

const MIN_FRAME_DELAY: Duration = Duration::from_millis(20);
const MAX_ATTEMPTS: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(5);

struct GifState {
    frame: Option<Pixmap>,
    live: bool,
}

pub struct GifTexture {
    state: Arc<Mutex<GifState>>,
}

impl GifTexture {
    pub fn spawn(url: String, tint: Color) -> Self {
        let state = Arc::new(Mutex::new(GifState {
            frame: None,
            live: true,
        }));

        let writer = Arc::downgrade(&state);
        let spawned = std::thread::Builder::new()
            .name("MousePadGif".into())
            .spawn(move || decode_loop(&url, &writer, tint))
            .is_ok();
        if !spawned {
            tracing::error!("gif_texture: failed to spawn decode thread");
        }
        GifTexture { state }
    }

    pub fn latest_frame(&self) -> Option<Pixmap> {
        self.state.lock().unwrap().frame.clone()
    }

    pub fn is_live(&self) -> bool {
        self.state.lock().unwrap().live
    }
}

fn mark_dead(state: &Weak<Mutex<GifState>>) {
    if let Some(state) = state.upgrade() {
        state.lock().unwrap().live = false;
    }
}

fn decode_loop(url: &str, state: &Weak<Mutex<GifState>>, tint: Color) {
    let mut bytes = None;
    for attempt in 1..=MAX_ATTEMPTS {
        if state.strong_count() == 0 {
            return;
        }
        bytes = fetch(url);
        if bytes.is_some() {
            break;
        }
        tracing::warn!("gif_texture: failed to fetch {url} (attempt {attempt}/{MAX_ATTEMPTS})");
        if attempt < MAX_ATTEMPTS {
            std::thread::sleep(RETRY_DELAY);
        }
    }
    let Some(bytes) = bytes else {
        mark_dead(state);
        return;
    };

    let mut failures = 0;
    loop {
        match decode_once(&bytes, state, tint) {
            Ok(true) => failures = 0,
            Ok(false) => {
                mark_dead(state);
                return;
            }
            Err(e) => {
                failures += 1;
                tracing::warn!("gif_texture: failed to decode {url}: {e}");
                if failures >= MAX_ATTEMPTS || state.strong_count() == 0 {
                    mark_dead(state);
                    return;
                }
                std::thread::sleep(RETRY_DELAY);
            }
        }
    }
}

fn decode_once(bytes: &[u8], state: &Weak<Mutex<GifState>>, tint: Color) -> Result<bool, String> {
    let mut decoder = GifDecoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    decoder.set_limits(decode_limits()).map_err(|e| e.to_string())?;

    let mut frame_count = 0u32;
    for frame in decoder.into_frames() {
        let frame = frame.map_err(|e| e.to_string())?;
        frame_count += 1;

        let delay = Duration::from(frame.delay()).max(MIN_FRAME_DELAY);
        if let Some(pixmap) = rgba_to_tinted_pixmap(frame.buffer(), tint) {
            let Some(state) = state.upgrade() else {
                return Ok(false);
            };
            state.lock().unwrap().frame = Some(pixmap);
        }
        std::thread::sleep(delay);
    }

    if frame_count == 0 {
        return Err("no frames decoded".to_string());
    }
    Ok(frame_count > 1)
}
