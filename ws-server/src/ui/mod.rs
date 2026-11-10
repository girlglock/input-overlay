pub mod markdown;

use std::collections::HashMap;
use std::ffi::{c_char, c_int, CStr, CString};
use std::io::Cursor;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use image::{AnimationDecoder, Frame, ImageResult, RgbaImage};

#[derive(rust_embed::Embed)]
#[folder = "$CARGO_MANIFEST_DIR/ui"]
struct UiAssets;

pub const APP_ICON_PNG: &[u8] = include_bytes!("../../icons/icon.png");

#[repr(C)]
struct UiHost {
    read_file: extern "C" fn(*const c_char, *mut *const u8, *mut usize) -> c_int,
    release_file: extern "C" fn(*const u8, usize),
    load_image: extern "C" fn(*const c_char, *mut *mut u8, *mut usize, *mut c_int, *mut c_int, *mut u32) -> c_int,
    free_image: extern "C" fn(*mut u8, usize),
    anim_frame: extern "C" fn(u32, f64, *mut *const u8, *mut usize, *mut f64) -> c_int,
    anim_release: extern "C" fn(u32),
}

#[repr(C)]
struct RawEvent {
    window: c_int,
    kind: c_int,
    value: c_int,
    id: [c_char; 64],
}

extern "C" {
    fn ui_init(host: *const UiHost) -> c_int;
    fn ui_shutdown();
    fn ui_load_font(path: *const c_char, family: *const c_char, weight: c_int, fallback: c_int) -> c_int;
    fn ui_open_window(
        title: *const c_char,
        width: c_int,
        height: c_int,
        resizable: c_int,
        document: *const c_char,
        icon: *const u8,
        icon_w: c_int,
        icon_h: c_int,
    ) -> c_int;
    fn ui_close_window(window: c_int);
    fn ui_window_exists(window: c_int) -> c_int;
    fn ui_minimize_window(window: c_int);
    fn ui_focus_window(window: c_int);
    fn ui_set_dark_theme(dark: c_int);
    fn ui_set_inner_rml(window: c_int, id: *const c_char, rml: *const c_char);
    fn ui_set_attribute(window: c_int, id: *const c_char, name: *const c_char, value: *const c_char);
    fn ui_remove_attribute(window: c_int, id: *const c_char, name: *const c_char);
    fn ui_has_attribute(window: c_int, id: *const c_char, name: *const c_char) -> c_int;
    fn ui_set_class(window: c_int, id: *const c_char, class: *const c_char, active: c_int);
    fn ui_set_property(window: c_int, id: *const c_char, name: *const c_char, value: *const c_char);
    fn ui_remove_property(window: c_int, id: *const c_char, name: *const c_char);
    fn ui_get_value(window: c_int, id: *const c_char, out: *mut c_char, cap: usize) -> usize;
    fn ui_set_value(window: c_int, id: *const c_char, value: *const c_char);
    #[allow(dead_code)]
    fn ui_select_clear(window: c_int, id: *const c_char);
    fn ui_select_add(
        window: c_int,
        id: *const c_char,
        label: *const c_char,
        value: *const c_char,
        selectable: c_int,
        class_name: *const c_char,
    );
    fn ui_blur(window: c_int);
    fn ui_poll_event(out: *mut RawEvent) -> c_int;
    fn ui_frame(max_wait: f64);
    fn ui_wake();
    fn ui_set_clipboard(text: *const c_char);
}

fn c(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

fn normalize(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.trim_start_matches("./").trim_start_matches('/').to_string()
}

fn read_ui_file(path: &str) -> Option<Vec<u8>> {
    if let Some(file) = UiAssets::get(&normalize(path)) {
        return Some(file.data.into_owned());
    }
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return std::fs::read(p).ok();
    }
    None
}

fn leak(bytes: Vec<u8>) -> (*mut u8, usize) {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    (Box::into_raw(boxed) as *mut u8, len)
}

unsafe fn unleak(ptr: *mut u8, len: usize) {
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
}

extern "C" fn host_read_file(path: *const c_char, data: *mut *const u8, len: *mut usize) -> c_int {
    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    let Some(bytes) = read_ui_file(&path) else {
        tracing::warn!("ui: file not found: {path}");
        return 0;
    };
    let (ptr, n) = leak(bytes);
    unsafe {
        *data = ptr;
        *len = n;
    }
    1
}

