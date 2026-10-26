mod analog;
mod blur;
mod color;
mod easing;
mod font_loader;
mod gif_texture;
mod history;
mod key_element;
mod label_image;
mod mouse_pad;
#[cfg(feature = "profiler")]
pub mod profiler;
mod scroller;
mod system_fonts;
mod text;
mod texture;
#[cfg(feature = "profiler")]
mod tiny_font;
mod transition;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use fontdue::Font;
use font_loader::LoadedFont;
use tiny_skia::{Color, Pixmap, Rect};

use crate::overlay_link::{LayoutElementDef, OverlaySettings, StyleOverride};
use analog::{AnalogSmoother, AnalogVisual, DisplayMode, LegendMode};
use color::parse_hex_color;
use history::HistoryState;
use io_ws_common::services::consts::{mouse_button_name, vk_to_key_name};
use io_ws_common::input_event::InputEvent;
use key_element::{draw_key_face, draw_key_glow, GlowCache, KeyStyle};
use label_image::LabelImageCache;
use mouse_pad::{MousePadConfig, MousePadState, PadMode};
use scroller::{ScrollDisplayDef, ScrollState};
use transition::Transition;

//overlayVisualiser.js:461-463/567-568
const UNIT: f32 = 50.0;

//index.html:648-678 setDynamicScale
const FIT_MAX_FILL: f32 = 0.85;

//overlayVisualiser.js:29
const MOUSEPAD_TRAIL_PX: f32 = 2.5;

struct PositionedElement {
    def: LayoutElementDef,
    rect: Rect,
}

#[derive(Clone, Copy)]
enum ElementKind {
    Key,
    MousePad,
    History(usize),
    Scroll(usize),
    Skip,
}

pub struct Renderer {
    elements: Vec<PositionedElement>,
    element_order: Vec<ElementKind>,
    has_overlap: Vec<bool>,
    content_bounds: Option<Rect>,
    active: HashSet<String>,
    element_styles: Vec<KeyStyle>,
    active_color: Color,
    font_color: Color,
    mouse_pad: Option<(MousePadState, Rect)>,
    mouse_sensitivity: f32,
    mouse_trail_length: f32,
    mouse_trail_fadeout_ms: f32,
    histories: Vec<(HistoryState, Rect)>,
    glow_cache: GlowCache,
    scroll_displays: Vec<(ScrollDisplayDef, Rect)>,
    scroll: ScrollState,
    scroll_hold_ms: f32,
    hide_scroll_count: bool,
    custom_font: Option<Font>,
    custom_font_real_bold: bool,
    font_rx: Option<Receiver<Option<LoadedFont>>>,
    bold_font: bool,
    label_images: LabelImageCache,
    key_transitions: Vec<Transition>,
    scroll_transitions: Vec<Transition>,
    anim_duration_secs: f32,
    analog_target: HashMap<String, f32>,
    analog_smoothers: Vec<AnalogSmoother>,
    analog_mode: bool,
    force_disable_analog: bool,
    analog_smoothing: bool,
    analog_display_mode: DisplayMode,
    analog_legend_mode: LegendMode,
}

fn merge_style_override(base: &KeyStyle, ov: Option<&StyleOverride>) -> KeyStyle {
    let Some(ov) = ov else {
        return *base;
    };
    let mut merged = *base;
    if let Some(c) = &ov.key_inactive_bg_color {
        merged.background = parse_hex_color(c, 255);
    }
    if let Some(c) = &ov.key_active_bg_color {
        merged.active_background = parse_hex_color(c, 255);
    }
    if let Some(c) = &ov.key_outline_color {
        merged.outline = parse_hex_color(c, 255);
    }
    if let Some(c) = &ov.key_glow_color {
        merged.active_outline = parse_hex_color(c, 255);
        merged.glow = parse_hex_color(c, 255);
    }
    if let Some(c) = &ov.key_active_font_color {
        merged.font_color = parse_hex_color(c, 255);
    }
    if let Some(c) = &ov.key_inactive_font_color {
        merged.inactive_font_color = parse_hex_color(c, 255);
    }
    if let Some(r) = ov.key_border_radius_idle {
        merged.border_radius = r as f32;
    }
    if let Some(r) = ov.key_border_radius_pressed {
        merged.pressed_radius = r as f32;
    }
    merged
}

