use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;
use tiny_skia::{Color, Pixmap};

use super::texture::{fetch, rgba_to_tinted_pixmap};

const MIN_FRAME_DELAY: Duration = Duration::from_millis(20);

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
        let writer = state.clone();
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

fn decode_loop(url: &str, state: &Mutex<GifState>, tint: Color) {
    let bytes = loop {
        match fetch(url) {
            Some(bytes) => break bytes,
            None => {
                tracing::warn!("gif_texture: failed to fetch {url}");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    };

    loop {
        match decode_once(&bytes, state, tint) {
            Ok(true) => {}
            Ok(false) => {
                state.lock().unwrap().live = false;
                return;
            }
            Err(e) => {
                tracing::warn!("gif_texture: failed to decode {url}: {e}");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

fn decode_once(bytes: &[u8], state: &Mutex<GifState>, tint: Color) -> Result<bool, String> {
    let decoder = GifDecoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;

    let mut frame_count = 0u32;
    for frame in decoder.into_frames() {
        let frame = frame.map_err(|e| e.to_string())?;
        frame_count += 1;

        let delay = Duration::from(frame.delay()).max(MIN_FRAME_DELAY);
        if let Some(pixmap) = rgba_to_tinted_pixmap(frame.buffer(), tint) {
            state.lock().unwrap().frame = Some(pixmap);
        }
        std::thread::sleep(delay);
    }

    if frame_count == 0 {
        return Err("no frames decoded".to_string());
    }
    Ok(frame_count > 1)
}