extern "C" fn host_release_file(data: *const u8, len: usize) {
    unsafe { unleak(data as *mut u8, len) }
}

fn premultiply(mut img: RgbaImage) -> Vec<u8> {
    for px in img.pixels_mut() {
        let a = px[3] as u16;
        for ch in &mut px.0[..3] {
            *ch = ((*ch as u16 * a + 127) / 255) as u8;
        }
    }
    img.into_raw()
}

type Frames = Box<dyn Iterator<Item = ImageResult<Frame>>>;

#[derive(Clone, Copy)]
enum AnimKind {
    Gif,
    Apng,
    WebP,
}

fn open_frames(kind: AnimKind, bytes: &Arc<[u8]>) -> Option<Frames> {
    let cursor = Cursor::new(Arc::clone(bytes));
    Some(match kind {
        AnimKind::Gif => Box::new(image::codecs::gif::GifDecoder::new(cursor).ok()?.into_frames()),
        AnimKind::Apng => Box::new(image::codecs::png::PngDecoder::new(cursor).ok()?.apng().ok()?.into_frames()),
        AnimKind::WebP => Box::new(image::codecs::webp::WebPDecoder::new(cursor).ok()?.into_frames()),
    })
}

fn anim_kind(bytes: &Arc<[u8]>) -> Option<AnimKind> {
    match image::guess_format(bytes).ok()? {
        image::ImageFormat::Gif => Some(AnimKind::Gif),
        image::ImageFormat::Png => {
            let dec = image::codecs::png::PngDecoder::new(Cursor::new(Arc::clone(bytes))).ok()?;
            dec.is_apng().ok()?.then_some(AnimKind::Apng)
        }
        image::ImageFormat::WebP => {
            let dec = image::codecs::webp::WebPDecoder::new(Cursor::new(Arc::clone(bytes))).ok()?;
            dec.has_animation().then_some(AnimKind::WebP)
        }
        _ => None,
    }
}

fn frame_delay(frame: &Frame) -> f64 {
    let (num, den) = frame.delay().numer_denom_ms();
    let ms = if den == 0 { 0.0 } else { num as f64 / den as f64 };
    if ms <= 10.0 { 0.1 } else { ms / 1000.0 }
}

struct Anim {
    kind: AnimKind,
    bytes: Arc<[u8]>,
    frames: Frames,
    pending: Option<(RgbaImage, f64)>,
    current: Vec<u8>,
    shown_delay: f64,
    next_at: Option<f64>,
}

impl Anim {
    fn next_frame(&mut self) -> Option<(RgbaImage, f64)> {
        for restarted in [false, true] {
            if restarted {
                self.frames = open_frames(self.kind, &self.bytes)?;
            }
            if let Some(Ok(frame)) = self.frames.next() {
                let delay = frame_delay(&frame);
                return Some((frame.into_buffer(), delay));
            }
        }
        None
    }
}

thread_local! {
    static ANIMS: RefCell<HashMap<u32, Anim>> = RefCell::new(HashMap::new());
}
static NEXT_ANIM_ID: AtomicU32 = AtomicU32::new(1);

fn load_image(path: &str) -> Option<(Vec<u8>, u32, u32, u32)> {
    let bytes: Arc<[u8]> = read_ui_file(path)?.into();
    if let Some(kind) = anim_kind(&bytes) {
        let mut frames = open_frames(kind, &bytes)?;
        let first = frames.next()?.ok()?;
        let first_delay = frame_delay(&first);
        let first = first.into_buffer();
        let (w, h) = first.dimensions();
        let mut anim = Anim {
            kind,
            bytes,
            frames,
            pending: None,
            current: Vec::new(),
            shown_delay: first_delay,
            next_at: None,
        };
        anim.pending = anim.next_frame();
        if anim.pending.is_some() {
            let id = NEXT_ANIM_ID.fetch_add(1, Ordering::Relaxed);
            ANIMS.with(|a| a.borrow_mut().insert(id, anim));
            return Some((premultiply(first), w, h, id));
        }
        return Some((premultiply(first), w, h, 0));
    }
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some((premultiply(img), w, h, 0))
}

