use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

use tiny_skia::{
    Color, FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform,
};

use super::gif_texture::GifTexture;
use super::key_element::{clip_to_rounded_rect, draw_box};
use super::text::{draw_text_right_aligned_bottom, TextFont};
use super::texture;

const HARD_CAP: usize = 5000;
const STEPS: u32 = 6;
const TAPER_PTS: usize = 12;
const CHECKER_SIZE: f32 = 12.0; //overlayVisualiser.js:1153
const TINT_ALPHA: f32 = 0.13;

#[derive(Clone, Copy, PartialEq)]
pub enum PadMode {
    Wrap,
    Pan,
}

pub struct MousePadDrawParams<'a> {
    pub trail_px: f32,
    pub active_color: Color,
    pub font_color: Color,
    pub max_age_ms: f32,
    pub background: Color,
    pub outline: Color,
    pub border_radius: f32,
    pub border_width: f32,
    pub font: Option<TextFont<'a>>,
}

pub struct MousePadConfig {
    pub mode: PadMode,
    pub m1_highlight: bool,
    pub show_distance: bool,
    pub dpi: f32,
    pub reset_distance_after_fade: bool,
    pub texture_url: String,
    pub texture_zoom: f32,
    pub texture_opacity: f32,
}

struct TrailPoint {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    t: Instant,
    m1: bool,
    d: f32,
}

pub struct MousePadState {
    mode: PadMode,
    cursor_x: f32,
    cursor_y: f32,
    pending_recenter: bool,
    trail: VecDeque<Option<TrailPoint>>,
    local_w: f32,
    local_h: f32,
    pan_x: f32,
    pan_y: f32,
    total_distance_px: f32,
    was_live_trail: bool,
    texture: Option<Pixmap>,
    texture_rx: Option<Receiver<Option<Pixmap>>>,
    gif: Option<GifTexture>,
    texture_zoom: f32,
    texture_opacity: f32,
    m1_highlight: bool,
    show_distance: bool,
    dpi: f32,
    reset_distance_after_fade: bool,
}

impl MousePadState {
    pub fn new(local_w: f32, local_h: f32, config: &MousePadConfig, active_color: Color) -> Self {
        let url = config.texture_url.trim();
        let (texture_rx, gif) = if url.is_empty() {
            (None, None)
        } else if texture::is_gif(url) {
            (None, Some(GifTexture::spawn(url.to_string(), active_color)))
        } else {
            let url = url.to_string();
            let (tx, rx) = std::sync::mpsc::channel();
            let spawned = std::thread::Builder::new()
                .name("MousePadTexture".into())
                .spawn(move || {
                    let _ = tx.send(texture::load(&url, active_color));
                })
                .is_ok();
            (spawned.then_some(rx), None)
        };

        MousePadState {
            mode: config.mode,
            cursor_x: local_w / 2.0,
            cursor_y: local_h / 2.0,
            pending_recenter: false,
            trail: VecDeque::new(),
            local_w,
            local_h,
            pan_x: 0.0,
            pan_y: 0.0,
            total_distance_px: 0.0,
            was_live_trail: false,
            texture: None,
            texture_rx,
            gif,
            texture_zoom: config.texture_zoom.max(0.01),
            texture_opacity: config.texture_opacity,
            m1_highlight: config.m1_highlight,
            show_distance: config.show_distance,
            dpi: config.dpi.max(1.0),
            reset_distance_after_fade: config.reset_distance_after_fade,
        }
    }

