use std::sync::OnceLock;

use fontdue::{Font, FontSettings};
use tiny_skia::{Color, FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

#[derive(Clone, Copy)]
pub struct TextFont<'a> {
    pub primary: &'a Font,
    pub fallback: Option<&'a Font>,
    pub bold: bool,
}

fn glyph_font<'a>(font: &TextFont<'a>, ch: char) -> &'a Font {
    if !font.primary.has_glyph(ch) {
        if let Some(fallback) = font.fallback {
            if fallback.has_glyph(ch) {
                return fallback;
            }
        }
    }
    font.primary
}

pub fn load_system_font(bold: bool) -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    static FONT_BOLD: OnceLock<Option<Font>> = OnceLock::new();
    let cell = if bold { &FONT_BOLD } else { &FONT };
    cell.get_or_init(|| load_from_candidates(&system_font_candidates(bold)))
        .as_ref()
}

pub fn load_symbol_fallback_font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| load_from_candidates(&symbol_fallback_candidates()))
        .as_ref()
}

#[cfg(windows)]
fn symbol_fallback_candidates() -> Vec<String> {
    let root = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    ["seguisym.ttf", "seguiemj.ttf"]
        .iter()
        .map(|f| format!("{root}\\Fonts\\{f}"))
        .collect()
}

#[cfg(target_os = "linux")]
fn symbol_fallback_candidates() -> Vec<String> {
    [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn load_from_candidates(paths: &[String]) -> Option<Font> {
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            match Font::from_bytes(bytes, FontSettings::default()) {
                Ok(font) => return Some(font),
                Err(e) => tracing::warn!("text: failed to parse font {path}: {e}"),
            }
        }
    }
    None
}

#[cfg(windows)]
fn system_font_candidates(bold: bool) -> Vec<String> {
    let root = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let files: &[&str] = if bold {
        &[
            "segoeuib.ttf",
            "arialbd.ttf",
            "calibrib.ttf",
            "tahomabd.ttf",
        ]
    } else {
        &["segoeui.ttf", "arial.ttf", "calibri.ttf", "tahoma.ttf"]
    };
    files
        .iter()
        .map(|f| format!("{root}\\Fonts\\{f}"))
        .collect()
}

#[cfg(target_os = "linux")]
fn system_font_candidates(bold: bool) -> Vec<String> {
    let files: &[&str] = if bold {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        ]
    };
    files.iter().map(|s| s.to_string()).collect()
}

//utils.js scaleKeyFontSize
pub fn draw_label(
    pixmap: &mut Pixmap,
    text: &str,
    content_rect: &Rect,
    color: Color,
    base_size: f32,
    font: &TextFont,
) {
    let text = sanitize(text);
    let Some(text) = text else {
        return;
    };
    if content_rect.width() <= 1.0 || content_rect.height() <= 1.0 || base_size <= 0.0 {
        return;
    }

    let measured_width = measure_width(font, text, base_size);
    let size = if measured_width > content_rect.width() && measured_width > 0.0 {
        (base_size * (content_rect.width() / measured_width)).max(1.0)
    } else {
        base_size
    };

    let (ascent, descent) = line_metrics(font.primary, size);
    let text_height = ascent - descent;
    let text_width = measure_width(font, text, size);

    let start_x = content_rect.x() + (content_rect.width() - text_width) / 2.0;
    let baseline_y = content_rect.y() + (content_rect.height() - text_height) / 2.0 + ascent;
    draw_text_at(pixmap, text, start_x, baseline_y, size, color, font, None);
}

