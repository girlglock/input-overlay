use std::collections::HashMap;
use std::io::Read;

use base64::Engine;
use flate2::read::ZlibDecoder;
use serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OverlaySettings {
    pub activecolor: String,
    pub inactivecolor: String,
    pub backgroundcolor: String,
    pub activebgcolor: String,
    pub outlinecolor: String,
    pub fontcolor: String,
    pub glowradius: f64,
    pub borderradius: f64,
    pub pressedradius: f64,
    pub pressscale: f64,
    pub animationspeed: f64,
    pub scale: f64,
    pub opacity: f64,
    pub fontfamily: String,
    pub boldfont: bool,
    pub hidemouse: bool,
    pub hidescrollcombo: bool,
    pub mousetrailsensitivity: f64,
    pub mousetrailfadeout: f64,
    pub mousetrailmode: String,
    pub mousetraillength: f64,
    pub mousetrailm1highlight: bool,
    pub mousepadtexture: String,
    pub mousepadtexturezoom: f64,
    pub mousepadtextureopacity: f64,
    pub showmousedistance: bool,
    pub mousedistancedpi: f64,
    pub resetmousedistanceafterfade: bool,
    pub gapmodifier: f64,
    pub outlinescalepressed: f64,
    pub outlinescaleunpressed: f64,
    pub analogdisplaymode: String,
    pub analogsmoothing: bool,
    pub keylegendmode: String,
    pub forcedisableanalog: bool,
    pub gamepaddeadzone: f64,
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
}

pub struct DecodedLink {
    pub settings: OverlaySettings,
    pub layout: Vec<LayoutElementDef>,
}

