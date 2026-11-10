use std::collections::HashMap;
use std::time::{Duration, Instant};

use tiny_skia::{
    BlendMode, Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform,
};

use super::analog::{AnalogVisual, LegendMode};
use super::blur::gaussian_like_blur;
use super::label_image::LabelImageCache;
use super::text::{
    draw_image_label, draw_label, draw_label_bottom_clipped, draw_text_right_aligned_bottom,
    parse_img_tag, TextFont,
};

pub type GlowCache = HashMap<(i32, i32), Pixmap>;

#[derive(Clone, Copy)]
pub struct KeyStyle {
    pub background: Color,
    pub active_background: Color,
    pub outline: Color,
    pub active_outline: Color,
    pub glow: Color,
    pub font_color: Color,
    pub inactive_font_color: Color,
    pub border_radius: f32,
    pub pressed_radius: f32,
    pub outline_width_unpressed: f32,
    pub outline_width_pressed: f32,
    pub press_scale: f32,
    pub glow_radius: f32,
    pub font_size: f32,
    pub label_padding: f32,
}

impl KeyStyle {
    pub fn scaled(&self, factor: f32) -> KeyStyle {
        KeyStyle {
            border_radius: self.border_radius * factor,
            pressed_radius: self.pressed_radius * factor,
            outline_width_unpressed: self.outline_width_unpressed * factor,
            outline_width_pressed: self.outline_width_pressed * factor,
            glow_radius: self.glow_radius * factor,
            font_size: self.font_size * factor,
            label_padding: self.label_padding * factor,
            ..*self
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_key_face(
    pixmap: &mut Pixmap,
    rect: &Rect,
    style: &KeyStyle,
    t: f32,
    label: &str,
    font: Option<&TextFont>,
    analog: Option<AnalogVisual>,
    scale: f32,
    images: &LabelImageCache,
) {
    let analog_fill = analog.filter(|a| a.display_mode.show_fill());
    let shape_t = resolve_shape_t(t, analog);

    let press_scale_factor = lerp(1.0, style.press_scale, shape_t);
    let rect = if (press_scale_factor - 1.0).abs() > f32::EPSILON {
        scale_around_center(rect, press_scale_factor)
    } else {
        *rect
    };
    let radius = lerp(style.border_radius, style.pressed_radius, shape_t);
    let border_width = lerp(
        style.outline_width_unpressed,
        style.outline_width_pressed,
        shape_t,
    );
    let border_color = lerp_color(style.outline, style.active_outline, t);
    let text_color = lerp_color(style.inactive_font_color, style.font_color, t);
    let font_size = style.font_size * press_scale_factor;
    let fill_color = if analog_fill.is_some() {
        style.background
    } else {
        lerp_color(style.background, style.active_background, t)
    };

    let gauge = analog_fill
        .filter(|a| a.effective_depth > 0.0)
        .map(|a| (a.effective_depth, style.active_background));
    let fill_rect = draw_box_with_gauge(
        pixmap,
        &rect,
        radius,
        border_width,
        fill_color,
        border_color,
        gauge,
    );

    if let Some(font) = font {
        let content_rect = inset_rect(&fill_rect, style.label_padding);
        draw_legend(
            pixmap,
            label,
            &content_rect,
            style,
            font_size,
            text_color,
            analog,
            font,
            images,
            scale,
        );
    }

    if let Some(a) = analog {
        if a.display_mode.show_percent() {
            draw_percent_overlay(
                pixmap,
                &fill_rect,
                a.effective_depth,
                text_color,
                font,
                scale,
            );
        }
    }
}

fn draw_box_with_gauge(
    pixmap: &mut Pixmap,
    rect: &Rect,
    radius: f32,
    border_width: f32,
    fill_color: Color,
    border_color: Color,
    gauge: Option<(f32, Color)>,
) -> Rect {
    let fill_rect = inset_rect(rect, border_width);
    let fill_radius = (radius - border_width).max(0.0);
    match gauge {
        Some((depth, gauge_color)) => draw_gauge_fill(
            pixmap,
            &fill_rect,
            fill_radius,
            fill_color,
            depth,
            gauge_color,
        ),
        None => fill_rounded_rect(pixmap, &fill_rect, fill_radius, fill_color),
    }
    draw_border_stroke(pixmap, rect, radius, border_width, border_color);
    fill_rect
}

fn draw_gauge_fill(
    pixmap: &mut Pixmap,
    fill_rect: &Rect,
    fill_radius: f32,
    fill_color: Color,
    depth: f32,
    gauge_color: Color,
) {
    let buf_w = fill_rect.width().round().max(1.0) as u32;
    let buf_h = fill_rect.height().round().max(1.0) as u32;
    let Some(mut buf) = Pixmap::new(buf_w, buf_h) else {
        return;
    };
    buf.fill(fill_color);

    let gauge_h = buf_h as f32 * depth.clamp(0.0, 1.0);
    if let Some(gauge_rect) = Rect::from_xywh(0.0, buf_h as f32 - gauge_h, buf_w as f32, gauge_h) {
        let mut paint = Paint::default();
        paint.set_color(gauge_color);
        buf.fill_rect(gauge_rect, &paint, Transform::identity(), None);
    }

    clip_to_rounded_rect(&mut buf, fill_radius);
    pixmap.draw_pixmap(
        fill_rect.x().round() as i32,
        fill_rect.y().round() as i32,
        buf.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_legend(
    pixmap: &mut Pixmap,
    label: &str,
    content_rect: &Rect,
    style: &KeyStyle,
    font_size: f32,
    text_color: Color,
    analog: Option<AnalogVisual>,
    font: &TextFont,
    images: &LabelImageCache,
    scale: f32,
) {
    if let Some(tag) = parse_img_tag(label) {
        if let Some(image) = images.get(tag.src) {
            draw_image_label(pixmap, image, content_rect, &tag, scale);
        }
        return;
    }
    match analog {
        Some(a) if a.legend_mode == LegendMode::Fading => {
            let color = lerp_color(
                style.inactive_font_color,
                style.font_color,
                a.depth.min(1.0),
            );
            draw_label(pixmap, label, content_rect, color, font_size, font);
        }
        Some(a) if a.legend_mode == LegendMode::Inverting => {
            draw_label(
                pixmap,
                label,
                content_rect,
                style.inactive_font_color,
                font_size,
                font,
            );
            if a.effective_depth > 0.0 {
                draw_label_bottom_clipped(
                    pixmap,
                    label,
                    content_rect,
                    style.font_color,
                    font_size,
                    font,
                    a.effective_depth,
                );
            }
        }
        _ => draw_label(pixmap, label, content_rect, text_color, font_size, font),
    }
}

fn draw_percent_overlay(
    pixmap: &mut Pixmap,
    fill_rect: &Rect,
    effective_depth: f32,
    text_color: Color,
    font: Option<&TextFont>,
    scale: f32,
) {
    if effective_depth <= 0.0 {
        return;
    }
    let Some(font) = font else {
        return;
    };
    let text = format!("{}%", (effective_depth * 100.0).round() as i32);
    let size = 13.5 * scale;
    let right_x = fill_rect.right() - 3.0 * scale;
    let bottom_y = fill_rect.bottom() - 2.0 * scale;
    let color = with_alpha(text_color, 0.8);
    draw_text_right_aligned_bottom(pixmap, &text, right_x, bottom_y, size, color, font, None);
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::from_rgba(
        color.red(),
        color.green(),
        color.blue(),
        color.alpha() * alpha,
    )
    .unwrap_or(color)
}

pub(super) fn resolve_shape_t(t: f32, analog: Option<AnalogVisual>) -> f32 {
    match analog {
        Some(a) if a.display_mode.show_fill() => a.effective_depth,
        _ => t,
    }
}

pub fn draw_key_glow(
    pixmap: &mut Pixmap,
    rect: &Rect,
    style: &KeyStyle,
    t: f32,
    analog: Option<AnalogVisual>,
    glow_cache: &mut GlowCache,
    glow_time: &mut Duration,
) {
    let shape_t = resolve_shape_t(t, analog);
    let rect = press_scaled_rect(rect, style, shape_t);
    let t0 = Instant::now();
    draw_glow(
        pixmap,
        &rect,
        style.pressed_radius,
        style.glow,
        style.glow_radius,
        t,
        glow_cache,
    );
    *glow_time += t0.elapsed();
}

pub(super) fn press_scaled_rect(rect: &Rect, style: &KeyStyle, t: f32) -> Rect {
    let scale_factor = lerp(1.0, style.press_scale, t);
    if (scale_factor - 1.0).abs() > f32::EPSILON {
        scale_around_center(rect, scale_factor)
    } else {
        *rect
    }
}

pub(super) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub(super) fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::from_rgba(
        lerp(a.red(), b.red(), t),
        lerp(a.green(), b.green(), t),
        lerp(a.blue(), b.blue(), t),
        lerp(a.alpha(), b.alpha(), t),
    )
    .unwrap_or(a)
}

pub fn draw_box(
    pixmap: &mut Pixmap,
    rect: &Rect,
    radius: f32,
    border_width: f32,
    fill_color: Color,
    border_color: Color,
) -> Rect {
    let fill_rect = inset_rect(rect, border_width);
    let fill_radius = (radius - border_width).max(0.0);
    fill_rounded_rect(pixmap, &fill_rect, fill_radius, fill_color);
    draw_border_stroke(pixmap, rect, radius, border_width, border_color);
    fill_rect
}

fn fill_rounded_rect(pixmap: &mut Pixmap, rect: &Rect, radius: f32, color: Color) {
    if let Some(path) = rounded_rect_path(rect, radius) {
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
}

fn draw_border_stroke(
    pixmap: &mut Pixmap,
    rect: &Rect,
    radius: f32,
    border_width: f32,
    border_color: Color,
) {
    if border_width > 0.0 {
        let stroke_rect = inset_rect(rect, border_width / 2.0);
        let stroke_radius = (radius - border_width / 2.0).max(0.0);
        if let Some(path) = rounded_rect_path(&stroke_rect, stroke_radius) {
            let mut stroke_paint = Paint::default();
            stroke_paint.set_color(border_color);
            stroke_paint.anti_alias = true;
            let stroke = Stroke {
                width: border_width,
                ..Default::default()
            };
            pixmap.stroke_path(&path, &stroke_paint, &stroke, Transform::identity(), None);
        }
    }
}

pub(super) fn inset_rect(rect: &Rect, amount: f32) -> Rect {
    Rect::from_ltrb(
        rect.left() + amount,
        rect.top() + amount,
        rect.right() - amount,
        rect.bottom() - amount,
    )
    .unwrap_or(*rect)
}

fn draw_glow(
    pixmap: &mut Pixmap,
    rect: &Rect,
    radius: f32,
    glow: Color,
    glow_radius: f32,
    t: f32,
    cache: &mut GlowCache,
) {
    const MAX_GLOW_RADIUS: f32 = 40.0;
    let glow_radius = glow_radius.min(MAX_GLOW_RADIUS);

    let std_dev = glow_radius / 2.0;
    let pad = (glow_radius.ceil() as i32 + 2).max(1);

    let buf_w = (rect.width().ceil() as i32 + pad * 2).max(1);
    let buf_h = (rect.height().ceil() as i32 + pad * 2).max(1);
    let key = (buf_w, buf_h);

    if let std::collections::hash_map::Entry::Vacant(entry) = cache.entry(key) {
        let Some(mut glow_buf) = Pixmap::new(buf_w as u32, buf_h as u32) else {
            return;
        };

        let shape_rect = Rect::from_xywh(pad as f32, pad as f32, rect.width(), rect.height());
        if let Some(shape_rect) = shape_rect {
            if let Some(path) = rounded_rect_path(&shape_rect, radius) {
                let mut paint = Paint::default();
                paint.set_color(glow);
                paint.anti_alias = true;
                glow_buf.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }

        gaussian_like_blur(glow_buf.data_mut(), buf_w as usize, buf_h as usize, std_dev);
        entry.insert(glow_buf);
    }

    let glow_buf = &cache[&key];
    let paint = PixmapPaint {
        opacity: t.clamp(0.0, 1.0),
        ..PixmapPaint::default()
    };
    pixmap.draw_pixmap(
        rect.x().round() as i32 - pad,
        rect.y().round() as i32 - pad,
        glow_buf.as_ref(),
        &paint,
        Transform::identity(),
        None,
    );
}

fn scale_around_center(rect: &Rect, scale: f32) -> Rect {
    let cx = rect.x() + rect.width() / 2.0;
    let cy = rect.y() + rect.height() / 2.0;
    let w = rect.width() * scale;
    let h = rect.height() * scale;
    Rect::from_xywh(cx - w / 2.0, cy - h / 2.0, w, h).unwrap_or(*rect)
}

const KAPPA: f32 = 0.552_284_8;

pub(super) fn rounded_rect_path(rect: &Rect, radius: f32) -> Option<tiny_skia::Path> {
    let r = radius
        .max(0.0)
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0);
    let (x, y, w, h) = (rect.x(), rect.y(), rect.width(), rect.height());

    let mut pb = PathBuilder::new();
    if r <= 0.01 {
        pb.push_rect(*rect);
        return pb.finish();
    }

    let k = r * KAPPA;

    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

pub(super) fn clip_to_rounded_rect(pixmap: &mut Pixmap, radius: f32) {
    if radius <= 0.01 {
        return;
    }
    let Some(outer) = Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32)
    else {
        return;
    };
    let Some(rounded) = rounded_rect_path(&outer, radius) else {
        return;
    };

    let mut pb = PathBuilder::new();
    pb.push_rect(outer);
    pb.push_path(&rounded);
    let Some(corners) = pb.finish() else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(Color::TRANSPARENT);
    paint.anti_alias = true;
    paint.blend_mode = BlendMode::Clear;
    pixmap.fill_path(
        &corners,
        &paint,
        FillRule::EvenOdd,
        Transform::identity(),
        None,
    );
}
