use std::collections::HashMap;
use std::io::Read;

use base64::Engine;
use flate2::read::ZlibDecoder;
use serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OverlaySettings {
    pub key_glow_color: String,
    pub key_inactive_font_color: String,
    pub key_inactive_bg_color: String,
    pub key_active_bg_color: String,
    pub key_outline_color: String,
    pub key_active_font_color: String,
    pub key_glow_radius: f64,
    pub key_border_radius_idle: f64,
    pub key_border_radius_pressed: f64,
    pub key_press_scale: f64,
    pub key_animation_speed: f64,
    pub scale: f64,
    pub opacity: f64,
    pub key_font_family: String,
    pub key_font_file_path: String,
    pub key_bold_font: bool,
    pub hide_mouse: bool,
    pub scroll_hide_combo: bool,
    pub mouse_trail_sensitivity: f64,
    pub mouse_trail_fade_duration: f64,
    pub mouse_trail_mode: String,
    pub mouse_trail_length: f64,
    pub mouse_trail_highlight_m1: bool,
    pub mouse_pad_texture_url: String,
    pub mouse_pad_texture_zoom: f64,
    pub mouse_pad_texture_opacity: f64,
    pub mouse_show_distance: bool,
    pub mouse_distance_dpi: f64,
    pub mouse_reset_distance_after_fade: bool,
    pub key_gap_scale: f64,
    pub key_outline_width_pressed: f64,
    pub key_outline_width_idle: f64,
    pub analog_display_mode: String,
    pub analog_smoothing: bool,
    pub analog_legend_fill_mode: String,
    pub analog_force_disable: bool,
    pub gamepad_deadzone: f64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct StyleOverride {
    pub key_inactive_bg_color: Option<String>,
    pub key_active_bg_color: Option<String>,
    pub key_outline_color: Option<String>,
    pub key_glow_color: Option<String>,
    pub key_active_font_color: Option<String>,
    pub key_inactive_font_color: Option<String>,
    pub key_border_radius_idle: Option<f64>,
    pub key_border_radius_pressed: Option<f64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct LayoutElementDef {
    pub type_: String,
    pub w: f64,
    pub h: f64,
    pub x: f64,
    pub y: f64,
    pub label: Option<String>,
    pub labels: Vec<String>,
    pub keys: Vec<String>,
    pub move_to_top: bool,
    pub tracked_keys: Vec<String>,
    pub vertical: bool,
    pub scroll_speed: f64,
    pub highlight_overlap: bool,
    pub reverse_direction: bool,
    pub perfect_gap_ms: f64,
    pub style_override: Option<StyleOverride>,
}

pub struct DecodedLink {
    pub settings: OverlaySettings,
    pub layout: Vec<LayoutElementDef>,
}

//configurator.js:182-207 applyDefaultSettings()
pub fn default_settings() -> OverlaySettings {
    let mut p: HashMap<String, String> = HashMap::new();
    for (k, v) in [
        ("key_glow_color", "#5cf67d"),
        ("key_inactive_font_color", "#808080"),
        ("key_inactive_bg_color", "#1a1a1ad1"),
        ("key_active_bg_color", "#47bd61"),
        ("key_outline_color", "#4f4f4f"),
        ("key_active_font_color", "#ffffff"),
        ("key_glow_radius", "40"),
        ("key_border_radius_idle", "10"),
        ("key_border_radius_pressed", "10"),
        ("key_press_scale", "110"),
        ("key_animation_speed", "300"),
        ("key_font_family", "ArialPixel"),
        ("key_bold_font", "1"),
        ("key_gap_scale", "100"),
        ("key_outline_width_pressed", "3"),
        ("key_outline_width_idle", "2"),
        ("analog_display_mode", "fill"),
        ("analog_legend_fill_mode", "inverting"),
        ("gamepad_deadzone", "3"),
        ("mouse_trail_sensitivity", "100"),
        ("mouse_trail_fade_duration", "600"),
        ("mouse_trail_mode", "wrap"),
        ("mouse_trail_length", "150"),
        ("mouse_trail_highlight_m1", "1"),
        ("mouse_pad_texture_zoom", "1"),
        ("mouse_pad_texture_opacity", "1"),
        ("mouse_show_distance", "1"),
        ("mouse_distance_dpi", "400"),
    ] {
        p.insert(k.to_string(), v.to_string());
    }
    parse_overlay_settings(&p)
}

pub fn default_layout() -> Vec<LayoutElementDef> {
    let key = |type_: &str, label: &str, w: f64, h: f64, x: f64, y: f64| LayoutElementDef {
        type_: type_.to_string(),
        w,
        h,
        x,
        y,
        label: Some(label.to_string()),
        ..Default::default()
    };

    vec![
        key("key_1", "1", 1.0, 1.0, 56.25, 0.0),
        key("key_2", "2", 1.0, 1.0, 112.5, 0.0),
        key("key_3", "3", 1.0, 1.0, 168.75, 0.0),
        key("key_4", "4", 1.0, 1.0, 225.0, 0.0),
        key("key_tab", "TAB", 1.5, 1.0, 0.0, 56.25),
        key("key_q", "Q", 1.0, 1.0, 81.25, 56.25),
        key("key_w", "W", 1.0, 1.0, 137.5, 56.25),
        key("key_e", "E", 1.0, 1.0, 193.75, 56.25),
        key("key_r", "R", 1.0, 1.0, 250.0, 56.25),
        key("key_leftshift", "SHIFT", 2.0, 1.0, 0.0, 112.5),
        key("key_a", "A", 1.0, 1.0, 106.25, 112.5),
        key("key_s", "S", 1.0, 1.0, 162.5, 112.5),
        key("key_d", "D", 1.0, 1.0, 218.75, 112.5),
        key("key_f", "F", 1.0, 1.0, 275.0, 112.5),
        key("key_leftctrl", "CTRL", 1.5, 1.0, 0.0, 168.75),
        key("key_leftalt", "ALT", 1.5, 1.0, 81.25, 168.75),
        key("key_space", "SPACE", 3.25, 1.0, 162.5, 168.75),
        LayoutElementDef {
            type_: "mouse_pad".to_string(),
            w: 5.0,
            h: 3.63,
            x: 331.25,
            y: 37.5,
            ..Default::default()
        },
        key("mouse_left", "M1", 1.625, 0.63, 331.25, 0.0),
        key("mouse_right", "M2", 1.625, 0.63, 500.0, 0.0),
        LayoutElementDef {
            type_: "scroller".to_string(),
            w: 1.5,
            h: 0.63,
            x: 418.75,
            y: 0.0,
            labels: vec!["-".to_string(), "🡅".to_string(), "🡇".to_string()],
            ..Default::default()
        },
        LayoutElementDef {
            type_: "input_history".to_string(),
            w: 1.0,
            h: 3.25,
            x: -56.25,
            y: 56.25,
            tracked_keys: vec!["key_a".to_string(), "key_d".to_string()],
            vertical: true,
            scroll_speed: 100.0,
            highlight_overlap: true,
            reverse_direction: true,
            ..Default::default()
        },
    ]
}

const SETTING_KEYS: &[&str] = &[
    "key_glow_color",
    "key_inactive_font_color",
    "key_inactive_bg_color",
    "key_active_bg_color",
    "key_outline_color",
    "key_active_font_color",
    "key_glow_radius",
    "key_border_radius_idle",
    "key_border_radius_pressed",
    "key_press_scale",
    "key_animation_speed",
    "scale",
    "opacity",
    "key_font_family",
    "key_font_file_path",
    "key_bold_font",
    "hide_mouse",
    "scroll_hide_combo",
    "mouse_trail_sensitivity",
    "mouse_trail_fade_duration",
    "mouse_trail_mode",
    "mouse_trail_length",
    "mouse_trail_highlight_m1",
    "mouse_pad_texture_url",
    "mouse_pad_texture_zoom",
    "mouse_pad_texture_opacity",
    "mouse_show_distance",
    "mouse_distance_dpi",
    "mouse_reset_distance_after_fade",
    "key_gap_scale",
    "key_outline_width_pressed",
    "key_outline_width_idle",
    "analog_display_mode",
    "analog_smoothing",
    "analog_legend_fill_mode",
    "analog_force_disable",
    "gamepad_deadzone",
];

const IGNORED_KEYS: &[&str] = &["keyLayout", "wsauth", "wsaddress", "wsport", "gamepad_no_cef"];

fn warn_unknown_keys<'a>(keys: impl Iterator<Item = &'a String>) {
    for key in keys {
        if !SETTING_KEYS.contains(&key.as_str()) && !IGNORED_KEYS.contains(&key.as_str()) {
            tracing::warn!("config: unknown setting '{key}' ignored");
        }
    }
}

fn parse_layout(elements: &[Value]) -> Vec<LayoutElementDef> {
    let gamepad_only = elements
        .iter()
        .filter(|v| {
            v.get("type")
                .map(value_to_string)
                .is_some_and(|t| !t.is_empty() && t.split('|').all(is_gamepad_type))
        })
        .count();
    let layout: Vec<LayoutElementDef> = elements.iter().filter_map(parse_layout_element).collect();
    if gamepad_only > 0 {
        tracing::info!("config: {gamepad_only} gamepad elements ignored, gamepads arent supported in the plugin yet");
    }
    if layout.len() + gamepad_only < elements.len() {
        tracing::warn!(
            "config: {} keyLayout entries skipped - elements must be {{\"type\": ...}} objects",
            elements.len() - layout.len() - gamepad_only
        );
    }
    layout
}

fn is_gamepad_type(part: &str) -> bool {
    part.starts_with("gp_")
}

fn strip_gamepad_types(type_: &str) -> String {
    type_
        .split('|')
        .filter(|p| !is_gamepad_type(p))
        .collect::<Vec<_>>()
        .join("|")
}

pub fn decode_json_config(json: &str) -> Option<DecodedLink> {
    let value: Value = serde_json::from_str(json).ok()?;
    let obj = value.as_object()?;

    let mut params: HashMap<String, String> = HashMap::new();
    let mut layout = Vec::new();
    for (key, val) in obj {
        if key == "keyLayout" {
            if let Some(elements) = val.as_array() {
                layout = parse_layout(elements);
            }
            continue;
        }
        let s = match val {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => {
                if *b {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            _ => continue,
        };
        params.insert(key.clone(), s);
    }
    warn_unknown_keys(obj.keys());

    Some(DecodedLink {
        settings: parse_overlay_settings(&params),
        layout,
    })
}

//links are always ?cfg=<base64url zlib of the settings query> with keyLayout inside it as plain JSON
pub fn decode_link(link: &str) -> DecodedLink {
    let query = extract_query(link);
    let params = parse_query(&query);

    let Some(inner) = params
        .get("cfg")
        .and_then(|cfg| decode_base64url_zlib(cfg))
        .map(|d| parse_query(&d))
    else {
        tracing::warn!("config: link has no valid cfg= parameter, using defaults");
        return DecodedLink {
            settings: parse_overlay_settings(&HashMap::new()),
            layout: Vec::new(),
        };
    };
    warn_unknown_keys(inner.keys());

    let layout = inner
        .get("keyLayout")
        .and_then(|kl| serde_json::from_str::<Vec<Value>>(kl).ok())
        .map(|elements| parse_layout(&elements))
        .unwrap_or_default();

    DecodedLink {
        settings: parse_overlay_settings(&inner),
        layout,
    }
}

fn extract_query(link: &str) -> String {
    let after_q = link.split_once('?').map(|(_, q)| q).unwrap_or(link);
    after_q.split('#').next().unwrap_or("").to_string()
}

fn parse_query(s: &str) -> HashMap<String, String> {
    let s = s.strip_prefix('?').unwrap_or(s);
    let mut map = HashMap::new();
    for pair in s.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("");
        let v = it.next().unwrap_or("");
        map.insert(percent_decode(k), percent_decode(v));
    }
    map
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn decode_base64url_zlib(s: &str) -> Option<String> {
    let normalized: String = s
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            c => c,
        })
        .collect();
    let rem = normalized.len() % 4;
    let padded = if rem == 0 {
        normalized
    } else {
        normalized + &"=".repeat(4 - rem)
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(padded.as_bytes())
        .ok()?;
    let mut decoder = ZlibDecoder::new(&bytes[..]);
    let mut out = String::new();
    decoder.read_to_string(&mut out).ok()?;
    Some(out)
}

pub fn parse_layout_element(value: &Value) -> Option<LayoutElementDef> {
    parse_element_object(value.as_object()?)
}

const SCROLL_TYPES: [&str; 4] = ["scroller", "scroll_updown", "scroll_up", "scroll_down"];

fn normalize_pipe_type(type_: &str) -> String {
    if !type_.contains('|') {
        return type_.to_string();
    }
    let parts: Vec<&str> = type_.split('|').collect();
    match parts.iter().position(|p| SCROLL_TYPES.contains(p)) {
        Some(idx) if idx > 0 => std::iter::once(parts[idx])
            .chain(parts.iter().enumerate().filter(|&(i, _)| i != idx).map(|(_, p)| *p))
            .collect::<Vec<_>>()
            .join("|"),
        _ => type_.to_string(),
    }
}

fn label_arity(type_: &str) -> usize {
    let base = type_.split('|').next().unwrap_or(type_);
    match base {
        "scroller" => 3,
        "scroll_updown" => 2,
        "scroll_up" | "scroll_down" => 1,
        "mouse_side" => 2,
        "mouse_pad" | "gp_joystick_ls" | "gp_joystick_rs" | "input_history" | "$" => 0,
        _ => 1,
    }
}

fn value_to_f64(v: Option<&Value>, default: f64) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(default),
        Some(Value::String(s)) => s.parse().unwrap_or(default),
        _ => default,
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn value_to_bool(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Bool(true)))
}