extern "C" fn host_load_image(
    path: *const c_char,
    rgba: *mut *mut u8,
    len: *mut usize,
    w: *mut c_int,
    h: *mut c_int,
    anim_id: *mut u32,
) -> c_int {
    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    let Some((pixels, width, height, id)) = load_image(&path) else {
        return 0;
    };
    let (ptr, n) = leak(pixels);
    unsafe {
        *rgba = ptr;
        *len = n;
        *w = width as c_int;
        *h = height as c_int;
        *anim_id = id;
    }
    1
}

extern "C" fn host_free_image(rgba: *mut u8, len: usize) {
    unsafe { unleak(rgba, len) }
}

extern "C" fn host_anim_frame(id: u32, now: f64, rgba: *mut *const u8, len: *mut usize, next_in: *mut f64) -> c_int {
    ANIMS.with(|anims| anim_frame(&mut anims.borrow_mut(), id, now, rgba, len, next_in))
}

fn anim_frame(anims: &mut HashMap<u32, Anim>, id: u32, now: f64, rgba: *mut *const u8, len: *mut usize, next_in: *mut f64) -> c_int {
    let Some(anim) = anims.get_mut(&id) else {
        unsafe { *next_in = f64::INFINITY };
        return 0;
    };
    let next_at = *anim.next_at.get_or_insert(now + anim.shown_delay);
    if now < next_at {
        unsafe { *next_in = next_at - now };
        return 0;
    }
    let Some((frame, delay)) = anim.pending.take() else {
        unsafe { *next_in = f64::INFINITY };
        return 0;
    };
    anim.current = premultiply(frame);
    anim.shown_delay = delay;
    let base = if now - next_at > 0.25 { now } else { next_at };
    anim.next_at = Some(base + delay);
    anim.pending = anim.next_frame();
    unsafe {
        *rgba = anim.current.as_ptr();
        *len = anim.current.len();
        *next_in = base + delay - now;
    }
    1
}

extern "C" fn host_anim_release(id: u32) {
    ANIMS.with(|a| a.borrow_mut().remove(&id));
}

static ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

pub fn init() -> bool {
    let host = UiHost {
        read_file: host_read_file,
        release_file: host_release_file,
        load_image: host_load_image,
        free_image: host_free_image,
        anim_frame: host_anim_frame,
        anim_release: host_anim_release,
    };
    let ok = unsafe { ui_init(&host) != 0 };
    ACTIVE.store(ok, Ordering::Release);
    ok
}

pub fn shutdown() {
    if ACTIVE.swap(false, Ordering::AcqRel) {
        unsafe { ui_shutdown() }
    }
}

pub fn load_font(path: &str, family: &str, weight: i32, fallback: bool) -> bool {
    unsafe { ui_load_font(c(path).as_ptr(), c(family).as_ptr(), weight, fallback as c_int) != 0 }
}

pub fn set_dark_theme(dark: bool) {
    unsafe { ui_set_dark_theme(dark as c_int) }
}

pub fn frame(max_wait: Duration) {
    unsafe { ui_frame(max_wait.as_secs_f64()) }
}

pub fn wake() {
    if is_active() {
        unsafe { ui_wake() }
    }
}

pub fn set_clipboard(text: &str) {
    unsafe { ui_set_clipboard(c(text).as_ptr()) }
}

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn app_icon() -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory(APP_ICON_PNG).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Win(i32);

impl Win {
    pub fn open(title: &str, width: i32, height: i32, resizable: bool, document: &str) -> Option<Win> {
        let icon = app_icon();
        let (ptr, w, h) = icon
            .as_ref()
            .map(|(px, w, h)| (px.as_ptr(), *w as c_int, *h as c_int))
            .unwrap_or((std::ptr::null(), 0, 0));
        let id = unsafe {
            ui_open_window(c(title).as_ptr(), width, height, resizable as c_int, c(document).as_ptr(), ptr, w, h)
        };
        (id != 0).then_some(Win(id))
    }

    pub fn close(self) {
        unsafe { ui_close_window(self.0) }
    }
    pub fn exists(self) -> bool {
        unsafe { ui_window_exists(self.0) != 0 }
    }
    pub fn minimize(self) {
        unsafe { ui_minimize_window(self.0) }
    }
    pub fn focus(self) {
        unsafe { ui_focus_window(self.0) }
    }
    pub fn blur(self) {
        unsafe { ui_blur(self.0) }
    }

