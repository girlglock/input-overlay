use std::time::Instant;
const SNAP: f32 = 0.001;
const DEPTH_THRESHOLD: f32 = 0.01;
const LERP_PRESS: f32 = 0.35;
const LERP_RELEASE: f32 = 0.7;
const FRAME_REF_MS: f32 = 16.667;

//overlayVisualiser.js:840-878 _analogRafLoop
pub struct AnalogSmoother {
    current: f32,
    target: f32,
    last_update: Instant,
}

impl AnalogSmoother {
    pub fn new() -> Self {
        AnalogSmoother {
            current: 0.0,
            target: 0.0,
            last_update: Instant::now(),
        }
    }

    pub fn update(&mut self, target: f32, smoothing: bool) -> f32 {
        let now = Instant::now();
        let dt_ms = now.duration_since(self.last_update).as_secs_f32() * 1000.0;
        self.last_update = now;
        self.target = target;

        if !smoothing {
            self.current = target;
            return self.current;
        }

        let delta = target - self.current;
        if delta.abs() < SNAP {
            self.current = target;
        } else {
            let k = if delta < 0.0 {
                LERP_RELEASE
            } else {
                LERP_PRESS
            };
            let alpha = 1.0 - (1.0 - k).powf(dt_ms / FRAME_REF_MS);
            self.current += delta * alpha;
        }
        self.current
    }

    pub fn is_animating(&self) -> bool {
        (self.current - self.target).abs() >= SNAP
    }
}

pub fn effective_depth(raw: f32) -> f32 {
    if raw < DEPTH_THRESHOLD {
        0.0
    } else {
        raw
    }
}

//overlayVisualiser.js:892-893 analogdisplaymode
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Fill,
    Percent,
    Both,
}

impl DisplayMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "percent" => DisplayMode::Percent,
            "both" => DisplayMode::Both,
            _ => DisplayMode::Fill,
        }
    }

    pub fn show_fill(self) -> bool {
        self != DisplayMode::Percent
    }

    pub fn show_percent(self) -> bool {
        matches!(self, DisplayMode::Percent | DisplayMode::Both)
    }
}

//overlayVisualiser.js:950-962 keylegendmode
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LegendMode {
    Fading,
    Inverting,
    Digital,
}

impl LegendMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "inverting" => LegendMode::Inverting,
            "digital" => LegendMode::Digital,
            _ => LegendMode::Fading,
        }
    }
}

#[derive(Clone, Copy)]
pub struct AnalogVisual {
    pub depth: f32,
    pub effective_depth: f32,
    pub display_mode: DisplayMode,
    pub legend_mode: LegendMode,
}