impl Renderer {
    pub fn new(layout: &[LayoutElementDef], settings: &OverlaySettings) -> Self {
        let elements = layout_elements(layout);
        let content_bounds = union_bounds(&elements);

        let style = KeyStyle {
            background: parse_hex_color(&settings.key_inactive_bg_color, 255),
            active_background: parse_hex_color(&settings.key_active_bg_color, 255),
            outline: parse_hex_color(&settings.key_outline_color, 255),
            active_outline: parse_hex_color(&settings.key_glow_color, 255),
            glow: parse_hex_color(&settings.key_glow_color, 255),
            font_color: parse_hex_color(&settings.key_active_font_color, 255),
            inactive_font_color: parse_hex_color(&settings.key_inactive_font_color, 255),
            border_radius: settings.key_border_radius_idle as f32,
            pressed_radius: settings.key_border_radius_pressed as f32,
            outline_width_unpressed: settings.key_outline_width_idle as f32,
            outline_width_pressed: settings.key_outline_width_pressed as f32,
            press_scale: (settings.key_press_scale / 100.0) as f32,
            glow_radius: settings.key_glow_radius as f32,
            font_size: 16.0, //overlayVisualiser.js style.css:1580
            label_padding: 8.0,
        };

        let active_color = parse_hex_color(&settings.key_glow_color, 255);
        let mouse_pad_config = MousePadConfig {
            mode: if settings.mouse_trail_mode == "pan" {
                PadMode::Pan
            } else {
                PadMode::Wrap
            },
            m1_highlight: settings.mouse_trail_highlight_m1,
            show_distance: settings.mouse_show_distance,
            dpi: settings.mouse_distance_dpi as f32,
            reset_distance_after_fade: settings.mouse_reset_distance_after_fade,
            texture_url: settings.mouse_pad_texture_url.clone(),
            texture_zoom: settings.mouse_pad_texture_zoom as f32,
            texture_opacity: settings.mouse_pad_texture_opacity as f32,
        };

        let mut mouse_pad = None;
        let mut histories = Vec::new();
        let mut scroll_displays = Vec::new();
        let mut element_order = Vec::with_capacity(elements.len());
        for el in &elements {
            let base_type = el.def.type_.split('|').next().unwrap_or(&el.def.type_);
            let kind = match base_type {
                "mouse_pad" if mouse_pad.is_none() => {
                    let state = MousePadState::new(
                        el.rect.width(),
                        el.rect.height(),
                        &mouse_pad_config,
                        active_color,
                    );
                    mouse_pad = Some((state, el.rect));
                    ElementKind::MousePad
                }
                "input_history" => {
                    let history = HistoryState::new(
                        el.def.tracked_keys.clone(),
                        el.def.vertical,
                        el.def.scroll_speed as f32,
                        el.def.highlight_overlap,
                        el.def.reverse_direction,
                        el.def.perfect_gap_ms as f32,
                    );
                    let idx = histories.len();
                    histories.push((history, el.rect));
                    ElementKind::History(idx)
                }
                "scroller" => {
                    let idx = scroll_displays.len();
                    scroll_displays.push((ScrollDisplayDef::scroller(&el.def.labels), el.rect));
                    ElementKind::Scroll(idx)
                }
                "scroll_updown" => {
                    let idx = scroll_displays.len();
                    scroll_displays
                        .push((ScrollDisplayDef::scroll_updown(&el.def.labels), el.rect));
                    ElementKind::Scroll(idx)
                }
                "scroll_up" => {
                    let idx = scroll_displays.len();
                    scroll_displays.push((
                        ScrollDisplayDef::scroll_up(el.def.label.as_deref()),
                        el.rect,
                    ));
                    ElementKind::Scroll(idx)
                }
                "scroll_down" => {
                    let idx = scroll_displays.len();
                    scroll_displays.push((
                        ScrollDisplayDef::scroll_down(el.def.label.as_deref()),
                        el.rect,
                    ));
                    ElementKind::Scroll(idx)
                }
                _ if el.def.label.is_some() => ElementKind::Key,
                _ => ElementKind::Skip,
            };
            element_order.push(kind);
        }

        let element_styles: Vec<KeyStyle> = elements
            .iter()
            .map(|el| merge_style_override(&style, el.def.style_override.as_ref()))
            .collect();

        let has_overlap = compute_overlaps(&elements);
        let key_transitions = elements.iter().map(|_| Transition::new()).collect();
        let scroll_transitions = scroll_displays.iter().map(|_| Transition::new()).collect();
        let analog_smoothers = elements.iter().map(|_| AnalogSmoother::new()).collect();
        
        let anim_duration_secs = if settings.key_animation_speed <= 0.0 { //overlayVisualiser.js:154
            0.0
        } else {
            0.15 * (100.0 / settings.key_animation_speed as f32)
        };

        let mut label_images = LabelImageCache::new();
        for el in &elements {
            if let Some(label) = &el.def.label {
                if let Some(tag) = text::parse_img_tag(label) {
                    label_images.ensure_loading(tag.src);
                }
            }
        }
        for (def, _) in &scroll_displays {
            for label in def.all_labels() {
                if let Some(tag) = text::parse_img_tag(label) {
                    label_images.ensure_loading(tag.src);
                }
            }
        }

        let font_rx = font_loader::spawn_load(
            &settings.key_font_family,
            &settings.key_font_file_path,
            settings.key_bold_font,
        );

        Renderer {
            elements,
            element_order,
            has_overlap,
            content_bounds,
            active: HashSet::new(),
            element_styles,
            active_color,
            font_color: parse_hex_color(&settings.key_active_font_color, 255),
            mouse_pad,
            mouse_sensitivity: (settings.mouse_trail_sensitivity / 100.0) as f32,
            mouse_trail_length: settings.mouse_trail_length as f32,
            mouse_trail_fadeout_ms: settings.mouse_trail_fade_duration as f32,
            histories,
            glow_cache: GlowCache::new(),
            scroll_displays,
            scroll: ScrollState::new(),
            scroll_hold_ms: 250.0 * (100.0 / (settings.key_animation_speed as f32).max(1.0)), //overlayVisualiser.js:163
            hide_scroll_count: settings.scroll_hide_combo,
            custom_font: None,
            custom_font_real_bold: false,
            font_rx,
            bold_font: settings.key_bold_font,
            key_transitions,
            scroll_transitions,
            anim_duration_secs,
            analog_target: HashMap::new(),
            analog_smoothers,
            analog_mode: false,
            force_disable_analog: settings.analog_force_disable,
            analog_smoothing: settings.analog_smoothing,
            analog_display_mode: DisplayMode::parse(&settings.analog_display_mode),
            analog_legend_mode: LegendMode::parse(&settings.analog_legend_fill_mode),
            label_images,
        }
    }

