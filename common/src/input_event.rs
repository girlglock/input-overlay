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
    //gpbutton uses w3c standard
    GamepadButton {
        pad: u32,
        button: u8,
        pressed: bool,
        value: f32,
        timestamp: u64,
    },
    //axis 0: left stick x
    //axis 1: left stick y
    //axis 2: right stick x
    //axis 3: right stick y
    GamepadAxis {
        pad: u32,
        axis: u8,
        value: f32,
        timestamp: u64,
    },
}
