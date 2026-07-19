use std::collections::VecDeque;
use std::time::Instant;

use tiny_skia::{Color, Pixmap, PixmapPaint, Rect, Transform};

use super::key_element::{clip_to_rounded_rect, draw_box};

//overlayVisualiser.js:1413-1471
struct LogEntry {
    key: String,
    start: Instant,
    end: Option<Instant>,
}

pub struct HistoryDrawParams {
    pub outline_color: Color,
    pub active_color: Color,
    pub highlight_color: Color,
    pub background: Color,
    pub border_radius: f32,
    pub border_width: f32,
}

pub struct HistoryState {
    tracked_keys: Vec<String>,
    vertical: bool,
    scroll_speed: f32,
    highlight_overlap: bool,
    reverse_direction: bool,
    log: VecDeque<LogEntry>,
}

impl HistoryState {
    pub fn new(
        tracked_keys: Vec<String>,
        vertical: bool,
        scroll_speed: f32,
        highlight_overlap: bool,
        reverse_direction: bool,
    ) -> Self {
        HistoryState {
            tracked_keys,
            vertical,
            scroll_speed,
            highlight_overlap,
            reverse_direction,
            log: VecDeque::new(),
        }
    }

    pub fn tracks(&self, key: &str) -> bool {
        self.tracked_keys.iter().any(|k| k == key)
    }

    //overlayVisualiser.js:125-142
    pub fn record_edge(&mut self, key: &str, is_press: bool) {
        if is_press {
            self.log.push_back(LogEntry {
                key: key.to_string(),
                start: Instant::now(),
                end: None,
            });
        } else {
            for entry in self.log.iter_mut().rev() {
                if entry.key == key && entry.end.is_none() {
                    entry.end = Some(Instant::now());
                    break;
                }
            }
        }
    }

    pub fn is_live(&self) -> bool {
        !self.log.is_empty()
    }

    pub fn draw(
        &mut self,
        pixmap: &mut Pixmap,
        screen_rect: &Rect,
        scale: f32,
        params: &HistoryDrawParams,
    ) {
        let content_rect = draw_box(
            pixmap,
            screen_rect,
            params.border_radius,
            params.border_width,
            params.background,
            params.outline_color,
        );

        if self.tracked_keys.is_empty() {
            return;
        }

        let buf_w = content_rect.width().round().max(1.0) as u32;
        let buf_h = content_rect.height().round().max(1.0) as u32;
        let Some(mut content) = Pixmap::new(buf_w, buf_h) else {
            return;
        };
        let (w, h) = (buf_w as f32, buf_h as f32);

        let now = Instant::now();
        let px_per_ms = self.scroll_speed * scale / 1000.0;
        let (scroll_axis_size, cross_axis_size) = if self.vertical { (h, w) } else { (w, h) };
        let lane_size = cross_axis_size / self.tracked_keys.len() as f32;
        let bar_thickness = (lane_size - 1.0).max(1.0);
        let lane_inset = (lane_size - bar_thickness) / 2.0;

        let ago_px = |t: Instant| t.elapsed().as_secs_f32() * 1000.0 * px_per_ms;
        let pos_of = |t: Instant| {
            if self.reverse_direction {
                scroll_axis_size - ago_px(t)
            } else {
                ago_px(t)
            }
        };
        let bar_range = |start: Instant, end: Instant| {
            let p0 = pos_of(start);
            let p1 = pos_of(end);
            (p0.min(p1), (p1 - p0).abs().max(1.0))
        };

        self.log
            .retain(|e| ago_px(e.end.unwrap_or(now)) <= scroll_axis_size);

        for i in 1..self.tracked_keys.len() {
            let p = i as f32 * lane_size;
            let rect = if self.vertical {
                Rect::from_xywh(p - 0.5, 0.0, 1.0, h)
            } else {
                Rect::from_xywh(0.0, p - 0.5, w, 1.0)
            };
            if let Some(rect) = rect {
                fill_rect(&mut content, &rect, with_alpha(params.outline_color, 0.5));
            }
        }

        let bar_color = with_alpha(params.active_color, 0.9);
        for (i, key_name) in self.tracked_keys.iter().enumerate() {
            let lane_start = i as f32 * lane_size + lane_inset;
            for entry in &self.log {
                if &entry.key != key_name {
                    continue;
                }
                let (bar_start, bar_len) = bar_range(entry.start, entry.end.unwrap_or(now));
                let rect = if self.vertical {
                    Rect::from_xywh(lane_start, bar_start, bar_thickness, bar_len)
                } else {
                    Rect::from_xywh(bar_start, lane_start, bar_len, bar_thickness)
                };
                if let Some(rect) = rect {
                    fill_rect(&mut content, &rect, bar_color);
                }
            }
        }

        if self.highlight_overlap {
            for (range_start, range_end) in overlap_ranges(&self.log, now) {
                let (bar_start, bar_len) = bar_range(range_start, range_end);
                let rect = if self.vertical {
                    Rect::from_xywh(0.0, bar_start, w, bar_len)
                } else {
                    Rect::from_xywh(bar_start, 0.0, bar_len, h)
                };
                if let Some(rect) = rect {
                    fill_rect(&mut content, &rect, params.highlight_color);
                }
            }
        }

        let content_radius = (params.border_radius - params.border_width).max(0.0);
        clip_to_rounded_rect(&mut content, content_radius);
        pixmap.draw_pixmap(
            content_rect.x().round() as i32,
            content_rect.y().round() as i32,
            content.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}

//overlayVisualiser.js:1391-1411
fn overlap_ranges(log: &VecDeque<LogEntry>, now: Instant) -> Vec<(Instant, Instant)> {
    let mut events: Vec<(Instant, i32)> = Vec::new();
    for e in log {
        let end = e.end.unwrap_or(now);
        if end <= e.start {
            continue;
        }
        events.push((e.start, 1));
        events.push((end, -1));
    }
    events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut ranges = Vec::new();
    let mut active = 0;
    let mut seg_start = None;
    for (t, delta) in events {
        let was_overlap = active >= 2;
        active += delta;
        let is_overlap = active >= 2;
        if !was_overlap && is_overlap {
            seg_start = Some(t);
        } else if was_overlap && !is_overlap {
            if let Some(s) = seg_start.take() {
                ranges.push((s, t));
            }
        }
    }
    ranges
}

fn fill_rect(pixmap: &mut Pixmap, rect: &Rect, color: Color) {
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(color);
    pixmap.fill_rect(*rect, &paint, Transform::identity(), None);
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    let c = color.to_color_u8();
    Color::from_rgba8(
        c.red(),
        c.green(),
        c.blue(),
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}
