fn bezier_component(t: f32, p1: f32, p2: f32) -> f32 {
    let mt = 1.0 - t;
    3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t
}

fn bezier_derivative(t: f32, p1: f32, p2: f32) -> f32 {
    let mt = 1.0 - t;
    3.0 * mt * mt * p1 + 6.0 * mt * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

fn cubic_bezier(x: f32, p1: (f32, f32), p2: (f32, f32)) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let mut t = x;
    for _ in 0..6 {
        let x_t = bezier_component(t, p1.0, p2.0) - x;
        let dx_t = bezier_derivative(t, p1.0, p2.0);
        if dx_t.abs() < 1e-6 {
            break;
        }
        t -= x_t / dx_t;
        t = t.clamp(0.0, 1.0);
    }
    bezier_component(t, p1.1, p2.1)
}

pub fn ease_standard(x: f32) -> f32 {
    cubic_bezier(x, (0.4, 0.0), (0.2, 1.0))
}

pub fn ease_bounce(x: f32) -> f32 {
    cubic_bezier(x, (0.68, -0.55), (0.265, 1.55))
}
