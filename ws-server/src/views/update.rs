use crate::app::App;
use crate::services::updater::UpdateInfo;
use crate::system::open_url;
use crate::ui::{markdown, Event, EventKind, Win};

pub struct UpdateView {
    pub win: Win,
    info: UpdateInfo,
    updating: bool,
}

impl UpdateView {
    pub fn open(info: &UpdateInfo) -> Option<Self> {
        let win = Win::open("Update Available", 560, 600, false, "update.rml")?;
        win.set_text("update-version-text", &format!("v{} is now available", info.version));
        let notes = info.body.trim();
        let notes = if notes.is_empty() { "(no release notes)" } else { notes };
        win.set_inner_rml("update-notes", &markdown::to_rml(notes));
        Some(Self { win, info: info.clone(), updating: false })
    }

    pub fn handle(&mut self, app: &mut App, ev: &Event) {
        match (ev.kind, ev.id.as_str()) {
            (EventKind::Click, "close-btn" | "update-later-btn") | (EventKind::Close, _) if !self.updating => self.win.close(),
            (EventKind::Click, "update-skip-btn") => {
                app.dismiss_update(&self.info.version);
                self.win.close();
            }
            (EventKind::Click, "update-notes-link") => open_url(&self.info.release_url),
            (EventKind::Click, "update-now-btn") if !self.updating => {
                self.updating = true;
                self.win.set_hidden("update-progress-modal", false);
                self.progress(0, "starting...");
                app.apply_update(&self.info);
            }
            _ => {}
        }
    }

    pub fn progress(&mut self, percent: u32, status: &str) {
        self.win.set_attr("update-progress-fill", "value", &percent.to_string());
        self.win.set_text("update-progress-status", status);
    }

    pub fn failed(&mut self, err: &str) {
        self.updating = false;
        self.win.set_hidden("update-progress-modal", true);
        self.win.set_text("update-progress-status", "");
        self.win.set_text("update-version-text", &format!("update failed: {err}"));
        self.win.set_property("update-version-text", "color", "#ff7070");
    }
}
