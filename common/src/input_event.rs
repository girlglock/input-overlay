#[derive(Debug, Clone)]
pub enum InputEvent {
    KeyPress {
        rawcode: u16,
        timestamp: u64,
    },
    KeyRelease {
        rawcode: u16,
        timestamp: u64,
    },
    MouseButton {
        button: u8,
        pressed: bool,
        timestamp: u64,
    },
    MouseScroll {
        rotation: i8,
        timestamp: u64,
    },
    MouseMove {
        dx: i32,
        dy: i32,
        timestamp: u64,
    },
    AnalogDepth {
        rawcode: u16,
        depth: f32,
        timestamp: u64,
    },
}