fn value_to_string_array(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|arr| arr.iter().map(value_to_string).collect())
        .unwrap_or_default()
}

fn value_to_f64_opt(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn parse_style_override(obj: &serde_json::Map<String, Value>) -> Option<StyleOverride> {
    let style = obj.get("style")?.as_object()?;
    let get_color = |key: &str| style.get(key).map(value_to_string).filter(|s| !s.is_empty());

    let override_ = StyleOverride {
        key_inactive_bg_color: get_color("key_inactive_bg_color"),
        key_active_bg_color: get_color("key_active_bg_color"),
        key_outline_color: get_color("key_outline_color"),
        key_glow_color: get_color("key_glow_color"),
        key_active_font_color: get_color("key_active_font_color"),
        key_inactive_font_color: get_color("key_inactive_font_color"),
        key_border_radius_idle: value_to_f64_opt(style.get("key_border_radius_idle")),
        key_border_radius_pressed: value_to_f64_opt(style.get("key_border_radius_pressed")),
    };

    let is_empty = override_.key_inactive_bg_color.is_none()
        && override_.key_active_bg_color.is_none()
        && override_.key_outline_color.is_none()
        && override_.key_glow_color.is_none()
        && override_.key_active_font_color.is_none()
        && override_.key_inactive_font_color.is_none()
        && override_.key_border_radius_idle.is_none()
        && override_.key_border_radius_pressed.is_none();

    if is_empty {
        None
    } else {
        Some(override_)
    }
}

pub fn parse_element_object(obj: &serde_json::Map<String, Value>) -> Option<LayoutElementDef> {
    let type_ = normalize_pipe_type(&strip_gamepad_types(
        &obj.get("type").map(value_to_string).unwrap_or_default(),
    ));
    if type_.is_empty() {
        return None;
    }

    let w = value_to_f64(obj.get("w"), 1.0);
    let h = value_to_f64(obj.get("h"), 1.0);
    let x = value_to_f64(obj.get("x"), 0.0);
    let y = value_to_f64(obj.get("y"), 0.0);
    let move_to_top = value_to_bool(obj.get("moveToTop"));
    let style_override = parse_style_override(obj);

    if type_ == "input_history" {
        return Some(LayoutElementDef {
            type_,
            w,
            h,
            x,
            y,
            tracked_keys: value_to_string_array(obj.get("trackedKeys"))
                .into_iter()
                .filter(|k| !is_gamepad_type(k))
                .collect(),
            vertical: value_to_bool(obj.get("vertical")),
            scroll_speed: obj
                .get("scrollSpeed")
                .map(|v| value_to_f64(Some(v), 200.0))
                .unwrap_or(200.0),
            highlight_overlap: value_to_bool(obj.get("highlightOverlap")),
            reverse_direction: value_to_bool(obj.get("reverseDirection")),
            perfect_gap_ms: value_to_f64(obj.get("perfectGapMs"), 0.0),
            move_to_top,
            style_override,
            ..Default::default()
        });
    }

    let arity = label_arity(&type_);
    let keys = if type_.contains('|') {
        type_.split('|').map(String::from).collect()
    } else {
        Vec::new()
    };

    Some(LayoutElementDef {
        type_,
        w,
        h,
        x,
        y,
        label: if arity == 1 {
            Some(obj.get("label").map(value_to_string).unwrap_or_default())
        } else {
            None
        },
        labels: if arity > 1 {
            value_to_string_array(obj.get("labels"))
        } else {
            Vec::new()
        },
        keys,
        move_to_top,
        style_override,
        ..Default::default()
    })
}

fn normalize_color_value(v: &str) -> Option<String> {
    if v.is_empty() {
        return None;
    }
    if let Some(rest) = v.strip_prefix('#') {
        return Some(format!("#{}", rest.to_lowercase()));
    }
    if let Some(rest) = v.strip_prefix("%23") {
        return Some(format!("#{}", rest.to_lowercase()));
    }
    Some(format!("#{}", v.to_lowercase()))
}

fn get_f64(p: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    p.get(key)
        .filter(|v| !v.is_empty())
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn get_bool(p: &HashMap<String, String>, key: &str) -> bool {
    p.get(key).is_some_and(|v| v == "1")
}

fn get_str(p: &HashMap<String, String>, key: &str, default: &str) -> String {
    p.get(key)
        .filter(|v| !v.is_empty())
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

fn get_color(p: &HashMap<String, String>, key: &str, default: &str) -> String {
    p.get(key)
        .and_then(|v| normalize_color_value(v))
        .unwrap_or_else(|| default.to_string())
}

pub fn parse_overlay_settings(p: &HashMap<String, String>) -> OverlaySettings {
    let radius = get_f64(p, "key_border_radius_idle", 8.0);

    OverlaySettings {
        key_glow_color: get_color(p, "key_glow_color", "#8b5cf6"),
        key_inactive_font_color: get_color(p, "key_inactive_font_color", "#808080"),
        key_inactive_bg_color: get_color(p, "key_inactive_bg_color", "#1a1a1ad1"),
        key_active_bg_color: get_color(p, "key_active_bg_color", "#202020"),
        key_outline_color: get_color(p, "key_outline_color", "#4f4f4f"),
        key_active_font_color: get_color(p, "key_active_font_color", "#ffffff"),
        key_glow_radius: get_f64(p, "key_glow_radius", 24.0),
        key_border_radius_idle: radius,
        key_border_radius_pressed: get_f64(p, "key_border_radius_pressed", radius),
        key_press_scale: get_f64(p, "key_press_scale", 105.0),
        key_animation_speed: get_f64(p, "key_animation_speed", 100.0),
        scale: get_f64(p, "scale", 100.0),
        opacity: get_f64(p, "opacity", 100.0),
        key_font_family: get_str(p, "key_font_family", ""),
        key_font_file_path: get_str(p, "key_font_file_path", ""),
        key_bold_font: get_bool(p, "key_bold_font"),
        hide_mouse: get_bool(p, "hide_mouse"),
        scroll_hide_combo: get_bool(p, "scroll_hide_combo"),
        mouse_trail_sensitivity: get_f64(p, "mouse_trail_sensitivity", 100.0),
        mouse_trail_fade_duration: get_f64(p, "mouse_trail_fade_duration", 600.0),
        mouse_trail_mode: get_str(p, "mouse_trail_mode", "wrap"),
        mouse_trail_length: get_f64(p, "mouse_trail_length", 150.0),
        mouse_trail_highlight_m1: get_bool(p, "mouse_trail_highlight_m1"),
        mouse_pad_texture_url: get_str(p, "mouse_pad_texture_url", ""),
        mouse_pad_texture_zoom: get_f64(p, "mouse_pad_texture_zoom", 1.0),
        mouse_pad_texture_opacity: get_f64(p, "mouse_pad_texture_opacity", 1.0),
        mouse_show_distance: get_bool(p, "mouse_show_distance"),
        mouse_distance_dpi: get_f64(p, "mouse_distance_dpi", 400.0),
        mouse_reset_distance_after_fade: get_bool(p, "mouse_reset_distance_after_fade"),
        key_gap_scale: get_f64(p, "key_gap_scale", 100.0),
        key_outline_width_pressed: get_f64(p, "key_outline_width_pressed", 2.0),
        key_outline_width_idle: get_f64(p, "key_outline_width_idle", 2.0),
        analog_display_mode: get_str(p, "analog_display_mode", "fill"),
        analog_smoothing: get_bool(p, "analog_smoothing"),
        analog_legend_fill_mode: get_str(p, "analog_legend_fill_mode", "fading"),
        analog_force_disable: get_bool(p, "analog_force_disable"),
        gamepad_deadzone: get_f64(p, "gamepad_deadzone", 3.0),
    }
}