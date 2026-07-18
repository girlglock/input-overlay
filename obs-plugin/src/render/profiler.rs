use std::time::{Duration, Instant};

use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

use super::tiny_font;

const TEXT_SCALE: f32 = 2.0;
const LINE_GAP: f32 = 2.0;
const PADDING: f32 = 6.0;
const MARGIN: f32 = 8.0;

fn text_color() -> Color {
    Color::from_rgba8(90, 255, 130, 255)
}

fn bg_color() -> Color {
    Color::from_rgba8(0, 0, 0, 190)
}

pub struct Profiler {
    start: Instant,
    last: Instant,
    marks: Vec<(&'static str, Duration)>,
}

impl Profiler {
    pub fn start() -> Self {
        let now = Instant::now();
        Profiler {
            start: now,
            last: now,
            marks: Vec::new(),
        }
    }

    pub fn mark(&mut self, label: &'static str) {
        let now = Instant::now();
        self.marks.push((label, now.duration_since(self.last)));
        self.last = now;
    }

    pub fn record(&mut self, label: &'static str, duration: Duration) {
        self.marks.push((label, duration));
    }

    pub fn draw_overlay(&self, pixmap: &mut Pixmap) {
        let total = self.last.duration_since(self.start);
        let mut lines: Vec<String> = self
            .marks
            .iter()
            .map(|(label, d)| format!("{label} {}", fmt_duration(*d)))
            .collect();
        lines.push(format!("TOTAL {}", fmt_duration(total)));

        let line_h = (tiny_font::GLYPH_H as f32 + LINE_GAP) * TEXT_SCALE;
        let max_w = lines
            .iter()
            .map(|l| tiny_font::text_width(l, TEXT_SCALE))
            .fold(0.0f32, f32::max);
        let box_w = max_w + PADDING * 2.0;
        let box_h = line_h * lines.len() as f32 + PADDING * 2.0;
        let box_x = (pixmap.width() as f32 - box_w - MARGIN).max(0.0);
        let box_y = MARGIN;

        if let Some(bg_rect) = Rect::from_xywh(box_x, box_y, box_w, box_h) {
            let mut paint = Paint::default();
            paint.set_color(bg_color());
            pixmap.fill_rect(bg_rect, &paint, Transform::identity(), None);
        }

        for (i, line) in lines.iter().enumerate() {
            let ty = box_y + PADDING + i as f32 * line_h;
            tiny_font::draw_text(pixmap, line, box_x + PADDING, ty, TEXT_SCALE, text_color());
        }
    }
}

fn fmt_duration(d: Duration) -> String {
    format!("{:.3}MS", d.as_secs_f64() * 1000.0)
}