pub fn draw_label_bottom_clipped(
    pixmap: &mut Pixmap,
    text: &str,
    content_rect: &Rect,
    color: Color,
    base_size: f32,
    font: &TextFont,
    fraction: f32,
) {
    let Some(text) = sanitize(text) else {
        return;
    };
    if content_rect.width() <= 1.0 || content_rect.height() <= 1.0 || base_size <= 0.0 {
        return;
    }

    let measured_width = measure_width(font, text, base_size);
    let size = if measured_width > content_rect.width() && measured_width > 0.0 {
        (base_size * (content_rect.width() / measured_width)).max(1.0)
    } else {
        base_size
    };

    let (ascent, descent) = line_metrics(font.primary, size);
    let text_height = ascent - descent;
    let text_width = measure_width(font, text, size);

    let start_x = content_rect.x() + (content_rect.width() - text_width) / 2.0;
    let baseline_y = content_rect.y() + (content_rect.height() - text_height) / 2.0 + ascent;
    let clip_top_y = content_rect.bottom() - content_rect.height() * fraction.clamp(0.0, 1.0);
    draw_text_at(
        pixmap,
        text,
        start_x,
        baseline_y,
        size,
        color,
        font,
        Some(clip_top_y),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_right_aligned_bottom(
    pixmap: &mut Pixmap,
    text: &str,
    right_x: f32,
    bottom_y: f32,
    size: f32,
    color: Color,
    font: &TextFont,
    outline: Option<Color>,
) {
    let Some(text) = sanitize(text) else {
        return;
    };
    let (_, descent) = line_metrics(font.primary, size);
    let text_width = measure_width(font, text, size);
    draw_outlined_text_at(
        pixmap,
        text,
        right_x - text_width,
        bottom_y + descent,
        size,
        color,
        font,
        outline,
    );
}

pub enum Corner {
    TopLeft,
    BottomRight,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_rotated(
    pixmap: &mut Pixmap,
    text: &str,
    anchor_x: f32,
    anchor_y: f32,
    corner: Corner,
    size: f32,
    color: Color,
    font: &TextFont,
    outline: Option<Color>,
    angle_degrees: f32,
    extra_scale: f32,
) {
    let Some(text) = sanitize(text) else {
        return;
    };
    let (ink_left, ink_top, ink_right, ink_bottom) = measure_ink_bbox(font, text, size);
    let ink_width = (ink_right - ink_left).max(1.0);
    let ink_height = (ink_bottom - ink_top).max(1.0);

    let diagonal = (ink_width * ink_width + ink_height * ink_height).sqrt();
    let buf_side = (diagonal + 4.0).ceil().max(1.0) as u32;
    let (buf_w, buf_h) = (buf_side, buf_side);
    let Some(mut buf) = Pixmap::new(buf_w, buf_h) else {
        return;
    };

    let start_x = buf_w as f32 / 2.0 - (ink_left + ink_right) / 2.0;
    let baseline_y = buf_h as f32 / 2.0 - (ink_top + ink_bottom) / 2.0;
    draw_outlined_text_at(
        &mut buf, text, start_x, baseline_y, size, color, font, outline,
    );

    let (center_x, center_y) = match corner {
        Corner::TopLeft => (anchor_x + ink_width / 2.0, anchor_y + ink_height / 2.0),
        Corner::BottomRight => (anchor_x - ink_width / 2.0, anchor_y - ink_height / 2.0),
    };

    let transform = Transform::identity()
        .post_translate(-(buf_w as f32) / 2.0, -(buf_h as f32) / 2.0)
        .post_scale(extra_scale, extra_scale)
        .post_rotate(angle_degrees)
        .post_translate(center_x, center_y);
    pixmap.draw_pixmap(0, 0, buf.as_ref(), &PixmapPaint::default(), transform, None);
}

fn measure_ink_bbox(font: &TextFont, text: &str, size: f32) -> (f32, f32, f32, f32) {
    let mut pen_x = 0.0f32;
    let mut left = f32::MAX;
    let mut top = f32::MAX;
    let mut right = f32::MIN;
    let mut bottom = f32::MIN;

    for ch in text.chars() {
        let m = glyph_font(font, ch).metrics(ch, size);
        if m.width > 0 && m.height > 0 {
            let glyph_left = pen_x + m.xmin as f32;
            let glyph_top = -(m.ymin as f32 + m.height as f32);
            left = left.min(glyph_left);
            right = right.max(glyph_left + m.width as f32);
            top = top.min(glyph_top);
            bottom = bottom.max(-(m.ymin as f32));
        }
        pen_x += m.advance_width;
    }

    if left > right {
        return (0.0, 0.0, pen_x, 0.0);
    }
    (left, top, right, bottom)
}

#[allow(clippy::too_many_arguments)]
fn draw_outlined_text_at(
    pixmap: &mut Pixmap,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    font: &TextFont,
    outline: Option<Color>,
) {
    if let Some(outline_color) = outline {
        let d = (size * 0.03).max(1.0);
        for (dx, dy) in [(-d, 0.0), (d, 0.0), (0.0, -d), (0.0, d)] {
            draw_text_at(
                pixmap,
                text,
                x + dx,
                y + dy,
                size,
                outline_color,
                font,
                None,
            );
        }
    }
    draw_text_at(pixmap, text, x, y, size, color, font, None);
}

fn sanitize(text: &str) -> Option<&str> {
    let text = text.trim();
    if text.is_empty() || contains_html_tag_start(text) {
        None
    } else {
        Some(text)
    }
}

fn contains_html_tag_start(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.iter().enumerate().any(|(i, &b)| {
        b == b'<'
            && bytes.get(i + 1).is_some_and(|&next| {
                next.is_ascii_alphabetic() || next == b'/' || next == b'!' || next == b'?'
            })
    })
}

pub struct ImgTag<'a> {
    pub src: &'a str,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub opacity: f32,
}

pub fn parse_img_tag(label: &str) -> Option<ImgTag<'_>> {
    let trimmed = label.trim();
    let after_tag = trimmed.strip_prefix("<img")?;
    if !after_tag.starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>') {
        return None;
    }
    let src = extract_quoted_attr(after_tag, "src").filter(|s| !s.is_empty())?;

    let mut max_width = None;
    let mut max_height = None;
    let mut opacity = 1.0;
    if let Some(style) = extract_quoted_attr(after_tag, "style") {
        for decl in style.split(';') {
            let Some((prop, value)) = decl.split_once(':') else {
                continue;
            };
            let value = value.replace("!important", "");
            let value = value.trim();
            match prop.trim() {
                "max-width" => max_width = parse_css_px(value),
                "max-height" => max_height = parse_css_px(value),
                "opacity" => opacity = value.parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0),
                _ => {}
            }
        }
    }

    Some(ImgTag {
        src,
        max_width,
        max_height,
        opacity,
    })
}