//configurator.js:182-207 applyDefaultSettings()
pub fn default_settings() -> OverlaySettings {
    let mut p: HashMap<String, String> = HashMap::new();
    for (k, v) in [
        ("activecolor", "#5cf67d"),
        ("inactivecolor", "#808080"),
        ("backgroundcolor", "#1a1a1ad1"),
        ("activebgcolor", "#47bd61"),
        ("outlinecolor", "#4f4f4f"),
        ("fontcolor", "#ffffff"),
        ("glowradius", "40"),
        ("borderradius", "10"),
        ("pressedradius", "10"),
        ("pressscale", "110"),
        ("animationspeed", "300"),
        ("fontfamily", "ArialPixel"),
        ("boldfont", "1"),
        ("gapmodifier", "100"),
        ("outlinescalepressed", "3"),
        ("outlinescaleunpressed", "2"),
        ("analogdisplaymode", "fill"),
        ("keylegendmode", "inverting"),
        ("gamepaddeadzone", "3"),
        ("mousetrailsensitivity", "100"),
        ("mousetrailfadeout", "600"),
        ("mousetrailmode", "wrap"),
        ("mousetraillength", "150"),
        ("mousetrailm1highlight", "1"),
        ("mousepadtexturezoom", "1"),
        ("mousepadtextureopacity", "1"),
        ("showmousedistance", "1"),
        ("mousedistancedpi", "400"),
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

pub fn decode_json_config(json: &str) -> Option<DecodedLink> {
    let value: Value = serde_json::from_str(json).ok()?;
    let obj = value.as_object()?;

    let mut params: HashMap<String, String> = HashMap::new();
    let mut layout = Vec::new();
    for (key, val) in obj {
        if key == "keyLayout" {
            if let Some(tuples) = val.as_array() {
                layout = tuples.iter().filter_map(parse_tuple).collect();
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

    Some(DecodedLink {
        settings: parse_overlay_settings(&params),
        layout,
    })
}

pub fn decode_link(link: &str) -> DecodedLink {
    let query = extract_query(link);
    let params = parse_query(&query);

    let (settings_params, key_layout_raw) =
        match params.get("cfg").and_then(|cfg| decode_base64url_zlib(cfg)) {
            Some(decompressed) => {
                let inner = parse_query(&decompressed);
                let kl = inner
                    .get("keyLayout")
                    .cloned()
                    .or_else(|| params.get("keyLayout").cloned());
                (inner, kl)
            }
            None => (params.clone(), params.get("keyLayout").cloned()),
        };

    let settings = parse_overlay_settings(&settings_params);
    let layout = key_layout_raw
        .map(|kl| decode_key_layout(&kl))
        .unwrap_or_default();

    DecodedLink { settings, layout }
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

fn decompress_tuples(s: &str) -> Option<Vec<Value>> {
    if s.is_empty() || s.starts_with('[') || s.starts_with('{') {
        return None;
    }
    let json_str = decode_base64url_zlib(s)?;
    serde_json::from_str(&json_str).ok()
}

fn decode_key_layout(raw: &str) -> Vec<LayoutElementDef> {
    let tuples = decompress_tuples(raw).or_else(|| serde_json::from_str(raw).ok());
    tuples
        .map(|t| t.iter().filter_map(parse_tuple).collect())
        .unwrap_or_default()
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

pub fn parse_tuple(tuple: &Value) -> Option<LayoutElementDef> {
    let full = tuple.as_array()?;
    if full.is_empty() {
        return None;
    }
    let mut arr = full.as_slice();
    let mut move_to_top = false;
    if arr.len() > 1 && arr.last().and_then(Value::as_str) == Some("top") {
        move_to_top = true;
        arr = &arr[..arr.len() - 1];
    }

    let type_ = value_to_string(&arr[0]);

    if type_ == "input_history" {
        let rest = &arr[1..];
        let cfg = rest.first().map(value_to_string).unwrap_or_default();
        let w = value_to_f64(rest.get(1), 1.0);
        let h = value_to_f64(rest.get(2), 1.0);
        let x = value_to_f64(rest.get(3), 0.0);
        let y = value_to_f64(rest.get(4), 0.0);

        let parts: Vec<&str> = cfg.split(';').collect();
        let tracked_keys = parts
            .first()
            .map(|s| {
                s.split(',')
                    .filter(|k| !k.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let vertical = parts.get(1) == Some(&"1");
        let scroll_speed = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(200.0);
        let highlight_overlap = parts.get(3) == Some(&"1");
        let reverse_direction = parts.get(4) == Some(&"1");

        return Some(LayoutElementDef {
            type_,
            w,
            h,
            x,
            y,
            tracked_keys,
            vertical,
            scroll_speed,
            highlight_overlap,
            reverse_direction,
            move_to_top,
            ..Default::default()
        });
    }

    let arity = label_arity(&type_);
    let labels: Vec<String> = arr[1..].iter().take(arity).map(value_to_string).collect();
    let dims = &arr[(1 + arity).min(arr.len())..];
    let w = value_to_f64(dims.first(), 1.0);
    let h = value_to_f64(dims.get(1), 1.0);
    let x = value_to_f64(dims.get(2), 0.0);
    let y = value_to_f64(dims.get(3), 0.0);

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
            Some(labels.first().cloned().unwrap_or_default())
        } else {
            None
        },
        labels: if arity > 1 { labels } else { Vec::new() },
        keys,
        move_to_top,
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

fn get_str(p: &HashMap<String, String>, key: &str, default: &str) -> String {
    p.get(key)
        .filter(|v| !v.is_empty())
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

fn get_bool_flag(p: &HashMap<String, String>, key: &str) -> bool {
    p.get(key).map(String::as_str) == Some("1")
}

fn get_f64(p: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    p.get(key)
        .filter(|v| !v.is_empty())
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

//urlManager.js buildURLParams renames some vars to save up on url chars
fn get_f64_aliased(p: &HashMap<String, String>, keys: &[&str], default: f64) -> f64 {
    keys.iter()
        .find_map(|key| {
            p.get(*key)
                .filter(|v| !v.is_empty())
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(default)
}

pub fn parse_overlay_settings(p: &HashMap<String, String>) -> OverlaySettings {
    let color = |key: &str, default: &str| {
        normalize_color_value(p.get(key).map(String::as_str).unwrap_or(""))
            .unwrap_or_else(|| default.to_string())
    };
    let radius = get_f64_aliased(p, &["borderradius", "radius"], 8.0);

    OverlaySettings {
        activecolor: color("activecolor", "#8b5cf6"),
        inactivecolor: color("inactivecolor", "#808080"),
        backgroundcolor: color("backgroundcolor", "#1a1a1ad1"),
        activebgcolor: color("activebgcolor", "#202020"),
        outlinecolor: color("outlinecolor", "#4f4f4f"),
        fontcolor: color("fontcolor", "#ffffff"),
        glowradius: get_f64_aliased(p, &["glowradius", "glow"], 24.0),
        borderradius: radius,
        pressedradius: get_f64(p, "pressedradius", radius),
        pressscale: get_f64(p, "pressscale", 105.0),
        animationspeed: get_f64_aliased(p, &["animationspeed", "speed"], 100.0),
        scale: get_f64(p, "scale", 100.0),
        opacity: get_f64(p, "opacity", 100.0),
        fontfamily: get_str(p, "fontfamily", ""),
        boldfont: get_bool_flag(p, "boldfont"),
        hidemouse: get_bool_flag(p, "hidemouse"),
        hidescrollcombo: get_bool_flag(p, "hidescrollcombo"),
        mousetrailsensitivity: get_f64(p, "mousetrailsensitivity", 100.0),
        mousetrailfadeout: get_f64(p, "mousetrailfadeout", 600.0),
        mousetrailmode: get_str(p, "mousetrailmode", "wrap"),
        mousetraillength: get_f64(p, "mousetraillength", 150.0),
        mousetrailm1highlight: get_bool_flag(p, "mousetrailm1highlight"),
        mousepadtexture: get_str(p, "mousepadtexture", ""),
        mousepadtexturezoom: get_f64(p, "mousepadtexturezoom", 1.0),
        mousepadtextureopacity: get_f64(p, "mousepadtextureopacity", 1.0),
        showmousedistance: get_bool_flag(p, "showmousedistance"),
        mousedistancedpi: get_f64(p, "mousedistancedpi", 400.0),
        resetmousedistanceafterfade: get_bool_flag(p, "resetmousedistanceafterfade"),
        gapmodifier: get_f64(p, "gapmodifier", 100.0),
        outlinescalepressed: get_f64(p, "outlinescalepressed", 2.0),
        outlinescaleunpressed: get_f64(p, "outlinescaleunpressed", 2.0),
        analogdisplaymode: get_str(p, "analogdisplaymode", "fill"),
        analogsmoothing: get_bool_flag(p, "analogsmoothing"),
        keylegendmode: get_str(p, "keylegendmode", "fading"),
        forcedisableanalog: get_bool_flag(p, "forcedisableanalog"),
        gamepaddeadzone: get_f64(p, "gamepaddeadzone", 3.0),
    }
}