use tiny_skia::{
    Color, FillRule, GradientStop, Paint, PathBuilder, Pixmap, Point, RadialGradient, Rect,
    SpreadMode, Stroke, Transform,
};

use super::key_element::{lerp, lerp_color, KeyStyle};

//overlayVisualiser.js _drawJoystick
pub fn draw_joystick(pixmap: &mut Pixmap, rect: &Rect, style: &KeyStyle, x: f32, y: f32) {
    let (w, h) = (rect.width(), rect.height());
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let pos_x = (x.clamp(-1.0, 1.0) + 1.0) / 2.0;
    let pos_y = (y.clamp(-1.0, 1.0) + 1.0) / 2.0;
    let deflection = (((pos_x - 0.5).powi(2) + (pos_y - 0.5).powi(2)).sqrt() * 8.0).min(1.0);

    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };

    if let Some(bg) = PathBuilder::from_oval(*rect) {
        paint.set_color(style.background);
        pixmap.fill_path(&bg, &paint, FillRule::Winding, Transform::identity(), None);
    }

    let ring_w = lerp(
        style.outline_width_unpressed,
        style.outline_width_pressed,
        deflection * 0.8,
    );
    
    if ring_w > 0.0 {
        let ring_color = if deflection > 0.05 {
            lerp_color(style.outline, style.glow, deflection * 0.8)
        } else {
            style.outline
        };
        
        let half = ring_w / 2.0;
        let ring_rect = Rect::from_xywh(rect.x() + half, rect.y() + half, w - ring_w, h - ring_w);
        
        if let Some(ring) = ring_rect.and_then(PathBuilder::from_oval) {
            paint.set_color(ring_color);
            let stroke = Stroke {
                width: ring_w,
                ..Default::default()
            };
            pixmap.stroke_path(&ring, &paint, &stroke, Transform::identity(), None);
        }
    }

    let dot_r = w.min(h) * 0.15;
    let dot_x = rect.x() + dot_r + pos_x * (w - dot_r * 2.0);
    let dot_y = rect.y() + dot_r + pos_y * (h - dot_r * 2.0);
    let dot_color = lerp_color(style.inactive_font_color, style.glow, deflection);

    if deflection > 0.02 {
        let glow_r = w.min(h) * 0.45;
        let mut inner = dot_color;
        inner.set_alpha(0.55 * deflection);
        let mut outer = dot_color;
        outer.set_alpha(0.0);
        let center = Point::from_xy(dot_x, dot_y);
        
        let shader = RadialGradient::new(
            center,
            center,
            glow_r,
            vec![GradientStop::new(0.0, inner), GradientStop::new(1.0, outer)],
            SpreadMode::Pad,
            Transform::identity(),
        );
        
        if let (Some(shader), Some(glow)) = (shader, PathBuilder::from_circle(dot_x, dot_y, glow_r))
        {
            let glow_paint = Paint {
                shader,
                anti_alias: true,
                ..Default::default()
            };
            pixmap.fill_path(
                &glow,
                &glow_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    if let Some(dot) = PathBuilder::from_circle(dot_x, dot_y, dot_r) {
        let mut color: Color = dot_color;
        color.set_alpha(0.95);
        paint.set_color(color);
        pixmap.fill_path(&dot, &paint, FillRule::Winding, Transform::identity(), None);
    }
}