    pub fn apply_event(&mut self, event: &InputEvent) -> bool {
        match event {
            InputEvent::KeyPress { rawcode, .. } => self.set_active(vk_to_key_name(*rawcode), true),
            InputEvent::KeyRelease { rawcode, .. } => {
                self.set_active(vk_to_key_name(*rawcode), false)
            }
            InputEvent::MouseButton {
                button, pressed, ..
            } => self.set_active(mouse_button_name(*button), *pressed),
            InputEvent::MouseMove { dx, dy, .. } => {
                let m1_active = self.active.contains("mouse_left");
                let Some((pad, _)) = self.mouse_pad.as_mut() else {
                    return false;
                };
                pad.handle_move(
                    *dx as f32,
                    *dy as f32,
                    self.mouse_sensitivity,
                    self.mouse_trail_length,
                    m1_active,
                );
                true
            }

            InputEvent::MouseScroll { rotation, .. } => {
                if self.scroll_displays.is_empty() {
                    return false;
                }
                self.scroll.handle_scroll(*rotation, self.scroll_hold_ms);
                true
            }

            InputEvent::AnalogDepth { rawcode, depth, .. } => {
                if self.force_disable_analog {
                    return false;
                }
                let Some(name) = vk_to_key_name(*rawcode) else {
                    return false;
                };
                self.analog_target.insert(name.to_string(), *depth);
                self.analog_mode = true;
                true
            }
        }
    }