    pub fn handle_move(
        &mut self,
        dx: f32,
        dy: f32,
        sensitivity: f32,
        max_trail_length: f32,
        m1_active: bool,
    ) {
        let (w, h) = (self.local_w, self.local_h);
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        if self.pending_recenter {
            self.cursor_x = w / 2.0;
            self.cursor_y = h / 2.0;
            self.pending_recenter = false;
            return;
        }

        let scale = ((w * h) / (300.0 * 200.0)).sqrt();
        let base_sensitivity = 0.05 * sensitivity;
        let m1 = self.m1_highlight && m1_active;
        let now = Instant::now();

        self.total_distance_px += (dx * dx + dy * dy).sqrt();//overlayVisualiser.js:1013 & 1030

        match self.mode {
            PadMode::Pan => {
                let mov_x = dx * base_sensitivity * scale;
                let mov_y = dy * base_sensitivity * scale;
                self.pan_x -= mov_x;
                self.pan_y -= mov_y;
                let seg_len = (mov_x * mov_x + mov_y * mov_y).sqrt();
                let prev_dist = self
                    .trail
                    .back()
                    .and_then(|p| p.as_ref())
                    .map(|p| p.d)
                    .unwrap_or(0.0);
                self.trail.push_back(Some(TrailPoint {
                    x: 0.0,
                    y: 0.0,
                    dx: mov_x,
                    dy: mov_y,
                    t: now,
                    m1,
                    d: prev_dist + seg_len,
                }));
            }
            PadMode::Wrap => {
                let prev_x = self.cursor_x;
                let prev_y = self.cursor_y;
                self.cursor_x = (self.cursor_x + dx * base_sensitivity * scale).rem_euclid(w);
                self.cursor_y = (self.cursor_y + dy * base_sensitivity * scale).rem_euclid(h);

                let wrapped = (self.cursor_x - prev_x).abs() > w / 2.0
                    || (self.cursor_y - prev_y).abs() > h / 2.0;
                if wrapped {
                    self.trail.push_back(None);
                }

                let seg_len = if wrapped {
                    0.0
                } else {
                    ((self.cursor_x - prev_x).powi(2) + (self.cursor_y - prev_y).powi(2)).sqrt()
                };
                let prev_dist = self
                    .trail
                    .iter()
                    .rev()
                    .find_map(|p| p.as_ref())
                    .map(|p| p.d)
                    .unwrap_or(0.0);
                self.trail.push_back(Some(TrailPoint {
                    x: self.cursor_x,
                    y: self.cursor_y,
                    dx: 0.0,
                    dy: 0.0,
                    t: now,
                    m1,
                    d: prev_dist + seg_len,
                }));
            }
        }

        let tip_d = self
            .trail
            .back()
            .and_then(|p| p.as_ref())
            .map(|p| p.d)
            .unwrap_or(0.0);
        while self.trail.len() > 1 {
            let first_d = match self.trail.front() {
                Some(Some(p)) => p.d,
                _ => self
                    .trail
                    .get(1)
                    .and_then(|p| p.as_ref())
                    .map(|p| p.d)
                    .unwrap_or(tip_d),
            };
            if tip_d - first_d > max_trail_length {
                self.trail.pop_front();
            } else {
                break;
            }
        }
        if self.trail.len() > HARD_CAP {
            self.trail.pop_front();
        }
    }