fn parse_css_px(value: &str) -> Option<f32> {
    let numeric = value.strip_suffix("px").unwrap_or(value).trim();
    numeric.parse::<f32>().ok().filter(|v| *v > 0.0)
}

fn extract_quoted_attr<'a>(tag_body: &'a str, attr: &str) -> Option<&'a str> {
    let attr_pos = tag_body.find(attr)?;
    let after_attr = &tag_body[attr_pos + attr.len()..];
    let eq_pos = after_attr.find('=')?;
    if !after_attr[..eq_pos].chars().all(char::is_whitespace) {
        return None;
    }
    let after_eq = after_attr[eq_pos + 1..].trim_start();
    let quote = after_eq.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after_eq[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some(&rest[..end])
}

pub fn draw_image_label(
    pixmap: &mut Pixmap,
    image: &Pixmap,
    content_rect: &Rect,
    tag: &ImgTag,
    scale: f32,
) {
    if content_rect.width() <= 1.0 || content_rect.height() <= 1.0 {
        return;
    }
    let (iw, ih) = (image.width() as f32, image.height() as f32);
    if iw <= 0.0 || ih <= 0.0 {
        return;
    }

    let mut max_w = content_rect.width();
    let mut max_h = content_rect.height();
    if let Some(mw) = tag.max_width {
        max_w = max_w.min(mw * scale);
    }
    if let Some(mh) = tag.max_height {
        max_h = max_h.min(mh * scale);
    }

    let img_scale = ((max_w / iw).min(max_h / ih)).min(1.0);
    let (draw_w, draw_h) = (iw * img_scale, ih * img_scale);
    let x = content_rect.x() + (content_rect.width() - draw_w) / 2.0;
    let y = content_rect.y() + (content_rect.height() - draw_h) / 2.0;

    let transform = Transform::from_scale(img_scale, img_scale).post_translate(x, y);
    let paint = PixmapPaint {
        opacity: tag.opacity,
        quality: FilterQuality::Bilinear,
        ..Default::default()
    };
    pixmap.draw_pixmap(0, 0, image.as_ref(), &paint, transform, None);
}

