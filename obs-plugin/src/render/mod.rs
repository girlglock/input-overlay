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
mod text;
mod texture;
#[cfg(feature = "profiler")]
mod tiny_font;
mod transition;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use fontdue::{Font, FontSettings};
use tiny_skia::{Color, Pixmap, Rect};

use crate::overlay_link::{LayoutElementDef, OverlaySettings};
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
    style: KeyStyle,
    active_color: Color,
    outline_color: Color,
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
    font_rx: Option<Receiver<Option<Vec<u8>>>>,
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

impl Renderer {
    pub fn new(layout: &[LayoutElementDef], settings: &OverlaySettings) -> Self {
        let elements = layout_elements(layout);
        let content_bounds = union_bounds(&elements);

        let style = KeyStyle {
            background: parse_hex_color(&settings.backgroundcolor, 255),
            active_background: parse_hex_color(&settings.activebgcolor, 255),
            outline: parse_hex_color(&settings.outlinecolor, 255),
            active_outline: parse_hex_color(&settings.activecolor, 255),
            glow: parse_hex_color(&settings.activecolor, 255),
            font_color: parse_hex_color(&settings.fontcolor, 255),
            inactive_font_color: parse_hex_color(&settings.inactivecolor, 255),
            border_radius: settings.borderradius as f32,
            pressed_radius: settings.pressedradius as f32,
            outline_width_unpressed: settings.outlinescaleunpressed as f32,
            outline_width_pressed: settings.outlinescalepressed as f32,
            press_scale: (settings.pressscale / 100.0) as f32,
            glow_radius: settings.glowradius as f32,
            font_size: 16.0, //overlayVisualiser.js style.css:1580
            label_padding: 8.0,
        };

        let active_color = parse_hex_color(&settings.activecolor, 255);
        let mouse_pad_config = MousePadConfig {
            mode: if settings.mousetrailmode == "pan" {
                PadMode::Pan
            } else {
                PadMode::Wrap
            },
            m1_highlight: settings.mousetrailm1highlight,
            show_distance: settings.showmousedistance,
            dpi: settings.mousedistancedpi as f32,
            reset_distance_after_fade: settings.resetmousedistanceafterfade,
            texture_url: settings.mousepadtexture.clone(),
            texture_zoom: settings.mousepadtexturezoom as f32,
            texture_opacity: settings.mousepadtextureopacity as f32,
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

        let has_overlap = compute_overlaps(&elements);
        let key_transitions = elements.iter().map(|_| Transition::new()).collect();
        let scroll_transitions = scroll_displays.iter().map(|_| Transition::new()).collect();
        let analog_smoothers = elements.iter().map(|_| AnalogSmoother::new()).collect();
        
        let anim_duration_secs = if settings.animationspeed <= 0.0 { //overlayVisualiser.js:154
            0.0
        } else {
            0.15 * (100.0 / settings.animationspeed as f32)
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

        Renderer {
            elements,
            element_order,
            has_overlap,
            content_bounds,
            active: HashSet::new(),
            style,
            active_color,
            outline_color: parse_hex_color(&settings.outlinecolor, 255),
            font_color: parse_hex_color(&settings.fontcolor, 255),
            mouse_pad,
            mouse_sensitivity: (settings.mousetrailsensitivity / 100.0) as f32,
            mouse_trail_length: settings.mousetraillength as f32,
            mouse_trail_fadeout_ms: settings.mousetrailfadeout as f32,
            histories,
            glow_cache: GlowCache::new(),
            scroll_displays,
            scroll: ScrollState::new(),
            scroll_hold_ms: 250.0 * (100.0 / (settings.animationspeed as f32).max(1.0)), //overlayVisualiser.js:163
            hide_scroll_count: settings.hidescrollcombo,
            custom_font: None,
            font_rx: font_loader::spawn_load(&settings.fontfamily, settings.boldfont),
            bold_font: settings.boldfont,
            key_transitions,
            scroll_transitions,
            anim_duration_secs,
            analog_target: HashMap::new(),
            analog_smoothers,
            analog_mode: false,
            force_disable_analog: settings.forcedisableanalog,
            analog_smoothing: settings.analogsmoothing,
            analog_display_mode: DisplayMode::parse(&settings.analogdisplaymode),
            analog_legend_mode: LegendMode::parse(&settings.keylegendmode),
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
        let style = self.style.scaled(scale);

        self.poll_font();
        self.label_images.poll();
        let primary_font = self
            .custom_font
            .as_ref()
            .or_else(|| text::load_system_font(self.bold_font));
        let font = primary_font.map(|primary| text::TextFont {
            primary,
            fallback: text::load_symbol_fallback_font(),
            bold: self.bold_font,
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
        let mut scroll_ts: Vec<f32> = Vec::with_capacity(self.scroll_displays.len());
        for (i, (def, _)) in self.scroll_displays.iter().enumerate() {
            let active = self.scroll.is_active(def, mouse_middle_held);
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
                    let glowing = t > 0.0 && style.glow_radius > 0.0;
                    if !glowing || self.has_overlap[i] {
                        let label = el.def.label.as_deref().unwrap_or_default();
                        let rect = fit_rect(&el.rect, scale, tx, ty);
                        if glowing {
                            draw_key_glow(
                                pixmap,
                                &rect,
                                &style,
                                t,
                                key_analog[i],
                                &mut self.glow_cache,
                                &mut glow_time,
                            );
                        }
                        draw_key_face(
                            pixmap,
                            &rect,
                            &style,
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
                            background: style.background,
                            outline: style.outline,
                            border_radius: style.border_radius,
                            border_width: style.outline_width_unpressed,
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
                        outline_color: self.outline_color,
                        active_color: self.active_color,
                        highlight_color: self.font_color,
                        background: style.background,
                        border_radius: style.border_radius,
                        border_width: style.outline_width_unpressed,
                    };
                    history.draw(pixmap, &screen_rect, scale, &params);
                    hist_time += t0.elapsed();
                }
                ElementKind::Scroll(idx) => {
                    let t0 = Instant::now();
                    let t = scroll_ts[idx];
                    let glowing = t > 0.0 && style.glow_radius > 0.0;
                    if !glowing || self.has_overlap[i] {
                        let (def, rect) = &self.scroll_displays[idx];
                        let screen_rect = fit_rect(rect, scale, tx, ty);
                        if glowing {
                            draw_key_glow(
                                pixmap,
                                &screen_rect,
                                &style,
                                t,
                                None,
                                &mut self.glow_cache,
                                &mut glow_time,
                            );
                        }
                        scroller::draw_scroll_display(
                            pixmap,
                            &screen_rect,
                            &style,
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
                    if t > 0.0 && style.glow_radius > 0.0 {
                        let label = el.def.label.as_deref().unwrap_or_default();
                        let rect = fit_rect(&el.rect, scale, tx, ty);
                        draw_key_glow(
                            pixmap,
                            &rect,
                            &style,
                            t,
                            key_analog[i],
                            &mut self.glow_cache,
                            &mut glow_time,
                        );
                        draw_key_face(
                            pixmap,
                            &rect,
                            &style,
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
                    if t > 0.0 && style.glow_radius > 0.0 {
                        let (def, rect) = &self.scroll_displays[idx];
                        let screen_rect = fit_rect(rect, scale, tx, ty);
                        draw_key_glow(
                            pixmap,
                            &screen_rect,
                            &style,
                            t,
                            None,
                            &mut self.glow_cache,
                            &mut glow_time,
                        );
                        scroller::draw_scroll_display(
                            pixmap,
                            &screen_rect,
                            &style,
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
            Ok(bytes) => {
                self.custom_font =
                    bytes.and_then(|b| Font::from_bytes(b, FontSettings::default()).ok());
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
