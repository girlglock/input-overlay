use crate::ui::{Event, EventKind, Win};

pub struct PostUpdateView {
    pub win: Win,
}

impl PostUpdateView {
    pub fn open(version: &str) -> Option<Self> {
        let win = Win::open("Update Complete", 380, 120, false, "post-update.rml")?;
        let text = if version.is_empty() {
            "Update complete!".to_string()
        } else {
            format!("Successfully updated to v{version}!")
        };
        win.set_text("update-done-text", &text);
        Some(Self { win })
    }

    pub fn handle(&mut self, ev: &Event) {
        match (ev.kind, ev.id.as_str()) {
            (EventKind::Click, "close-btn" | "ok-btn") | (EventKind::Close, _) => self.win.close(),
            _ => {}
        }
    }
}
