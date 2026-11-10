use std::time::{Duration, Instant};

use tiny_skia::{Color, Pixmap, Rect};

use super::easing::ease_bounce;
use super::key_element::{draw_box, inset_rect, lerp, lerp_color, press_scaled_rect, KeyStyle};
use super::label_image::LabelImageCache;
use super::text::{
    draw_image_label, draw_label, draw_text_rotated, parse_img_tag, Corner, TextFont,
};

pub struct ScrollDisplayDef {
    default_label: String,
    up_label: String,
    down_label: String,
    dir_filter: Option<i8>,
    is_scroller: bool,
}

fn first_nonempty(labels: &[String], idx: usize, default: &str) -> String {
    labels
        .get(idx)
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

impl ScrollDisplayDef {
    pub fn scroller(labels: &[String]) -> Self {
        ScrollDisplayDef {
            default_label: first_nonempty(labels, 0, "M3"),
            up_label: first_nonempty(labels, 1, "▲"),
            down_label: first_nonempty(labels, 2, "▼"),
            dir_filter: None,
            is_scroller: true,
        }
    }

    pub fn scroll_updown(labels: &[String]) -> Self {
        ScrollDisplayDef {
            default_label: String::new(),
            up_label: first_nonempty(labels, 0, "▲"),
            down_label: first_nonempty(labels, 1, "▼"),
            dir_filter: None,
            is_scroller: false,
        }
    }

    pub fn scroll_up(label: Option<&str>) -> Self {
        let label = label.filter(|s| !s.is_empty()).unwrap_or("▲").to_string();
        ScrollDisplayDef {
            default_label: label.clone(),
            up_label: label,
            down_label: String::new(),
            dir_filter: Some(-1),
            is_scroller: false,
        }
    }

    pub fn scroll_down(label: Option<&str>) -> Self {
        let label = label.filter(|s| !s.is_empty()).unwrap_or("▼").to_string();
        ScrollDisplayDef {
            default_label: label.clone(),
            up_label: String::new(),
            down_label: label,
            dir_filter: Some(1),
            is_scroller: false,
        }
    }

    fn arrow_label(&self, last_direction: i8) -> &str {
        if self.dir_filter.is_some() {
            &self.default_label
        } else {
            match last_direction {
                -1 => &self.up_label,
                1 => &self.down_label,
                _ => &self.default_label,
            }
        }
    }

    pub(super) fn all_labels(&self) -> [&str; 3] {
        [&self.default_label, &self.up_label, &self.down_label]
    }
}

pub struct ScrollState {
    current_count: u32,
    last_direction: i8,
    up_active: bool,
    down_active: bool,
    up_count: u32,
    down_count: u32,
    deadline: Option<Instant>,
    //overlayVisualiser.js:796-802
    last_tick: Instant,
}

impl ScrollState {
    pub fn new() -> Self {
        ScrollState {
            current_count: 0,
            last_direction: 0,
            up_active: false,
            down_active: false,
            up_count: 0,
            down_count: 0,
            deadline: None,
            last_tick: Instant::now(),
        }
    }

    pub fn handle_scroll(&mut self, rotation: i8, hold_ms: f32) {
        let dir: i8 = if rotation < 0 { -1 } else { 1 };
        if self.last_direction != 0 && self.last_direction != dir {
            self.current_count = 0;
        }
        self.last_direction = dir;
        self.current_count += 1;

        if dir == -1 {
            self.up_active = true;
            self.up_count = self.current_count;
        } else {
            self.down_active = true;
            self.down_count = self.current_count;
        }

        self.last_tick = Instant::now();
        self.deadline = Some(Instant::now() + Duration::from_secs_f32(hold_ms.max(0.0) / 1000.0));
    }

    fn pop_progress(&self) -> f32 {
        (self.last_tick.elapsed().as_secs_f32() / 0.1).min(1.0)
    }

    pub fn prune_expired(&mut self) {
        if self.deadline.is_some_and(|d| Instant::now() >= d) {
            *self = ScrollState::new();
        }
    }

    pub fn is_live(&self) -> bool {
        self.deadline.is_some()
    }

    pub fn is_active(&self, def: &ScrollDisplayDef, mouse_middle_held: bool) -> bool {
        let (scroll_active, _, _) = self.display_state(def);
        (def.is_scroller && mouse_middle_held) || scroll_active
    }

    fn display_state(&self, def: &ScrollDisplayDef) -> (bool, u32, i8) {
        match def.dir_filter {
            Some(-1) => (self.up_active, self.up_count, -1),
            Some(1) => (self.down_active, self.down_count, 1),
            _ => (
                self.up_active || self.down_active,
                self.current_count,
                self.last_direction,
            ),
        }
    }
}

const COUNT_FONT_SIZE: f32 = 64.0;
const COUNT_OFFSET_X: f32 = 20.0;
const COUNT_OFFSET_Y: f32 = 30.0;

#[allow(clippy::too_many_arguments)]
pub fn draw_scroll_display(
    pixmap: &mut Pixmap,
    rect: &Rect,
    style: &KeyStyle,
    def: &ScrollDisplayDef,
    scroll: &ScrollState,
    t: f32,
    hide_count: bool,
    scale: f32,
    font: Option<&TextFont>,
    images: &LabelImageCache,
) {
    let press_scale_factor = lerp(1.0, style.press_scale, t);
    let rect = press_scaled_rect(rect, style, t);
    let radius = lerp(style.border_radius, style.pressed_radius, t);
    let border_width = lerp(
        style.outline_width_unpressed,
        style.outline_width_pressed,
        t,
    );
    let fill_color = lerp_color(style.background, style.active_background, t);
    let border_color = lerp_color(style.outline, style.active_outline, t);
    let text_color = lerp_color(style.inactive_font_color, style.font_color, t);
    let font_size = style.font_size * press_scale_factor;

    let fill_rect = draw_box(
        pixmap,
        &rect,
        radius,
        border_width,
        fill_color,
        border_color,
    );

    let Some(font) = font else {
        return;
    };
    let content_rect = inset_rect(&fill_rect, style.label_padding);
    let arrow_label = def.arrow_label(scroll.last_direction);
    if let Some(tag) = parse_img_tag(arrow_label) {
        if let Some(image) = images.get(tag.src) {
            draw_image_label(pixmap, image, &content_rect, &tag, scale);
        }
    } else {
        draw_label(
            pixmap,
            arrow_label,
            &content_rect,
            text_color,
            font_size,
            font,
        );
    }

    let (_, count, badge_dir) = scroll.display_state(def);
    if hide_count || count == 0 {
        return;
    }
    let text = format!("{count}x");
    let size = COUNT_FONT_SIZE * scale;
    let off_x = COUNT_OFFSET_X * scale;
    let off_y = COUNT_OFFSET_Y * scale;
    let (anchor, corner) = if badge_dir == -1 {
        ((rect.left() - off_x, rect.top() - off_y), Corner::TopLeft)
    } else {
        (
            (rect.right() + off_x, rect.bottom() + off_y),
            Corner::BottomRight,
        )
    };

    let outline = Some(contrast_color(style.font_color)); //overlayVisualiser.js:261-263
    let (angle, pop_scale) = bounce_state(scroll.pop_progress());
    draw_text_rotated(
        pixmap,
        &text,
        anchor.0,
        anchor.1,
        corner,
        size,
        style.font_color,
        font,
        outline,
        angle,
        pop_scale,
    );
}

fn bounce_state(pop_progress: f32) -> (f32, f32) {
    if pop_progress >= 1.0 {
        (45.0, 1.0)
    } else {
        let e = ease_bounce(pop_progress);
        (lerp(15.0, 0.0, e), lerp(0.7, 1.0, e))
    }
}

fn contrast_color(color: Color) -> Color {
    let c = color.to_color_u8();
    let value = ((c.red() as u32) << 16) | ((c.green() as u32) << 8) | c.blue() as u32;
    if value > 0x00FF_FFFF / 2 {
        Color::BLACK
    } else {
        Color::WHITE
    }
}