    pub fn set_inner_rml(self, id: &str, rml: &str) {
        unsafe { ui_set_inner_rml(self.0, c(id).as_ptr(), c(rml).as_ptr()) }
    }
    pub fn set_text(self, id: &str, text: &str) {
        self.set_inner_rml(id, &escape(text));
    }
    pub fn set_attr(self, id: &str, name: &str, value: &str) {
        unsafe { ui_set_attribute(self.0, c(id).as_ptr(), c(name).as_ptr(), c(value).as_ptr()) }
    }
    pub fn remove_attr(self, id: &str, name: &str) {
        unsafe { ui_remove_attribute(self.0, c(id).as_ptr(), c(name).as_ptr()) }
    }
    pub fn has_attr(self, id: &str, name: &str) -> bool {
        unsafe { ui_has_attribute(self.0, c(id).as_ptr(), c(name).as_ptr()) != 0 }
    }
    fn set_flag(self, id: &str, name: &str, on: bool) {
        if on {
            self.set_attr(id, name, "");
        } else {
            self.remove_attr(id, name);
        }
    }
    pub fn set_hidden(self, id: &str, hidden: bool) {
        self.set_flag(id, "hidden", hidden);
    }
    pub fn set_disabled(self, id: &str, disabled: bool) {
        self.set_flag(id, "disabled", disabled);
    }
    pub fn checked(self, id: &str) -> bool {
        self.has_attr(id, "checked")
    }
    pub fn set_checked(self, id: &str, checked: bool) {
        self.set_flag(id, "checked", checked);
    }
    pub fn set_class(self, id: &str, class: &str, active: bool) {
        unsafe { ui_set_class(self.0, c(id).as_ptr(), c(class).as_ptr(), active as c_int) }
    }
    pub fn set_property(self, id: &str, name: &str, value: &str) {
        unsafe { ui_set_property(self.0, c(id).as_ptr(), c(name).as_ptr(), c(value).as_ptr()) }
    }
    pub fn remove_property(self, id: &str, name: &str) {
        unsafe { ui_remove_property(self.0, c(id).as_ptr(), c(name).as_ptr()) }
    }
    pub fn value(self, id: &str) -> String {
        let mut buf = vec![0u8; 1024];
        let len = unsafe { ui_get_value(self.0, c(id).as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len()) };
        if len >= buf.len() {
            buf = vec![0u8; len + 1];
            unsafe { ui_get_value(self.0, c(id).as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len()) };
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..end]).into_owned()
    }
    pub fn set_value(self, id: &str, value: &str) {
        unsafe { ui_set_value(self.0, c(id).as_ptr(), c(value).as_ptr()) }
    }
    #[allow(dead_code)]
    pub fn select_clear(self, id: &str) {
        unsafe { ui_select_clear(self.0, c(id).as_ptr()) }
    }
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn select_add(self, id: &str, label: &str, value: &str) {
        self.select_add_ex(id, label, value, true, "");
    }
    pub fn select_add_ex(self, id: &str, label: &str, value: &str, selectable: bool, class: &str) {
        unsafe {
            ui_select_add(
                self.0,
                c(id).as_ptr(),
                c(&escape(label)).as_ptr(),
                c(value).as_ptr(),
                selectable as c_int,
                c(class).as_ptr(),
            )
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Click,
    Change,
    Blur,
    KeyDown(i32),
    MouseDown(i32),
    Wheel,
    Close,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub window: Win,
    pub kind: EventKind,
    pub id: String,
}

pub fn poll_events() -> Vec<Event> {
    let mut events = Vec::new();
    let mut raw = RawEvent { window: 0, kind: 0, value: 0, id: [0; 64] };
    while unsafe { ui_poll_event(&mut raw) } != 0 {
        let kind = match raw.kind {
            1 => EventKind::Click,
            2 => EventKind::Change,
            3 => EventKind::Blur,
            4 => EventKind::KeyDown(raw.value),
            5 => EventKind::MouseDown(raw.value),
            6 => EventKind::Wheel,
            7 => EventKind::Close,
            _ => continue,
        };
        let id = unsafe { CStr::from_ptr(raw.id.as_ptr()) }.to_string_lossy().into_owned();
        events.push(Event { window: Win(raw.window), kind, id });
    }
    events
}
