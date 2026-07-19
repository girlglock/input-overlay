use std::time::Instant;

use super::easing::ease_standard;

//overlayVisualiser.js:257-259 and278
pub struct Transition {
    visual_t: f32,
    target: f32,
    start_t: f32,
    start_time: Instant,
}

impl Transition {
    pub fn new() -> Self {
        Transition {
            visual_t: 0.0,
            target: 0.0,
            start_t: 0.0,
            start_time: Instant::now(),
        }
    }

    pub fn set_active(&mut self, active: bool) {
        let target = if active { 1.0 } else { 0.0 };
        if target != self.target {
            self.start_t = self.visual_t;
            self.start_time = Instant::now();
            self.target = target;
        }
    }

    pub fn update(&mut self, duration_secs: f32) -> f32 {
        if duration_secs <= 0.0 {
            self.visual_t = self.target;
            return self.visual_t;
        }
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let raw = (elapsed / duration_secs).clamp(0.0, 1.0);
        self.visual_t = self.start_t + (self.target - self.start_t) * ease_standard(raw);
        self.visual_t
    }

    pub fn is_animating(&self) -> bool {
        self.visual_t != self.target
    }
}