fn line_metrics(font: &Font, size: f32) -> (f32, f32) {
    font.horizontal_line_metrics(size)
        .map(|m| (m.ascent, m.descent))
        .unwrap_or((size * 0.8, -size * 0.2))
}

#[allow(clippy::too_many_arguments)]
fn draw_text_at(
    pixmap: &mut Pixmap,
    text: &str,
    start_x: f32,
    baseline_y: f32,
    size: f32,
    color: Color,
    font: &TextFont,
    clip_top_y: Option<f32>,
) {
    let bold_offset = if font.bold {
        (size * 0.04).max(0.4)
    } else {
        0.0
    };
    let mut pen_x = start_x;
    for ch in text.chars() {
        let glyph_font = glyph_font(font, ch);
        let (metrics, coverage) = glyph_font.rasterize(ch, size);
        let glyph_x = pen_x + metrics.xmin as f32;
        let glyph_y = baseline_y - (metrics.ymin as f32 + metrics.height as f32);
        blend_coverage(
            pixmap,
            &coverage,
            metrics.width,
            metrics.height,
            glyph_x,
            glyph_y,
            color,
            clip_top_y,
        );
        if bold_offset > 0.0 {
            blend_coverage(
                pixmap,
                &coverage,
                metrics.width,
                metrics.height,
                glyph_x + bold_offset,
                glyph_y,
                color,
                clip_top_y,
            );
        }
        pen_x += metrics.advance_width;
    }
}

fn measure_width(font: &TextFont, text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| glyph_font(font, c).metrics(c, size).advance_width)
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn blend_coverage(
    pixmap: &mut Pixmap,
    coverage: &[u8],
    w: usize,
    h: usize,
    x: f32,
    y: f32,
    color: Color,
    clip_top_y: Option<f32>,
) {
    let px_w = pixmap.width() as i32;
    let px_h = pixmap.height() as i32;
    let ox = x.round() as i32;
    let oy = y.round() as i32;
    let clip_top_y = clip_top_y.map(|c| c.round() as i32);
    let c = color.to_color_u8();
    let data = pixmap.data_mut();

    for row in 0..h {
        let py = oy + row as i32;
        if py < 0 || py >= px_h {
            continue;
        }
        if clip_top_y.is_some_and(|clip| py < clip) {
            continue;
        }
        for col in 0..w {
            let px = ox + col as i32;
            if px < 0 || px >= px_w {
                continue;
            }
            let cov = coverage[row * w + col];
            if cov == 0 {
                continue;
            }
            let idx = ((py * px_w + px) * 4) as usize;
            blend_pixel_over(
                &mut data[idx..idx + 4],
                c.red(),
                c.green(),
                c.blue(),
                c.alpha(),
                cov,
            );
        }
    }
}

fn blend_pixel_over(dst: &mut [u8], r: u8, g: u8, b: u8, a: u8, coverage: u8) {
    let src_a = a as u32 * coverage as u32 / 255;
    let inv = 255 - src_a;
    let premul = |channel: u8| channel as u32 * src_a / 255;

    dst[0] = (premul(r) + dst[0] as u32 * inv / 255).min(255) as u8;
    dst[1] = (premul(g) + dst[1] as u32 * inv / 255).min(255) as u8;
    dst[2] = (premul(b) + dst[2] as u32 * inv / 255).min(255) as u8;
    dst[3] = (src_a + dst[3] as u32 * inv / 255).min(255) as u8;
}