    // !!! bimportant overlayVisualiser.js:1169-1178
    fn prune_by_age(&mut self, max_age_ms: f32) {
        if max_age_ms <= 0.0 {
            return;
        }
        while let Some(front) = self.trail.front() {
            match front {
                None => break,
                Some(p) => {
                    if p.t.elapsed().as_secs_f32() * 1000.0 >= max_age_ms {
                        self.trail.pop_front();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    // !!! bimportant overlayVisualiser.js:1323-1324
    pub fn is_live(&self, max_age_ms: f32) -> bool {
        if self.texture_rx.is_some() || self.gif.as_ref().is_some_and(|g| g.is_live()) {
            return true;
        }
        if self.trail.is_empty() {
            return false;
        }
        let visually_live = max_age_ms <= 0.0
            || self
                .trail
                .back()
                .and_then(|p| p.as_ref())
                .is_some_and(|p| p.t.elapsed().as_secs_f32() * 1000.0 < max_age_ms);
        visually_live || self.show_distance
    }

    fn poll_texture(&mut self) {
        if let Some(gif) = &self.gif {
            self.texture = gif.latest_frame();
            return;
        }

        let Some(rx) = &self.texture_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(texture) => {
                self.texture = texture;
                self.texture_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.texture_rx = None,
        }
    }

    pub fn draw(
        &mut self,
        pixmap: &mut Pixmap,
        screen_rect: &Rect,
        scale: f32,
        params: &MousePadDrawParams,
    ) {
        self.poll_texture();
        let content_rect = draw_box(
            pixmap,
            screen_rect,
            params.border_radius,
            params.border_width,
            params.background,
            params.outline,
        );

        let (active_color, font_color, max_age_ms) =
            (params.active_color, params.font_color, params.max_age_ms);

        self.prune_by_age(max_age_ms);

        if !self.trail.is_empty() && self.trail.iter().all(|p| p.is_none()) {
            self.trail.clear();
        }
        let trail_empty = self.trail.is_empty();
        let last_point_age_ms = self
            .trail
            .iter()
            .rev()
            .find_map(|p| p.as_ref())
            .map(|p| p.t.elapsed().as_secs_f32() * 1000.0);
        let no_fadeout = max_age_ms <= 0.0;
        let idle_fade = if no_fadeout {
            if last_point_age_ms.is_some() {
                1.0
            } else {
                0.0
            }
        } else {
            last_point_age_ms
                .map(|age| (1.0 - age / max_age_ms).max(0.0))
                .unwrap_or(0.0)
        };
        let trail_visually_live = !trail_empty && (no_fadeout || idle_fade > 0.0);

        if self.reset_distance_after_fade && !trail_visually_live {
            self.total_distance_px = 0.0;
        }
        if self.was_live_trail && !trail_visually_live {
            self.trail.clear();
            if self.mode != PadMode::Pan {
                self.pending_recenter = true;
            }
        }
        self.was_live_trail = trail_visually_live;

        let buf_w = content_rect.width().round().max(1.0) as u32;
        let buf_h = content_rect.height().round().max(1.0) as u32;
        let Some(mut content) = Pixmap::new(buf_w, buf_h) else {
            return;
        };

        let content_radius = (params.border_radius - params.border_width).max(0.0);
        self.draw_background(
            &mut content,
            buf_w as f32,
            buf_h as f32,
            scale,
            active_color,
        );

        let to_local = |p: &TrailPoint, cx: f32, cy: f32| match self.mode {
            PadMode::Wrap => (p.x * scale, p.y * scale),
            PadMode::Pan => (cx * scale, cy * scale),
        };

        let trail_width = params.trail_px * scale;
        match self.mode {
            PadMode::Pan => {
                let total_len = self.trail.len();
                if total_len == 1 {
                    if let Some(Some(p)) = self.trail.back() {
                        let pt = to_local(p, self.local_w / 2.0, self.local_h / 2.0);
                        draw_tip(
                            &mut content,
                            pt,
                            idle_fade,
                            p.m1,
                            active_color,
                            font_color,
                            trail_width,
                        );
                    }
                } else if total_len > 1 {
                    let mut pts: Vec<((f32, f32), bool)> = vec![((0.0, 0.0), false); total_len];
                    let (mut cx, mut cy) = (self.local_w / 2.0, self.local_h / 2.0);
                    let items: Vec<&TrailPoint> =
                        self.trail.iter().filter_map(|p| p.as_ref()).collect();
                    if let Some(last) = items.last() {
                        pts[total_len - 1] = (to_local(last, cx, cy), last.m1);
                    }
                    for i in (0..total_len - 1).rev() {
                        let fwd = items[i + 1];
                        cx -= fwd.dx;
                        cy -= fwd.dy;
                        pts[i] = ((cx * scale, cy * scale), items[i].m1);
                    }
                    draw_smoothed_run(
                        &mut content,
                        &pts,
                        idle_fade,
                        active_color,
                        font_color,
                        trail_width,
                        true,
                    );
                }
            }
            PadMode::Wrap => {
                let mut run: Vec<((f32, f32), bool)> = Vec::new();
                for point in &self.trail {
                    match point {
                        None => {
                            //overlayVisualiser.js:1337
                            if run.len() > 1 {
                                draw_smoothed_run(
                                    &mut content,
                                    &run,
                                    idle_fade,
                                    active_color,
                                    font_color,
                                    trail_width,
                                    false,
                                );
                            }
                            run.clear();
                        }
                        Some(p) => run.push((to_local(p, 0.0, 0.0), p.m1)),
                    }
                }
                if run.len() == 1 {
                    draw_tip(
                        &mut content,
                        run[0].0,
                        idle_fade,
                        run[0].1,
                        active_color,
                        font_color,
                        trail_width,
                    );
                } else if run.len() > 1 {
                    draw_smoothed_run(
                        &mut content,
                        &run,
                        idle_fade,
                        active_color,
                        font_color,
                        trail_width,
                        true,
                    );
                }
            }
        }

        if self.show_distance {
            self.draw_distance(
                &mut content,
                buf_w as f32,
                buf_h as f32,
                active_color,
                params.font,
            );
        }

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

    fn draw_background(
        &self,
        content: &mut Pixmap,
        w: f32,
        h: f32,
        scale: f32,
        active_color: Color,
    ) {
        let pan = if self.mode == PadMode::Pan {
            Some((self.pan_x, self.pan_y))
        } else {
            None
        };

        if let Some(texture) = &self.texture {
            draw_tiled_texture(
                content,
                (w, h),
                texture,
                self.texture_zoom,
                self.texture_opacity,
                scale,
                pan,
            );
        } else if self.mode == PadMode::Pan {
            draw_checkerboard(content, w, h, scale, self.pan_x, self.pan_y, active_color);
        }
    }

    fn draw_distance(
        &self,
        content: &mut Pixmap,
        w: f32,
        h: f32,
        active_color: Color,
        font: Option<TextFont>,
    ) {
        let Some(font) = font else {
            return;
        };
        let inches_total = self.total_distance_px / self.dpi;
        let cm_total = inches_total * 2.54;
        let dist_str = if cm_total >= 100_000.0 {
            format!("{:.2} km", cm_total / 100_000.0)
        } else if cm_total >= 100.0 {
            format!("{:.2} m", cm_total / 100.0)
        } else {
            format!("{cm_total:.1} cm")
        };

        let font_size = (w.min(h) * 0.085).round().max(9.0);
        let pad = (font_size * 0.5).round();
        let right_x = w - pad;
        let bottom_y = h - pad * 0.55;
        let color = with_alpha(active_color, 0.92);
        draw_text_right_aligned_bottom(
            content, &dist_str, right_x, bottom_y, font_size, color, &font, None,
        );
    }
}

fn draw_tiled_texture(
    content: &mut Pixmap,
    (w, h): (f32, f32),
    texture: &Pixmap,
    zoom: f32,
    opacity: f32,
    scale: f32,
    pan: Option<(f32, f32)>,
) {
    let tile_w = (texture.width() as f32 * zoom * scale).max(1.0);
    let tile_h = (texture.height() as f32 * zoom * scale).max(1.0);
    let (pan_x, pan_y) = pan.unwrap_or((0.0, 0.0));

    let center_off_x = w / 2.0 - tile_w / 2.0;
    let center_off_y = h / 2.0 - tile_h / 2.0;
    let wrap_x = (center_off_x + pan_x * scale).rem_euclid(tile_w);
    let wrap_y = (center_off_y + pan_y * scale).rem_euclid(tile_h);

    let paint = PixmapPaint {
        opacity: opacity.clamp(0.0, 1.0),
        quality: FilterQuality::Bilinear,
        ..Default::default()
    };
    let scale_x = tile_w / texture.width() as f32;
    let scale_y = tile_h / texture.height() as f32;

    let mut y = wrap_y - tile_h;
    while y < h {
        let mut x = wrap_x - tile_w;
        while x < w {
            let transform = Transform::from_scale(scale_x, scale_y).post_translate(x, y);
            content.draw_pixmap(0, 0, texture.as_ref(), &paint, transform, None);
            x += tile_w;
        }
        y += tile_h;
    }
}

fn draw_checkerboard(
    content: &mut Pixmap,
    w: f32,
    h: f32,
    scale: f32,
    pan_x: f32,
    pan_y: f32,
    active_color: Color,
) {
    let sz = CHECKER_SIZE * scale;
    let period = sz * 2.0;
    let off_x = (pan_x * scale).rem_euclid(period);
    let off_y = (pan_y * scale).rem_euclid(period);

    let color = with_alpha(active_color, TINT_ALPHA);
    let rows = (h / sz).ceil() as i32 + 2;
    let cols = (w / sz).ceil() as i32 + 2;
    for row in -1..rows {
        for col in -1..cols {
            if (row + col) % 2 == 0 {
                continue;
            }
            let x = (col as f32 * sz + off_x - sz).floor();
            let y = (row as f32 * sz + off_y - sz).floor();
            if let Some(rect) = Rect::from_xywh(x, y, sz, sz) {
                fill_rect(content, &rect, color);
            }
        }
    }
}

fn catmull_rom(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    t: f32,
) -> (f32, f32) {
    let t2 = t * t;
    let t3 = t2 * t;
    let x = 0.5
        * ((2.0 * p1.0)
            + (-p0.0 + p2.0) * t
            + (2.0 * p0.0 - 5.0 * p1.0 + 4.0 * p2.0 - p3.0) * t2
            + (-p0.0 + 3.0 * p1.0 - 3.0 * p2.0 + p3.0) * t3);
    let y = 0.5
        * ((2.0 * p1.1)
            + (-p0.1 + p2.1) * t
            + (2.0 * p0.1 - 5.0 * p1.1 + 4.0 * p2.1 - p3.1) * t2
            + (-p0.1 + 3.0 * p1.1 - 3.0 * p2.1 + p3.1) * t3);
    (x, y)
}

fn pad_color(m1: bool, fade: f32, active_color: Color, font_color: Color) -> Color {
    if m1 {
        with_alpha(font_color, (fade * 1.2).min(1.0))
    } else {
        with_alpha(active_color, fade)
    }
}

// !!! bimportant overlayVisualiser.js:1201-1259 & 1252 & 1337 drawTipDot
fn draw_smoothed_run(
    pixmap: &mut Pixmap,
    pts: &[((f32, f32), bool)],
    fade: f32,
    active_color: Color,
    font_color: Color,
    base_width: f32,
    draw_tip_dot: bool,
) {
    if pts.len() < 2 {
        return;
    }
    let taper_end = (pts.len() - 1).min(TAPER_PTS);
    let at = |i: i64| -> (f32, f32) { pts[i.clamp(0, pts.len() as i64 - 1) as usize].0 };

    let mut samples: Vec<((f32, f32), f32, bool)> =
        Vec::with_capacity((pts.len() - 1) * STEPS as usize + 1);
    for i in 0..pts.len() - 1 {
        let is_m1 = pts[i].1 || pts[i + 1].1;
        let (p0, p1, p2, p3) = (
            at(i as i64 - 1),
            at(i as i64),
            at(i as i64 + 1),
            at(i as i64 + 2),
        );
        let start_s = if i == 0 { 0 } else { 1 };
        for s in start_s..=STEPS {
            let frac = s as f32 / STEPS as f32;
            let pt = catmull_rom(p0, p1, p2, p3, frac);
            let taper_pos = i as f32 + frac;
            let w = sample_width(taper_pos, taper_end as f32, is_m1, base_width);
            samples.push((pt, w / 2.0, is_m1));
        }
    }

    let mut run_start = 0;
    for i in 1..samples.len() {
        if samples[i].2 != samples[run_start].2 {
            fill_run(
                pixmap,
                &samples[run_start..=i],
                fade,
                active_color,
                font_color,
            );
            run_start = i;
        }
    }
    fill_run(
        pixmap,
        &samples[run_start..],
        fade,
        active_color,
        font_color,
    );

    if draw_tip_dot {
        let (tip, tip_m1) = pts[pts.len() - 1];
        draw_tip(
            pixmap,
            tip,
            fade,
            tip_m1,
            active_color,
            font_color,
            base_width,
        );
    }
}

fn sample_width(taper_pos: f32, taper_end: f32, is_m1: bool, base_width: f32) -> f32 {
    let m1_factor = if is_m1 { 1.5 } else { 1.0 };
    let taper_factor = if taper_end > 0.0 {
        (taper_pos / taper_end).min(1.0)
    } else {
        1.0
    };
    base_width * m1_factor * taper_factor
}

fn fill_run(
    pixmap: &mut Pixmap,
    samples: &[((f32, f32), f32, bool)],
    fade: f32,
    active_color: Color,
    font_color: Color,
) {
    if samples.len() < 2 {
        return;
    }
    let color = pad_color(samples[0].2, fade, active_color, font_color);
    let points: Vec<(f32, f32)> = samples.iter().map(|s| s.0).collect();
    let half_widths: Vec<f32> = samples.iter().map(|s| s.1).collect();
    fill_ribbon(pixmap, &points, &half_widths, color);
}

fn fill_ribbon(pixmap: &mut Pixmap, points: &[(f32, f32)], half_widths: &[f32], color: Color) {
    if points.len() < 2 {
        return;
    }
    let mut left = Vec::with_capacity(points.len());
    let mut right = Vec::with_capacity(points.len());
    for i in 0..points.len() {
        let (dx, dy) = if i == 0 {
            (points[1].0 - points[0].0, points[1].1 - points[0].1)
        } else if i == points.len() - 1 {
            (points[i].0 - points[i - 1].0, points[i].1 - points[i - 1].1)
        } else {
            (
                points[i + 1].0 - points[i - 1].0,
                points[i + 1].1 - points[i - 1].1,
            )
        };
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let (nx, ny) = (-dy / len, dx / len);
        let hw = half_widths[i];
        left.push((points[i].0 + nx * hw, points[i].1 + ny * hw));
        right.push((points[i].0 - nx * hw, points[i].1 - ny * hw));
    }

    let mut pb = PathBuilder::new();
    pb.move_to(left[0].0, left[0].1);
    for p in &left[1..] {
        pb.line_to(p.0, p.1);
    }
    for p in right.iter().rev() {
        pb.line_to(p.0, p.1);
    }
    pb.close();
    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn draw_tip(
    pixmap: &mut Pixmap,
    tip: (f32, f32),
    fade: f32,
    m1: bool,
    active_color: Color,
    font_color: Color,
    base_width: f32,
) {
    let w = base_width * if m1 { 1.5 } else { 1.0 };
    let r = w * 1.2;
    if r <= 0.0 {
        return;
    }
    let Some(path) = PathBuilder::from_circle(tip.0, tip.1, r) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(pad_color(m1, fade, active_color, font_color));
    paint.anti_alias = true;
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn fill_rect(pixmap: &mut Pixmap, rect: &Rect, color: Color) {
    let mut paint = Paint::default();
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
