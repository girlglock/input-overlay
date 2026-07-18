use std::collections::HashMap;
use std::sync::mpsc::{Receiver, TryRecvError};

use tiny_skia::Pixmap;

use super::texture;

enum Entry {
    Loading(Receiver<Option<Pixmap>>),
    Ready(Option<Pixmap>),
}

pub(super) struct LabelImageCache {
    entries: HashMap<String, Entry>,
}

impl LabelImageCache {
    pub(super) fn new() -> Self {
        LabelImageCache {
            entries: HashMap::new(),
        }
    }

    pub(super) fn ensure_loading(&mut self, src: &str) {
        if self.entries.contains_key(src) {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let url = src.to_string();
        let spawned = std::thread::Builder::new()
            .name("OverlayLabelImage".into())
            .spawn(move || {
                let _ = tx.send(texture::load_plain(&url));
            })
            .is_ok();
        if spawned {
            self.entries.insert(src.to_string(), Entry::Loading(rx));
        }
    }

    pub(super) fn poll(&mut self) {
        for entry in self.entries.values_mut() {
            if let Entry::Loading(rx) = entry {
                match rx.try_recv() {
                    Ok(pixmap) => *entry = Entry::Ready(pixmap),
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => *entry = Entry::Ready(None),
                }
            }
        }
    }

    pub(super) fn any_loading(&self) -> bool {
        self.entries
            .values()
            .any(|e| matches!(e, Entry::Loading(_)))
    }

    pub(super) fn get(&self, src: &str) -> Option<&Pixmap> {
        match self.entries.get(src) {
            Some(Entry::Ready(Some(pixmap))) => Some(pixmap),
            _ => None,
        }
    }
}
