use tiny_skia::Color;

//utils.js hexToRgba
pub fn parse_hex_color(s: &str, fallback_alpha: u8) -> Color {
    let s = s.trim_start_matches('#');
    let byte = |range: std::ops::Range<usize>| -> u8 {
        s.get(range)
            .and_then(|h| u8::from_str_radix(h, 16).ok())
            .unwrap_or(0)
    };
    let r = byte(0..2);
    let g = byte(2..4);
    let b = byte(4..6);
    let a = if s.len() >= 8 {
        byte(6..8)
    } else {
        fallback_alpha
    };
    Color::from_rgba8(r, g, b, a)
}