    pub fn needs_constant_redraw(&self) -> bool {
        let mouse_pad_live = self
            .mouse_pad
            .as_ref()
            .is_some_and(|(pad, _)| pad.is_live(self.mouse_trail_fadeout_ms));
        let history_live = self.histories.iter().any(|(h, _)| h.is_live());
        let transitions_live = self.key_transitions.iter().any(Transition::is_animating)
            || self.scroll_transitions.iter().any(Transition::is_animating);
        let analog_live = self.analog_mode
            && self
                .analog_smoothers
                .iter()
                .any(AnalogSmoother::is_animating);
        mouse_pad_live
            || history_live
            || self.scroll.is_live()
            || self.font_rx.is_some()
            || self.label_images.any_loading()
            || transitions_live
            || analog_live
    }

    fn set_active(&mut self, name: Option<&'static str>, pressed: bool) -> bool {
        let Some(name) = name else {
            return false;
        };
        for (history, _) in self.histories.iter_mut() {
            if history.tracks(name) {
                history.record_edge(name, pressed);
            }
        }
        if pressed {
            self.active.insert(name.to_string())
        } else {
            self.active.remove(name)
        }
    }

    pub fn draw(
        &mut self,
        pixmap: &mut Pixmap,
        mut mark: impl FnMut(&'static str, Option<Duration>),
    ) {
        pixmap.fill(Color::TRANSPARENT);
        let (scale, tx, ty) = self.fit_transform(pixmap.width() as f32, pixmap.height() as f32);
        let styles: Vec<KeyStyle> = self.element_styles.iter().map(|s| s.scaled(scale)).collect();

        self.poll_font();
        self.label_images.poll();
        let primary_font = self
            .custom_font
            .as_ref()
            .or_else(|| text::load_system_font(self.bold_font));
        let font = primary_font.map(|primary| text::TextFont {
            primary,
            fallback: text::load_symbol_fallback_font(),
            bold: self.bold_font && !(self.custom_font.is_some() && self.custom_font_real_bold),
        });
        let font = font.as_ref();

        let mut key_ts: Vec<f32> = Vec::with_capacity(self.elements.len());
        for (i, el) in self.elements.iter().enumerate() {
            let active = self.is_active(el);
            self.key_transitions[i].set_active(active);
            key_ts.push(self.key_transitions[i].update(self.anim_duration_secs));
        }

        let mut key_analog: Vec<Option<AnalogVisual>> = Vec::with_capacity(self.elements.len());
        for (i, el) in self.elements.iter().enumerate() {
            if !self.analog_mode {
                key_analog.push(None);
                continue;
            }
            let keys: &[String] = if el.def.keys.is_empty() {
                std::slice::from_ref(&el.def.type_)
            } else {
                &el.def.keys
            };
            if !keys.iter().any(|k| k.starts_with("key_")) {
                key_analog.push(None);
                continue;
            }
            let explicit = keys
                .iter()
                .filter(|k| k.starts_with("key_"))
                .map(|k| self.analog_target.get(k.as_str()).copied().unwrap_or(0.0))
                .fold(0.0f32, f32::max);
            let target = if explicit > 0.0 {
                explicit
            } else if self.is_active(el) {
                1.0
            } else {
                0.0
            };
            let raw = self.analog_smoothers[i].update(target, self.analog_smoothing);
            key_analog.push(Some(AnalogVisual {
                depth: raw,
                effective_depth: analog::effective_depth(raw),
                display_mode: self.analog_display_mode,
                legend_mode: self.analog_legend_mode,
            }));
        }

        self.scroll.prune_expired();
        let mouse_middle_held = self.active.contains("mouse_middle");
        let mut scroll_key_held = vec![false; self.scroll_displays.len()];
        for (i, el) in self.elements.iter().enumerate() {
            if let ElementKind::Scroll(idx) = self.element_order[i] {
                scroll_key_held[idx] = el.def.keys.iter().skip(1).any(|k| self.active.contains(k));
            }
        }
        let mut scroll_ts: Vec<f32> = Vec::with_capacity(self.scroll_displays.len());
        for (i, (def, _)) in self.scroll_displays.iter().enumerate() {
            let active = self.scroll.is_active(def, mouse_middle_held) || scroll_key_held[i];
            self.scroll_transitions[i].set_active(active);
            scroll_ts.push(self.scroll_transitions[i].update(self.anim_duration_secs));
        }

        let mut keys_time = Duration::ZERO;
        let mut pad_time = Duration::ZERO;
        let mut hist_time = Duration::ZERO;
        let mut scroll_time = Duration::ZERO;
        let mut glow_time = Duration::ZERO;
        for (i, el) in self.elements.iter().enumerate() {
            match self.element_order[i] {
                ElementKind::Skip => {}
                ElementKind::Key => {
                    let t0 = Instant::now();
                    let t = key_ts[i];
                    let glowing = t > 0.0 && styles[i].glow_radius > 0.0;
                    if !glowing || self.has_overlap[i] {
                        let label = el.def.label.as_deref().unwrap_or_default();
                        let rect = fit_rect(&el.rect, scale, tx, ty);
                        if glowing {
                            draw_key_glow(
                                pixmap,
                                &rect,
                                &styles[i],
                                t,
                                key_analog[i],
                                &mut self.glow_cache,
                                &mut glow_time,
                            );
                        }
                        draw_key_face(
                            pixmap,
                            &rect,
                            &styles[i],
                            t,
                            label,
                            font,
                            key_analog[i],
                            scale,
                            &self.label_images,
                        );
                    }
                    keys_time += t0.elapsed();
                }
                ElementKind::MousePad => {
                    let t0 = Instant::now();
                    if let Some((pad, rect)) = self.mouse_pad.as_mut() {
                        let screen_rect = fit_rect(rect, scale, tx, ty);
                        let params = mouse_pad::MousePadDrawParams {
                            trail_px: MOUSEPAD_TRAIL_PX,
                            active_color: self.active_color,
                            font_color: self.font_color,
                            max_age_ms: self.mouse_trail_fadeout_ms,
                            background: styles[i].background,
                            outline: styles[i].outline,
                            border_radius: styles[i].border_radius,
                            border_width: styles[i].outline_width_unpressed,
                            font: font.copied(),
                        };
                        pad.draw(pixmap, &screen_rect, scale, &params);
                    }
                    pad_time += t0.elapsed();
                }
                ElementKind::History(idx) => {
                    let t0 = Instant::now();
                    let (history, rect) = &mut self.histories[idx];
                    let screen_rect = fit_rect(rect, scale, tx, ty);
                    let params = history::HistoryDrawParams {
                        outline_color: styles[i].outline,
                        active_color: styles[i].active_background,
                        highlight_color: styles[i].font_color,
                        perfect_color: styles[i].glow,
                        background: styles[i].background,
                        border_radius: styles[i].border_radius,
                        border_width: styles[i].outline_width_unpressed,
                    };
                    history.draw(pixmap, &screen_rect, scale, &params);
                    hist_time += t0.elapsed();
                }
                ElementKind::Scroll(idx) => {
                    let t0 = Instant::now();
                    let t = scroll_ts[idx];
                    let glowing = t > 0.0 && styles[i].glow_radius > 0.0;
                    if !glowing || self.has_overlap[i] {
                        let (def, rect) = &self.scroll_displays[idx];
                        let screen_rect = fit_rect(rect, scale, tx, ty);
                        if glowing {
                            draw_key_glow(
                                pixmap,
                                &screen_rect,
                                &styles[i],
                                t,
                                None,
                                &mut self.glow_cache,
                                &mut glow_time,
                            );
                        }
                        scroller::draw_scroll_display(
                            pixmap,
                            &screen_rect,
                            &styles[i],
                            def,
                            &self.scroll,
                            t,
                            self.hide_scroll_count,
                            scale,
                            font,
                            &self.label_images,
                        );
                    }
                    scroll_time += t0.elapsed();
                }
            }
        }
        mark("ELEMENTS", None);
        mark("KEYS", Some(keys_time));
        mark("PAD", Some(pad_time));
        mark("HIST", Some(hist_time));
        mark("SCROLL", Some(scroll_time));

        for (i, el) in self.elements.iter().enumerate() {
            if self.has_overlap[i] {
                continue;
            }
            match self.element_order[i] {
                ElementKind::Key => {
                    let t = key_ts[i];
                    if t > 0.0 && styles[i].glow_radius > 0.0 {
                        let label = el.def.label.as_deref().unwrap_or_default();
                        let rect = fit_rect(&el.rect, scale, tx, ty);
                        draw_key_glow(
                            pixmap,
                            &rect,
                            &styles[i],
                            t,
                            key_analog[i],
                            &mut self.glow_cache,
                            &mut glow_time,
                        );
                        draw_key_face(
                            pixmap,
                            &rect,
                            &styles[i],
                            t,
                            label,
                            font,
                            key_analog[i],
                            scale,
                            &self.label_images,
                        );
                    }
                }
                ElementKind::Scroll(idx) => {
                    let t = scroll_ts[idx];
                    if t > 0.0 && styles[i].glow_radius > 0.0 {
                        let (def, rect) = &self.scroll_displays[idx];
                        let screen_rect = fit_rect(rect, scale, tx, ty);
                        draw_key_glow(
                            pixmap,
                            &screen_rect,
                            &styles[i],
                            t,
                            None,
                            &mut self.glow_cache,
                            &mut glow_time,
                        );
                        scroller::draw_scroll_display(
                            pixmap,
                            &screen_rect,
                            &styles[i],
                            def,
                            &self.scroll,
                            t,
                            self.hide_scroll_count,
                            scale,
                            font,
                            &self.label_images,
                        );
                    }
                }
                ElementKind::MousePad | ElementKind::History(_) | ElementKind::Skip => {}
            }
        }
        mark("GLOWPASS", None);
        mark("GLOW", Some(glow_time));
    }

    fn poll_font(&mut self) {
        let Some(rx) = &self.font_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(loaded) => {
                self.custom_font_real_bold = loaded.as_ref().is_some_and(|l| l.real_bold);
                self.custom_font = loaded.map(|l| l.font);
                self.font_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => self.font_rx = None,
        }
    }

    fn is_active(&self, el: &PositionedElement) -> bool {
        if el.def.keys.is_empty() {
            self.active.contains(&el.def.type_)
        } else {
            el.def.keys.iter().any(|k| self.active.contains(k))
        }
    }

    fn fit_transform(&self, canvas_w: f32, canvas_h: f32) -> (f32, f32, f32) {
        let Some(bounds) = self.content_bounds else {
            return (1.0, 0.0, 0.0);
        };
        let total_w = bounds.width().max(1.0);
        let total_h = bounds.height().max(1.0);
        let scale = (canvas_w / total_w).min(canvas_h / total_h) * FIT_MAX_FILL;
        let cx = bounds.x() + total_w / 2.0;
        let cy = bounds.y() + total_h / 2.0;
        let tx = canvas_w / 2.0 - cx * scale;
        let ty = canvas_h / 2.0 - cy * scale;
        (scale, tx, ty)
    }
}

fn compute_overlaps(elements: &[PositionedElement]) -> Vec<bool> {
    let mut overlaps = vec![false; elements.len()];
    for i in 0..elements.len() {
        for j in (i + 1)..elements.len() {
            if rects_intersect(&elements[i].rect, &elements[j].rect) {
                overlaps[i] = true;
                overlaps[j] = true;
            }
        }
    }
    overlaps
}

fn rects_intersect(a: &Rect, b: &Rect) -> bool {
    a.left() < b.right() && b.left() < a.right() && a.top() < b.bottom() && b.top() < a.bottom()
}

fn layout_elements(defs: &[LayoutElementDef]) -> Vec<PositionedElement> {
    defs.iter()
        .map(|def| {
            let x = def.x as f32;
            let y = def.y as f32;
            let w = (def.w as f32 * UNIT).max(1.0);
            let h = (def.h as f32 * UNIT).max(1.0);
            let rect =
                Rect::from_xywh(x, y, w, h).unwrap_or(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap());
            PositionedElement {
                def: def.clone(),
                rect,
            }
        })
        .collect()
}

fn union_bounds(elements: &[PositionedElement]) -> Option<Rect> {
    let mut it = elements.iter();
    let first = it.next()?.rect;
    let (mut left, mut top, mut right, mut bottom) =
        (first.left(), first.top(), first.right(), first.bottom());
    for el in it {
        left = left.min(el.rect.left());
        top = top.min(el.rect.top());
        right = right.max(el.rect.right());
        bottom = bottom.max(el.rect.bottom());
    }
    Rect::from_ltrb(left, top, right, bottom)
}

fn fit_rect(rect: &Rect, scale: f32, tx: f32, ty: f32) -> Rect {
    Rect::from_xywh(
        rect.x() * scale + tx,
        rect.y() * scale + ty,
        rect.width() * scale,
        rect.height() * scale,
    )
    .unwrap_or(*rect)
}
