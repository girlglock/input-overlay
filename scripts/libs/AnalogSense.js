const keys = [
    { "name": "Escape", "wooting": 0x29, "razer": 0x6E, "bytech": 1 },
    { "name": "F1", "wooting": 0x3A, "razer": 0x70, "bytech": 2 },
    { "name": "F2", "wooting": 0x3B, "razer": 0x71, "bytech": 3 },
    { "name": "F3", "wooting": 0x3C, "razer": 0x72, "bytech": 4 },
    { "name": "F4", "wooting": 0x3D, "razer": 0x73, "bytech": 5 },
    { "name": "F5", "wooting": 0x3E, "razer": 0x74, "bytech": 6 },
    { "name": "F6", "wooting": 0x3F, "razer": 0x75, "bytech": 7 },
    { "name": "F7", "wooting": 0x40, "razer": 0x76, "bytech": 8 },
    { "name": "F8", "wooting": 0x41, "razer": 0x77, "bytech": 9 },
    { "name": "F9", "wooting": 0x42, "razer": 0x78, "bytech": 10 },
    { "name": "F10", "wooting": 0x43, "razer": 0x79, "bytech": 11 },
    { "name": "F11", "wooting": 0x44, "razer": 0x7A, "bytech": 12 },
    { "name": "F12", "wooting": 0x45, "razer": 0x7B, "bytech": 13 },
    { "name": "`", "wooting": 0x35, "razer": 0x01, "bytech": 14 },
    { "name": "1", "wooting": 0x1E, "razer": 0x02, "bytech": 15 },
    { "name": "2", "wooting": 0x1F, "razer": 0x03, "bytech": 16 },
    { "name": "3", "wooting": 0x20, "razer": 0x04, "bytech": 17 },
    { "name": "4", "wooting": 0x21, "razer": 0x05, "bytech": 18 },
    { "name": "5", "wooting": 0x22, "razer": 0x06, "bytech": 19 },
    { "name": "6", "wooting": 0x23, "razer": 0x07, "bytech": 20 },
    { "name": "7", "wooting": 0x24, "razer": 0x08, "bytech": 21 },
    { "name": "8", "wooting": 0x25, "razer": 0x09, "bytech": 22 },
    { "name": "9", "wooting": 0x26, "razer": 0x0A, "bytech": 23 },
    { "name": "0", "wooting": 0x27, "razer": 0x0B, "bytech": 24 },
    { "name": "-", "wooting": 0x2D, "razer": 0x0C, "bytech": 25 },
    { "name": "=", "wooting": 0x2E, "razer": 0x0D, "bytech": 26 },
    { "name": "Backspace", "wooting": 0x2A, "razer": 0x0F, "bytech": 27 },
    { "name": "Tab", "wooting": 0x2B, "razer": 0x10, "bytech": 28 },
    { "name": "Q", "wooting": 0x14, "razer": 0x11, "bytech": 29 },
    { "name": "W", "wooting": 0x1A, "razer": 0x12, "bytech": 30 },
    { "name": "E", "wooting": 0x08, "razer": 0x13, "bytech": 31 },
    { "name": "R", "wooting": 0x15, "razer": 0x14, "bytech": 32 },
    { "name": "T", "wooting": 0x17, "razer": 0x15, "bytech": 33 },
    { "name": "Y", "wooting": 0x1C, "razer": 0x16, "bytech": 34 },
    { "name": "U", "wooting": 0x18, "razer": 0x17, "bytech": 35 },
    { "name": "I", "wooting": 0x0C, "razer": 0x18, "bytech": 36 },
    { "name": "O", "wooting": 0x12, "razer": 0x19, "bytech": 37 },
    { "name": "P", "wooting": 0x13, "razer": 0x1A, "bytech": 38 },
    { "name": "[", "wooting": 0x2F, "razer": 0x1B, "bytech": 39 },
    { "name": "]", "wooting": 0x30, "razer": 0x1C, "bytech": 40 },
    { "name": "Enter", "wooting": 0x28, "razer": 0x2B, "bytech": 54 },
    { "name": "Caps Lock", "wooting": 0x39, "razer": 0x1E, "bytech": 42 },
    { "name": "A", "wooting": 0x04, "razer": 0x1F, "bytech": 43 },
    { "name": "S", "wooting": 0x16, "razer": 0x20, "bytech": 44 },
    { "name": "D", "wooting": 0x07, "razer": 0x21, "bytech": 45 },
    { "name": "F", "wooting": 0x09, "razer": 0x22, "bytech": 46 },
    { "name": "G", "wooting": 0x0A, "razer": 0x23, "bytech": 47 },
    { "name": "H", "wooting": 0x0B, "razer": 0x24, "bytech": 48 },
    { "name": "J", "wooting": 0x0D, "razer": 0x25, "bytech": 49 },
    { "name": "K", "wooting": 0x0E, "razer": 0x26, "bytech": 50 },
    { "name": "L", "wooting": 0x0F, "razer": 0x27, "bytech": 51 },
    { "name": ";", "wooting": 0x33, "razer": 0x28, "bytech": 52 },
    { "name": "'", "wooting": 0x34, "razer": 0x29, "bytech": 53 },
    { "name": "Backslash", "wooting": 0x31, "razer": 0x2A, "bytech": 41 },
    { "name": "Left Shift", "wooting": 0xE1, "razer": 0x2C, "nuphy": 0x200, "bytech": 55 },
    { "name": "Intl Backslash", "wooting": 0x64, "razer": 0x2D },
    { "name": "Z", "wooting": 0x1D, "razer": 0x2E, "bytech": 56 },
    { "name": "X", "wooting": 0x1B, "razer": 0x2F, "bytech": 57 },
    { "name": "C", "wooting": 0x06, "razer": 0x30, "bytech": 58 },
    { "name": "V", "wooting": 0x19, "razer": 0x31, "bytech": 59 },
    { "name": "B", "wooting": 0x05, "razer": 0x32, "bytech": 60 },
    { "name": "N", "wooting": 0x11, "razer": 0x33, "bytech": 61 },
    { "name": "M", "wooting": 0x10, "razer": 0x34, "bytech": 62 },
    { "name": ",", "wooting": 0x36, "razer": 0x35, "bytech": 63 },
    { "name": ".", "wooting": 0x37, "razer": 0x36, "bytech": 64 },
    { "name": "/", "wooting": 0x38, "razer": 0x37, "bytech": 65 },
    { "name": "Right Shift", "wooting": 0xE5, "razer": 0x39, "nuphy": 0x2000, "bytech": 66 },
    { "name": "Left Ctrl", "wooting": 0xE0, "razer": 0x3A, "nuphy": 0x100, "bytech": 67 },
    { "name": "Left Meta", "wooting": 0xE3, "razer": 0x7F, "nuphy": 0x800, "bytech": 68 },
    { "name": "Left Alt", "wooting": 0xE2, "razer": 0x3C, "nuphy": 0x400, "bytech": 69 },
    { "name": "Space", "wooting": 0x2C, "razer": 0x3D, "bytech": 70 },
    { "name": "Right Alt", "wooting": 0xE6, "razer": 0x3E, "nuphy": 0x4000, "bytech": 71 },
    { "name": "Right Meta", "wooting": 0xE7, "nuphy": 0x8000 },
    { "name": "Fn", "wooting": 0x409, "razer": 0x3B, "nuphy": 0xff05, "bytech": 72 },
    { "name": "Context Menu", "wooting": 0x65, "razer": 0x81 },
    { "name": "Right Ctrl", "wooting": 0xE4, "razer": 0x40, "nuphy": 0x1000, "bytech": 73 },
    { "name": "Print Screen", "wooting": 0x46, "razer": 0x7C },
    { "name": "Pause", "wooting": 0x48, "razer": 0x7D },
    { "name": "Scroll Lock", "wooting": 0x47, "razer": 0x7E },
    { "name": "Insert", "wooting": 0x49, "razer": 0x4B },
    { "name": "Home", "wooting": 0x4A, "razer": 0x50, "bytech": 100 },
    { "name": "Page Up", "wooting": 0x4B, "razer": 0x55, "bytech": 102 },
    { "name": "Delete", "wooting": 0x4C, "razer": 0x4C, "bytech": 99 },
    { "name": "End", "wooting": 0x4D, "razer": 0x51 },
    { "name": "Page Down", "wooting": 0x4E, "razer": 0x56, "bytech": 103 },
    { "name": "Up Arrow", "wooting": 0x52, "razer": 0x53, "bytech": 74 },
    { "name": "Left Arrow", "wooting": 0x50, "razer": 0x4F, "bytech": 76 },
    { "name": "Down Arrow", "wooting": 0x51, "razer": 0x54, "bytech": 75 },
    { "name": "Right Arrow", "wooting": 0x4F, "razer": 0x59, "bytech": 77 },
    { "name": "Num Lock", "wooting": 0x53, "razer": 0x5A },
    { "name": "Numpad /", "wooting": 0x54, "razer": 0x5F },
    { "name": "Numpad *", "wooting": 0x55, "razer": 0x64 },
    { "name": "Numpad -", "wooting": 0x56, "razer": 0x69 },
    { "name": "Numpad 7", "wooting": 0x5F, "razer": 0x5B },
    { "name": "Numpad 8", "wooting": 0x60, "razer": 0x60 },
    { "name": "Numpad 9", "wooting": 0x61, "razer": 0x65 },
    { "name": "Numpad +", "wooting": 0x57, "razer": 0x6A },
    { "name": "Numpad 4", "wooting": 0x5C, "razer": 0x5C },
    { "name": "Numpad 5", "wooting": 0x5D, "razer": 0x61 },
    { "name": "Numpad 6", "wooting": 0x5E, "razer": 0x66 },
    { "name": "Numpad 1", "wooting": 0x59, "razer": 0x5D },
    { "name": "Numpad 2", "wooting": 0x5A, "razer": 0x62 },
    { "name": "Numpad 3", "wooting": 0x5B, "razer": 0x67 },
    { "name": "Numpad Enter", "wooting": 0x58, "razer": 0x6C },
    { "name": "Numpad 0", "wooting": 0x62, "razer": 0x63 },
    { "name": "Numpad .", "wooting": 0x63, "razer": 0x68 }
];
const wooting_to_name = {}; Object.values(keys).forEach(key => wooting_to_name[key.wooting] = key.name);
const razer_to_wooting = {}; Object.values(keys).forEach(key => razer_to_wooting[key.razer] = key.wooting);
const nuphy_to_wooting = {}; Object.values(keys).forEach(key => nuphy_to_wooting[key.nuphy ?? key.wooting] = key.wooting);
const bytech_to_wooting = {}; Object.values(keys).forEach(key => { if ("bytech" in key) bytech_to_wooting[key.bytech] = key.wooting; });

const KEY_NONE = 0;
const KEY_A = 0x04;
const KEY_B = 0x05;
const KEY_C = 0x06;
const KEY_D = 0x07;
const KEY_E = 0x08;
const KEY_F = 0x09;
const KEY_G = 0x0a;
const KEY_H = 0x0b;
const KEY_I = 0x0c;
const KEY_J = 0x0d;
const KEY_K = 0x0e;
const KEY_L = 0x0f;
const KEY_M = 0x10;
const KEY_N = 0x11;
const KEY_O = 0x12;
const KEY_P = 0x13;
const KEY_Q = 0x14;
const KEY_R = 0x15;
const KEY_S = 0x16;
const KEY_T = 0x17;
const KEY_U = 0x18;
const KEY_V = 0x19;
const KEY_W = 0x1a;
const KEY_X = 0x1b;
const KEY_Y = 0x1c;
const KEY_Z = 0x1d;
const KEY_1 = 0x1e;
const KEY_2 = 0x1f;
const KEY_3 = 0x20;
const KEY_4 = 0x21;
const KEY_5 = 0x22;
const KEY_6 = 0x23;
const KEY_7 = 0x24;
const KEY_8 = 0x25;
const KEY_9 = 0x26;
const KEY_0 = 0x27;
const KEY_ENTER = 0x28;
const KEY_ESCAPE = 0x29;
const KEY_BACKSPACE = 0x2a;
const KEY_TAB = 0x2b;
const KEY_SPACE = 0x2c;
const KEY_MINUS = 0x2d;
const KEY_EQUALS = 0x2e;
const KEY_BRACKET_LEFT = 0x2f;
const KEY_BRACKET_RIGHT = 0x30;
const KEY_BACKSLASH = 0x31;
const KEY_INTL_HASH = 0x32;
const KEY_SEMICOLON = 0x33;
const KEY_QUOTE = 0x34;
const KEY_BACKQUOTE = 0x35;
const KEY_COMMA = 0x36;
const KEY_PERIOD = 0x37;
const KEY_SLASH = 0x38;
const KEY_CAPS_LOCK = 0x39;
const KEY_F1 = 0x3a;
const KEY_F2 = 0x3b;
const KEY_F3 = 0x3c;
const KEY_F4 = 0x3d;
const KEY_F5 = 0x3e;
const KEY_F6 = 0x3f;
const KEY_F7 = 0x40;
const KEY_F8 = 0x41;
const KEY_F9 = 0x42;
const KEY_F10 = 0x43;
const KEY_F11 = 0x44;
const KEY_F12 = 0x45;
const KEY_PRINT_SCREEN = 0x46;
const KEY_SCROLL_LOCK = 0x47;
const KEY_PAUSE = 0x48;
const KEY_INSERT = 0x49;
const KEY_HOME = 0x4a;
const KEY_PAGE_UP = 0x4b;
const KEY_DEL = 0x4c;
const KEY_END = 0x4d;
const KEY_PAGE_DOWN = 0x4e;
const KEY_ARROW_RIGHT = 0x4f;
const KEY_ARROW_LEFT = 0x50;
const KEY_ARROW_DOWN = 0x51;
const KEY_ARROW_UP = 0x52;
const KEY_NUM_LOCK = 0x53;
const KEY_NUMPAD_DIVIDE = 0x54;
const KEY_NUMPAD_MULTIPLY = 0x55;
const KEY_NUMPAD_SUBTRACT = 0x56;
const KEY_NUMPAD_ADD = 0x57;
const KEY_NUMPAD_ENTER = 0x58;
const KEY_NUMPAD1 = 0x59;
const KEY_NUMPAD2 = 0x5a;
const KEY_NUMPAD3 = 0x5b;
const KEY_NUMPAD4 = 0x5c;
const KEY_NUMPAD5 = 0x5d;
const KEY_NUMPAD6 = 0x5e;
const KEY_NUMPAD7 = 0x5f;
const KEY_NUMPAD8 = 0x60;
const KEY_NUMPAD9 = 0x61;
const KEY_NUMPAD0 = 0x62;
const KEY_NUMPAD_DECIMAL = 0x63;
const KEY_INTL_BACKSLASH = 0x64;
const KEY_CTX = 0x65;
const KEY_POWER = 0x66;
const KEY_NUMPAD_EQUAL = 0x67;
const KEY_F13 = 0x68;
const KEY_F14 = 0x69;
const KEY_F15 = 0x6a;
const KEY_F16 = 0x6b;
const KEY_F17 = 0x6c;
const KEY_F18 = 0x6d;
const KEY_F19 = 0x6e;
const KEY_F20 = 0x6f;
const KEY_F21 = 0x70;
const KEY_F22 = 0x71;
const KEY_F23 = 0x72;
const KEY_F24 = 0x73;
const KEY_OPEN = 0x74;
const KEY_HELP = 0x75;
const KEY_SELECT = 0x77;
const KEY_AGAIN = 0x79;
const KEY_UNDO = 0x7a;
const KEY_CUT = 0x7b;
const KEY_COPY = 0x7c;
const KEY_PASTE = 0x7d;
const KEY_FIND = 0x7e;
const KEY_VOLUME_MUTE = 0x7f;
const KEY_VOLUME_UP = 0x80;
const KEY_VOLUME_DOWN = 0x81;
const KEY_NUMPAD_COMMA = 0x85;
const KEY_INTL_RO = 0x87;
const KEY_KANA_MODE = 0x88;
const KEY_INTL_YEN = 0x89;
const KEY_CONVERT = 0x8a;
const KEY_NON_CONVERT = 0x8b;
const KEY_LANG1 = 0x90;
const KEY_LANG2 = 0x91;
const KEY_LANG3 = 0x92;
const KEY_LANG4 = 0x93;
const KEY_LCTRL = 0xe0;
const KEY_LSHIFT = 0xe1;
const KEY_LALT = 0xe2;
const KEY_LMETA = 0xe3;
const KEY_RCTRL = 0xe4;
const KEY_RSHIFT = 0xe5;
const KEY_RALT = 0xe6;
const KEY_RMETA = 0xe7;
const KEY_OEM_1 = 0x403;
const KEY_OEM_2 = 0x404;
const KEY_OEM_3 = 0x405;
const KEY_FN = 0x409;

const layout_keychron_q1_he = [ 6, 15,
    KEY_ESCAPE,    KEY_F1,    KEY_F2,   KEY_F3,   KEY_F4,   KEY_F5,   KEY_F6,    KEY_F7,   KEY_F8,   KEY_F9,    KEY_F10,       KEY_F11,          KEY_F12,           KEY_DEL,        KEY_NONE /* mute */,
    KEY_BACKQUOTE, KEY_1,     KEY_2,    KEY_3,    KEY_4,    KEY_5,    KEY_6,     KEY_7,    KEY_8,    KEY_9,     KEY_0,         KEY_MINUS,        KEY_EQUALS,        KEY_BACKSPACE,  KEY_PAGE_UP,
    KEY_TAB,       KEY_Q,     KEY_W,    KEY_E,    KEY_R,    KEY_T,    KEY_Y,     KEY_U,    KEY_I,    KEY_O,     KEY_P,         KEY_BRACKET_LEFT, KEY_BRACKET_RIGHT, KEY_BACKSLASH,  KEY_PAGE_DOWN,
    KEY_CAPS_LOCK, KEY_A,     KEY_S,    KEY_D,    KEY_F,    KEY_G,    KEY_H,     KEY_J,    KEY_K,    KEY_L,     KEY_SEMICOLON, KEY_QUOTE,        KEY_ENTER,         KEY_HOME,       KEY_NONE,
    KEY_LSHIFT,    KEY_NONE,  KEY_Z,    KEY_X,    KEY_C,    KEY_V,    KEY_B,     KEY_N,    KEY_M,    KEY_COMMA, KEY_PERIOD,    KEY_NONE,         KEY_SLASH,         KEY_RSHIFT,     KEY_ARROW_UP,
    KEY_LCTRL,     KEY_LMETA, KEY_LALT, KEY_NONE, KEY_NONE, KEY_NONE, KEY_SPACE, KEY_NONE, KEY_NONE, KEY_RMETA, KEY_FN,        KEY_RCTRL,        KEY_ARROW_LEFT,    KEY_ARROW_DOWN, KEY_ARROW_RIGHT,
];

const layout_keychron_q3_he = [ 6, 16,
    KEY_ESCAPE,    KEY_F1,    KEY_F2,   KEY_F3,   KEY_F4,   KEY_F5,   KEY_F6,    KEY_F7,   KEY_F8,   KEY_F9,    KEY_F10,       KEY_F11,          KEY_F12,           KEY_PRINT_SCREEN, KEY_OEM_1,      KEY_OEM_2 /* cycle rgb effect */,
    KEY_BACKQUOTE, KEY_1,     KEY_2,    KEY_3,    KEY_4,    KEY_5,    KEY_6,     KEY_7,    KEY_8,    KEY_9,     KEY_0,         KEY_MINUS,        KEY_EQUALS,        KEY_BACKSPACE,    KEY_INSERT,     KEY_HOME,
    KEY_TAB,       KEY_Q,     KEY_W,    KEY_E,    KEY_R,    KEY_T,    KEY_Y,     KEY_U,    KEY_I,    KEY_O,     KEY_P,         KEY_BRACKET_LEFT, KEY_BRACKET_RIGHT, KEY_BACKSLASH,    KEY_DEL,        KEY_END,
    KEY_CAPS_LOCK, KEY_A,     KEY_S,    KEY_D,    KEY_F,    KEY_G,    KEY_H,     KEY_J,    KEY_K,    KEY_L,     KEY_SEMICOLON, KEY_QUOTE,        KEY_NONE,          KEY_ENTER,        KEY_PAGE_UP,    KEY_PAGE_DOWN,
    KEY_LSHIFT,    KEY_NONE,  KEY_Z,    KEY_X,    KEY_C,    KEY_V,    KEY_B,     KEY_N,    KEY_M,    KEY_COMMA, KEY_PERIOD,    KEY_NONE,         KEY_SLASH,         KEY_RSHIFT,       KEY_NONE,       KEY_ARROW_UP,
    KEY_LCTRL,     KEY_LMETA, KEY_LALT, KEY_NONE, KEY_NONE, KEY_NONE, KEY_SPACE, KEY_NONE, KEY_NONE, KEY_RALT,  KEY_RMETA,     KEY_FN,           KEY_RCTRL,         KEY_ARROW_LEFT,   KEY_ARROW_DOWN, KEY_ARROW_RIGHT,  
];

const layout_keychron_q5_he = [ 6, 19,
    KEY_ESCAPE,    KEY_NONE,  KEY_F1,   KEY_F2,   KEY_F3,   KEY_F4,   KEY_F5,    KEY_F6,   KEY_F7,   KEY_F8,    KEY_F9,        KEY_F10,          KEY_F11,           KEY_F12,        KEY_DEL,         KEY_OEM_1,     KEY_OEM_2,         KEY_OEM_3,           KEY_NONE /* mute */,
    KEY_BACKQUOTE, KEY_1,     KEY_2,    KEY_3,    KEY_4,    KEY_5,    KEY_6,     KEY_7,    KEY_8,    KEY_9,     KEY_0,         KEY_MINUS,        KEY_EQUALS,        KEY_BACKSPACE,  KEY_PAGE_UP,     KEY_NUM_LOCK,  KEY_NUMPAD_DIVIDE, KEY_NUMPAD_MULTIPLY, KEY_NUMPAD_SUBTRACT,
    KEY_TAB,       KEY_Q,     KEY_W,    KEY_E,    KEY_R,    KEY_T,    KEY_Y,     KEY_U,    KEY_I,    KEY_O,     KEY_P,         KEY_BRACKET_LEFT, KEY_BRACKET_RIGHT, KEY_BACKSLASH,  KEY_PAGE_DOWN,   KEY_NUMPAD7,   KEY_NUMPAD8,       KEY_NUMPAD9,         KEY_NUMPAD_ADD,
    KEY_CAPS_LOCK, KEY_A,     KEY_S,    KEY_D,    KEY_F,    KEY_G,    KEY_H,     KEY_J,    KEY_K,    KEY_L,     KEY_SEMICOLON, KEY_QUOTE,        KEY_ENTER,         KEY_HOME,       KEY_NONE,        KEY_NUMPAD4,   KEY_NUMPAD5,       KEY_NUMPAD6,         KEY_NONE,
    KEY_LSHIFT,    KEY_NONE,  KEY_Z,    KEY_X,    KEY_C,    KEY_V,    KEY_B,     KEY_N,    KEY_M,    KEY_COMMA, KEY_PERIOD,    KEY_NONE,         KEY_SLASH,         KEY_RSHIFT,     KEY_ARROW_UP,    KEY_NUMPAD1,   KEY_NUMPAD2,       KEY_NUMPAD3,         KEY_NUMPAD_ENTER,
    KEY_LCTRL,     KEY_LMETA, KEY_LALT, KEY_NONE, KEY_NONE, KEY_NONE, KEY_SPACE, KEY_NONE, KEY_NONE, KEY_RMETA, KEY_FN,        KEY_RCTRL,        KEY_ARROW_LEFT,    KEY_ARROW_DOWN, KEY_ARROW_RIGHT, KEY_NONE,      KEY_NUMPAD0,       KEY_NUMPAD_DECIMAL,  KEY_NONE,
];

const layout_keychron_k2_he = [ 6, 16,
    KEY_ESCAPE,    KEY_F1,    KEY_F2,   KEY_F3,   KEY_F4,   KEY_F5,   KEY_F6,    KEY_F7,   KEY_F8,   KEY_F9,    KEY_F10,       KEY_F11,          KEY_F12,           KEY_PRINT_SCREEN, KEY_DEL,         KEY_OEM_2 /* cycle rgb effect */,
    KEY_BACKQUOTE, KEY_1,     KEY_2,    KEY_3,    KEY_4,    KEY_5,    KEY_6,     KEY_7,    KEY_8,    KEY_9,     KEY_0,         KEY_MINUS,        KEY_EQUALS,        KEY_BACKSPACE,    KEY_PAGE_UP,     KEY_NONE,
    KEY_TAB,       KEY_Q,     KEY_W,    KEY_E,    KEY_R,    KEY_T,    KEY_Y,     KEY_U,    KEY_I,    KEY_O,     KEY_P,         KEY_BRACKET_LEFT, KEY_BRACKET_RIGHT, KEY_BACKSLASH,    KEY_PAGE_DOWN,   KEY_NONE,
    KEY_CAPS_LOCK, KEY_A,     KEY_S,    KEY_D,    KEY_F,    KEY_G,    KEY_H,     KEY_J,    KEY_K,    KEY_L,     KEY_SEMICOLON, KEY_QUOTE,        KEY_ENTER,         KEY_HOME,         KEY_NONE,        KEY_NONE,
    KEY_LSHIFT,    KEY_NONE,  KEY_Z,    KEY_X,    KEY_C,    KEY_V,    KEY_B,     KEY_N,    KEY_M,    KEY_COMMA, KEY_PERIOD,    KEY_SLASH,        KEY_RSHIFT,        KEY_ARROW_UP,     KEY_END,         KEY_NONE,
    KEY_LCTRL,     KEY_LMETA, KEY_LALT, KEY_NONE, KEY_NONE, KEY_NONE, KEY_SPACE, KEY_NONE, KEY_NONE, KEY_RALT,  KEY_FN,        KEY_RCTRL,        KEY_ARROW_LEFT,    KEY_ARROW_DOWN,   KEY_ARROW_RIGHT, KEY_NONE,
];

const layout_lemokey_p1_he = [ 6, 15,
    KEY_ESCAPE,    KEY_F1,    KEY_F2,   KEY_F3,   KEY_F4,   KEY_F5,   KEY_F6,    KEY_F7,   KEY_F8,   KEY_F9,    KEY_F10,       KEY_F11,          KEY_F12,           KEY_DEL,        KEY_NONE /* mute */,
    KEY_BACKQUOTE, KEY_1,     KEY_2,    KEY_3,    KEY_4,    KEY_5,    KEY_6,     KEY_7,    KEY_8,    KEY_9,     KEY_0,         KEY_MINUS,        KEY_EQUALS,        KEY_BACKSPACE,  KEY_HOME,
    KEY_TAB,       KEY_Q,     KEY_W,    KEY_E,    KEY_R,    KEY_T,    KEY_Y,     KEY_U,    KEY_I,    KEY_O,     KEY_P,         KEY_BRACKET_LEFT, KEY_BRACKET_RIGHT, KEY_BACKSLASH,  KEY_PAGE_UP,
    KEY_CAPS_LOCK, KEY_A,     KEY_S,    KEY_D,    KEY_F,    KEY_G,    KEY_H,     KEY_J,    KEY_K,    KEY_L,     KEY_SEMICOLON, KEY_QUOTE,        KEY_ENTER,         KEY_PAGE_DOWN,  KEY_NONE,
    KEY_LSHIFT,    KEY_NONE,  KEY_Z,    KEY_X,    KEY_C,    KEY_V,    KEY_B,     KEY_N,    KEY_M,    KEY_COMMA, KEY_PERIOD,    KEY_NONE,         KEY_SLASH,         KEY_RSHIFT,     KEY_ARROW_UP,
    KEY_LCTRL,     KEY_LMETA, KEY_LALT, KEY_NONE, KEY_NONE, KEY_NONE, KEY_SPACE, KEY_NONE, KEY_NONE, KEY_RMETA, KEY_FN,        KEY_RCTRL,        KEY_ARROW_LEFT,    KEY_ARROW_DOWN, KEY_ARROW_RIGHT,
];

const layout_get_rows = (layout) => layout[0];
const layout_get_cols = (layout) => layout[1];
const layout_get_size = (layout) => layout_get_rows(layout) * layout_get_cols(layout);
const layout_get_item = (layout, index) => layout[2 + index];
const layout_index_to_row = (layout, index) => parseInt(index / layout_get_cols(layout));
const layout_index_to_col = (layout, index) => index % layout_get_cols(layout);

class AsProvider
{
    constructor(dev)
    {
        this.dev = dev;
    }

    getProductName()
    {
        return this.dev.productName;
    }

    forget()
    {
        this.stopListening();
        this.dev.forget();
    }

    _bufferToActiveKeys()
    {
        const active_keys = [];
        for (const [scancode, value] of Object.entries(this.buffer))
        {
            active_keys.push({ scancode, value });
        }
        return active_keys;
    }
}

//wooting analog interface v1
class AsProviderWootingV1 extends AsProvider
{
    static populateFilters(filters)
    {
        filters.push({ usagePage: 0xFF54, vendorId: 0x31E3 });
        filters.push({ usagePage: 0xFF54, vendorId: 0x03EB, productId: 0xFF01 }); // Wooting One old firmware
        filters.push({ usagePage: 0xFF54, vendorId: 0x03EB, productId: 0xFF02 }); // Wooting Two old firmware
    }

    startListening(handler)
    {
        //v1 big eddie u16 scancodes , u8 value, up to 16 keys
        this.dev.oninputreport = function(event)
        {
            const active_keys = [];
            for (let i = 0; i < event.data.byteLength; )
            {
                const scancode = (event.data.getUint8(i++) << 8) | event.data.getUint8(i++);
                if (scancode == 0) break;
                const value = event.data.getUint8(i++);
                active_keys.push({ scancode, value: value / 255 });
            }
            handler(active_keys);
        };
    }

    stopListening()
    {
        this.dev.oninputreport = undefined;
    }
}

//wooting analog interface v2 (firmware 2.13.0 and up | wootility 5.3.0 and up)
class AsProviderWootingV2 extends AsProvider
{
    static populateFilters(filters)
    {
        filters.push({ usagePage: 0xFF53, vendorId: 0x31E3 });
    }

    startListening(handler)
    {
        this.dev.oninputreport = function(event)
        {
            const active_keys = [];
            const data = event.data;
            //each entry is 4 bytes (pos, keycode, namespace + analog lo, analog depth
            //namespace 0 are regular hid keys, non zero are media keys
            //analog value is 10bit 0 to 1023
            for (let i = 0; i + 4 <= data.byteLength; i += 4)
            {
                const keycode      = data.getUint8(i + 1);
                const packed       = data.getUint8(i + 2);
                const value_hi     = data.getUint8(i + 3);

                const keyNamespace = (packed >> 2) & 0xf;
                const value_lo     = (packed >> 6) & 0x3;

                const scancode = (keyNamespace << 8) | keycode;
                const value    = (value_hi << 2) | value_lo;

                if (scancode === 0) break;
                if (value === 0) continue;

                active_keys.push({ scancode, value: value / 1023 });
            }
            handler(active_keys);
        };
    }

    stopListening()
    {
        this.dev.oninputreport = undefined;
    }
}

class AsProviderRazerHuntsman extends AsProvider
{
    static populateFilters(filters)
    {
        filters.push({ vendorId: 0x1532, productId: 0x0266, reportId: 7 }); // Razer Huntsman V2 Analog
        filters.push({ vendorId: 0x1532, productId: 0x0282, reportId: 7 }); // Razer Huntsman Mini Analog
    }

    startListening(handler)
    {
        this.dev.oninputreport = function(event)
        {
            if (event.reportId == 7)
            {
                const active_keys = [];
                for (let i = 0; i < event.data.byteLength; )
                {
                    const scancode = event.data.getUint8(i++);
                    if (scancode == 0)
                    {
                        break;
                    }
                    const value = event.data.getUint8(i++);
                    active_keys.push({
                        scancode: analogsense.razerScancodeToHidScancode(scancode),
                        value: value / 255
                    });
                }
                handler(active_keys);
            }
        };
    }

    stopListening()
    {
        this.dev.oninputreport = undefined;
    }
}

class AsProviderRazerHuntsmanV3 extends AsProvider
{
    static populateFilters(filters)
    {
        filters.push({ vendorId: 0x1532, productId: 0x02a6, reportId: 11 }); // Razer Huntsman V3 Pro
        filters.push({ vendorId: 0x1532, productId: 0x02a7, reportId: 11 }); // Razer Huntsman V3 Pro Tenkeyless
        filters.push({ vendorId: 0x1532, productId: 0x02b0, reportId: 11 }); // Razer Huntsman V3 Pro Mini
    }

    startListening(handler)
    {
        this.dev.oninputreport = function(event)
        {
            if (event.reportId == 11)
            {
                const active_keys = [];
                for (let i = 0; i < event.data.byteLength; )
                {
                    const scancode = event.data.getUint8(i++);
                    if (scancode == 0)
                    {
                        break;
                    }
                    const value = event.data.getUint8(i++);
                    i++; // unclear, might be something like "priority."
                    active_keys.push({
                        scancode: analogsense.razerScancodeToHidScancode(scancode),
                        value: value / 255
                    });
                }
                handler(active_keys);
            }
        };
    }

    stopListening()
    {
        this.dev.oninputreport = undefined;
    }
}

class AsProviderNuphy extends AsProvider
{
    static populateFilters(filters)
    {
        filters.push({ vendorId: 0x19f5, usagePage: 1, usage: 0 });
    }

    startListening(handler)
    {
        const _this = this;
        this.buffer = {};
        this.dev.oninputreport = function(event)
        {
            if (event.data.getUint8(0) == 0xA0)
            {
                const scancode = analogsense.nuphyScancodeToHidScancode(event.data.getUint16(2));
                if (scancode != 0)
                {
                    if (event.data.getUint8(7) == 0)
                    {
                        delete _this.buffer[scancode];
                    }
                    else
                    {
                        _this.buffer[scancode] = event.data.getUint8(7) / 200;
                    }

                    handler(_this._bufferToActiveKeys());
                }
            }
        };
    }

    stopListening()
    {
        this.dev.oninputreport = undefined;
    }
}

class AsProviderDrunkdeer extends AsProvider
{
    static populateFilters(filters)
    {
        filters.push({ usagePage: 0xFF00, vendorId: 0x352D });
    }

    startListening(handler)
    {
        const _this = this;
        this.interval = setInterval(function()
        {
            const buf = new Uint8Array(63);
            buf.set([
                      0xb6, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ]);
            _this.dev.sendReport(0x04, buf);
        });
        this.dev.oninputreport = function(event)
        {
            const n = event.data.getUint8(3);
            if (n == 0)
            {
                this.active_keys = [];
            }
            for (let i = 4; i != event.data.byteLength; ++i)
            {
                const value = event.data.getUint8(i);
                if (value != 0)
                {
                    this.active_keys.push({
                        scancode: analogsense.drunkdeerIndexToHidScancode((n * (64 - 5)) + (i - 4)),
                        value: value / 40
                    });
                }
            }
            if (n == 2)
            {
                handler(this.active_keys);
            }
        };
    }

    stopListening()
    {
        if (this.interval)
        {
            clearInterval(this.interval);
            this.interval = undefined;
        }
        this.dev.oninputreport = undefined;
    }
}

class AsProviderKeychron extends AsProvider
{
    static populateFilters(filters)
    {
        filters.push({ vendorId: 0x3434, usagePage: 0xFF60, usage: 0x61, productId: 0x0B10 }); // Keychron Q1 HE ANSI
        filters.push({ vendorId: 0x3434, usagePage: 0xFF60, usage: 0x61, productId: 0x0B11 }); // Keychron Q1 HE ISO
        filters.push({ vendorId: 0x3434, usagePage: 0xFF60, usage: 0x61, productId: 0x0B12 }); // Keychron Q1 HE JIS
        filters.push({ vendorId: 0x3434, usagePage: 0xFF60, usage: 0x61, productId: 0x0B30 }); // Keychron Q3 HE ANSI
        filters.push({ vendorId: 0x3434, usagePage: 0xFF60, usage: 0x61, productId: 0x0B50 }); // Keychron Q5 HE ANSI
        filters.push({ vendorId: 0x3434, usagePage: 0xFF60, usage: 0x61, productId: 0x0E20 }); // Keychron K2 HE ANSI
        filters.push({ vendorId: 0x3434, usagePage: 0xFF60, usage: 0x61, productId: 0x0E21 }); // Keychron K2 HE ISO
        filters.push({ vendorId: 0x3434, usagePage: 0xFF60, usage: 0x61, productId: 0x0E22 }); // Keychron K2 HE JIS
        filters.push({ vendorId: 0x362D, usagePage: 0xFF60, usage: 0x61, productId: 0x0610 }); // Lemokey P1 HE ANSI
    }

    startListening(handler)
    {
        const buf = new Uint8Array(32);
        buf.set([
            0xa9, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        this.dev.sendReport(0, buf);

        if (this.dev.productId == 0x0B10 // ANSI
            || this.dev.productId == 0x0B11 // ISO
            || this.dev.productId == 0x0B12 // JIS
            )
        {
            this.layout = layout_keychron_q1_he;
        }
        else if (this.dev.productId == 0x0B30)
        {
            this.layout = layout_keychron_q3_he;
        }
        else if (this.dev.productId == 0x0B50)
        {
            this.layout = layout_keychron_q5_he;
        }
        else if (this.dev.productId == 0x0610)
        {
            this.layout = layout_lemokey_p1_he;
        }
        else
        {
            this.layout = layout_keychron_k2_he;
        }

        const _this = this;
        this.dev.oninputreport = function(event)
        {
            const am_version = event.data.getUint8(2);
            const has_full_analogue_report = (event.data.getUint8(31) == 0x45);

            _this.index = 0;
            _this.buffer = {};

            if (has_full_analogue_report)
            {
                _this._requestAllKeys();
                _this.dev.oninputreport = function(event)
                {
                    for (let i = 0; i != 30; ++i)
                    {
                        const li = _this.index * 30 + i;
                        if (li < layout_get_size(_this.layout))
                        {
                            const key = layout_get_item(_this.layout, li);
                            if (key != KEY_NONE)
                            {
                                const travel = event.data.getUint8(2 + i);
                                if (travel >= 5)
                                {
                                    _this.buffer[key] = Math.min(travel / 235, 1.0);
                                }
                                else
                                {
                                    delete _this.buffer[key];
                                }
                            }
                        }
                    }
                    if (++_this.index == 4)
                    {
                        handler(_this._bufferToActiveKeys());
                        _this.index = 0;
                        _this._requestAllKeys();
                    }
                };
            }
            else
            {
                _this._requestSingleKey();
                _this.dev.oninputreport = function(event)
                {
                    const key = layout_get_item(_this.layout, _this.index);
                    const travel = event.data.getUint8(am_version >= 4 ? 6 : 3);
                    if (travel >= 5)
                    {
                        _this.buffer[key] = Math.min(travel / 235, 1.0);
                    }
                    else
                    {
                        delete _this.buffer[key];
                    }

                    handler(_this._bufferToActiveKeys());

                    do {
                        if (++_this.index == layout_get_size(_this.layout))
                        {
                            _this.index = 0;
                        }
                    } while (!_this._requestSingleKey());
                };
            }
        };
    }

    _requestSingleKey()
    {
        const key = layout_get_item(this.layout, this.index);
        if (key != KEY_NONE)
        {
            const row = layout_index_to_row(this.layout, this.index);
            const col = layout_index_to_col(this.layout, this.index);
            const buf = new Uint8Array(32);
            buf.set([
                0xa9, 0x30, row,  col,  0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]);
            this.dev.sendReport(0, buf);
            return true;
        }
        return false;
    }

    _requestAllKeys()
    {
        const buf = new Uint8Array(32);
        buf.set([
            0xa9, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        this.dev.sendReport(0, buf);
    }

    stopListening()
    {
        this.dev.oninputreport = undefined;
    }
}

class AsProviderMadlions extends AsProvider
{
    static populateFilters(filters)
    {
        // MAD60HE
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x1053 });
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x1054 });
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x1055 });
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x1056 });
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x105D });
        // MAD68HE
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x1058 });
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x1059 });
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x105A });
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x105C });
        // MAD68R (sorta the same as MAD68HE)
        filters.push({ vendorId: 0x373b, usagePage: 0xFF60, usage: 0x61, productId: 0x10A7 });
    }

    startListening(handler)
    {
        if (this.dev.productId == 0x1053 || this.dev.productId == 0x1055 || this.dev.productId == 0x1056 || this.dev.productId == 0x105D)
        {
            this.layout = [
                KEY_ESCAPE,    KEY_1,     KEY_2,    KEY_3,    KEY_4,    KEY_5,    KEY_6,     KEY_7,    KEY_8,    KEY_9,     KEY_0,         KEY_MINUS,        KEY_EQUALS,        KEY_BACKSPACE,
                KEY_TAB,       KEY_Q,     KEY_W,    KEY_E,    KEY_R,    KEY_T,    KEY_Y,     KEY_U,    KEY_I,    KEY_O,     KEY_P,         KEY_BRACKET_LEFT, KEY_BRACKET_RIGHT, KEY_BACKSLASH,
                KEY_CAPS_LOCK, KEY_A,     KEY_S,    KEY_D,    KEY_F,    KEY_G,    KEY_H,     KEY_J,    KEY_K,    KEY_L,     KEY_SEMICOLON, KEY_QUOTE,        KEY_NONE,          KEY_ENTER,
                KEY_LSHIFT,    KEY_NONE,  KEY_Z,    KEY_X,    KEY_C,    KEY_V,    KEY_B,     KEY_N,    KEY_M,    KEY_COMMA, KEY_PERIOD,    KEY_SLASH,        KEY_NONE,          KEY_RSHIFT,
                KEY_LCTRL,     KEY_LMETA, KEY_LALT, KEY_NONE, KEY_NONE, KEY_NONE, KEY_SPACE, KEY_NONE, KEY_NONE, KEY_RMETA, KEY_RALT,      KEY_CTX,          KEY_RCTRL,         KEY_FN,
            ];
        }
        else
        {
            this.layout = [
                KEY_ESCAPE,    KEY_1,     KEY_2,    KEY_3,    KEY_4,    KEY_5,    KEY_6,     KEY_7,    KEY_8,    KEY_9,     KEY_0,         KEY_MINUS,        KEY_EQUALS,        KEY_BACKSPACE,  KEY_INSERT,
                KEY_TAB,       KEY_Q,     KEY_W,    KEY_E,    KEY_R,    KEY_T,    KEY_Y,     KEY_U,    KEY_I,    KEY_O,     KEY_P,         KEY_BRACKET_LEFT, KEY_BRACKET_RIGHT, KEY_BACKSLASH,  KEY_DEL,
                KEY_CAPS_LOCK, KEY_A,     KEY_S,    KEY_D,    KEY_F,    KEY_G,    KEY_H,     KEY_J,    KEY_K,    KEY_L,     KEY_SEMICOLON, KEY_QUOTE,        KEY_NONE,          KEY_ENTER,      KEY_PAGE_UP,
                KEY_LSHIFT,    KEY_NONE,  KEY_Z,    KEY_X,    KEY_C,    KEY_V,    KEY_B,     KEY_N,    KEY_M,    KEY_COMMA, KEY_PERIOD,    KEY_SLASH,        KEY_RSHIFT,        KEY_ARROW_UP,   KEY_PAGE_DOWN,
                KEY_LCTRL,     KEY_LMETA, KEY_LALT, KEY_NONE, KEY_NONE, KEY_NONE, KEY_SPACE, KEY_NONE, KEY_NONE, KEY_RALT,  KEY_FN,        KEY_RCTRL,        KEY_ARROW_LEFT,    KEY_ARROW_DOWN, KEY_ARROW_RIGHT,
            ];
        }

        this.offset = 0;
        this.buffer = {};

        const buf = new Uint8Array(32);
        buf.set([
            0x02, 0x96, 0x1C, 0x00, 0x00, 0x00, 0x00, 0x04,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        this.dev.sendReport(0, buf);

        const _this = this;
        this.dev.oninputreport = function(event)
        {
            for (let i = 0; i != 4; ++i)
            {
                if (_this.offset + i < _this.layout.length)
                {
                    const key = _this.layout[_this.offset + i];
                    const travel = event.data.getUint16(7 + (i * 5) + 3);
                    if (travel == 0)
                    {
                        delete _this.buffer[key];
                    }
                    else
                    {
                        _this.buffer[key] = travel / 350;
                    }
                }
            }
            handler(_this._bufferToActiveKeys());

            _this.offset += 4;
            if (_this.offset >= _this.layout.length)
            {
                _this.offset = 0;
            }

            buf[6] = _this.offset;
            _this.dev.sendReport(0, buf);
        };
    }

    stopListening()
    {
        this.dev.oninputreport = undefined;
    }
}

// by3265h arm chip
// possibly anything using the hubx.pro web software
// like: RK68 HE, DAREU COOL68 8K, Redragon K709 HE
class AsProviderBytech extends AsProvider
{
    static populateFilters(filters)
    {
        filters.push({ vendorId: 0x372E, productId: 0x105B, usagePage: 0xFF00 }); // Redragon K709 HE (K709PLB-RGB-M)
    }

    _buildPayload(cmd, sub)
    {
        const buf = new Uint8Array(63);
        buf[0] = cmd;
        buf[1] = sub;
        let sum = 9;
        for (let i = 0; i < buf.length - 1; i++) sum += buf[i];
        buf[buf.length - 1] = 255 - (sum % 256);
        return buf;
    }

    startListening(handler)
    {
        const _this = this;
        this.buffer = {};

        const payload = this._buildPayload(0x97, 0x00);
        this.dev.sendReport(9, payload);
        this._interval = setInterval(function() { _this.dev.sendReport(9, payload); }, 1000);

        this.dev.oninputreport = function(event)
        {
            if (event.data.getUint8(0) === 0x97 && event.data.getUint8(1) === 0x01)
            {
                _this.dev.sendReport(9, payload);

                _this.buffer = {};
                const count = event.data.getUint8(5);
                for (let i = 0; i < count; i += 4)
                {
                    const pos      = (event.data.getUint8(6 + i) << 8) | event.data.getUint8(7 + i);
                    const distance = (event.data.getUint8(8 + i) << 8) | event.data.getUint8(9 + i);
                    const scancode = analogsense.bytechScancodeToHidScancode(pos);
                    if (scancode !== 0 && distance > 10)
                    {
                        _this.buffer[scancode] = Math.min(distance / 355, 1.0);
                    }
                }
                handler(_this._bufferToActiveKeys());
            }
        };
    }

    stopListening()
    {
        if (this._interval)
        {
            clearInterval(this._interval);
            this._interval = undefined;
        }
        this.dev.oninputreport = undefined;
    }
}

window.analogsense = {
    providers: [
        AsProviderWootingV1,
        AsProviderWootingV2,
        AsProviderRazerHuntsman,
        AsProviderRazerHuntsmanV3,
        AsProviderNuphy,
        AsProviderDrunkdeer,
        AsProviderKeychron,
        AsProviderMadlions,
        AsProviderBytech,
    ],
    findProviderForDevice: function(dev)
    {
        for (const provider of analogsense.providers)
        {
            const filters = [];
            provider.populateFilters(filters);
            for (const filter of filters)
            {
                if ("vendorId" in filter && dev.vendorId != filter.vendorId)
                {
                    continue;
                }
                if ("productId" in filter && dev.productId != filter.productId)
                {
                    continue;
                }
                if ("usagePage" in filter)
                {
                    let found = false;
                    for (const collection of dev.collections)
                    {
                        if (collection.usagePage == filter.usagePage)
                        {
                            found = true;
                            break;
                        }
                    }
                    if (!found)
                    {
                        continue;
                    }
                }
                if ("reportId" in filter)
                {
                    let found = false;
                    loop1: for (const collection of dev.collections)
                    {
                        for (const inputReport of collection.inputReports)
                        {
                            if (inputReport.reportId == filter.reportId)
                            {
                                found = true;
                                break loop1;
                            }
                        }
                    }
                    if (!found)
                    {
                        continue;
                    }
                }
                return provider;
            }
        }
    },
    getDevices: async function()
    {
        const result = [];
        if ("hid" in navigator)
        {
            const devices = await navigator.hid.getDevices();
            for (const dev of devices)
            {
                const provider = analogsense.findProviderForDevice(dev);
                if (provider)
                {
                    if (!dev.opened)
                    {
                        await dev.open();
                    }
                    result.push(new provider(dev));
                }
            }
        }
        return result;
    },
    requestDevice: async function()
    {
        if ("hid" in navigator)
        {
            const filters = [];
            for (const provider of analogsense.providers)
            {
                provider.populateFilters(filters);
            }
            const devices = await navigator.hid.requestDevice({ filters });
            for (const dev of devices)
            {
                const provider = analogsense.findProviderForDevice(dev);
                if (provider)
                {
                    if (!dev.opened)
                    {
                        await dev.open();
                    }
                    return new provider(dev);
                }
            }
        }
        else
        {
            alert("Your browser does not support the WebHID API, so analog keyboard input will not be available.");
        }
        return null;
    },
    scancodeToString: function(scancode)
    {
        if (scancode in wooting_to_name)
        {
            return wooting_to_name[scancode];
        }
        return String(Number(scancode));
    },
    razerScancodeToHidScancode: function(scancode)
    {
        if (scancode in razer_to_wooting)
        {
            return razer_to_wooting[scancode];
        }
        console.warn("Failed to map Razer scancode to HID scancode:", scancode);
        return 0;
    },
    nuphyScancodeToHidScancode: function(scancode)
    {
        if (scancode in nuphy_to_wooting)
        {
            return nuphy_to_wooting[scancode];
        }
        console.warn("Failed to map NuPhy scancode to HID scancode:", scancode);
        return 0;
    },
    bytechScancodeToHidScancode: function(scancode)
    {
        if (scancode in bytech_to_wooting)
        {
            return bytech_to_wooting[scancode];
        }
        console.warn("Failed to map Bytech scancode to HID scancode:", scancode);
        return 0;
    },
    drunkdeerIndexToHidScancode(i)
    {
        switch (i)
        {
        case (0 * 21) + 0: return 0x29;   // KEY_ESCAPE
        case (0 * 21) + 2: return 0x3A;   // KEY_F1
        case (0 * 21) + 3: return 0x3B;   // KEY_F2
        case (0 * 21) + 4: return 0x3C;   // KEY_F3
        case (0 * 21) + 5: return 0x3D;   // KEY_F4
        case (0 * 21) + 6: return 0x3E;   // KEY_F5
        case (0 * 21) + 7: return 0x3F;   // KEY_F6
        case (0 * 21) + 8: return 0x40;   // KEY_F7
        case (0 * 21) + 9: return 0x41;   // KEY_F8
        case (0 * 21) + 10: return 0x42;  // KEY_F9
        case (0 * 21) + 11: return 0x43;  // KEY_F10
        case (0 * 21) + 12: return 0x44;  // KEY_F11
        case (0 * 21) + 13: return 0x45;  // KEY_F12
        case (0 * 21) + 14: return 0x4C;  // KEY_DEL

        case (1 * 21) + 0: return 0x35;   // KEY_BACKQUOTE
        case (1 * 21) + 1: return 0x1E;   // KEY_1
        case (1 * 21) + 2: return 0x1F;   // KEY_2
        case (1 * 21) + 3: return 0x20;   // KEY_3
        case (1 * 21) + 4: return 0x21;   // KEY_4
        case (1 * 21) + 5: return 0x22;   // KEY_5
        case (1 * 21) + 6: return 0x23;   // KEY_6
        case (1 * 21) + 7: return 0x24;   // KEY_7
        case (1 * 21) + 8: return 0x25;   // KEY_8
        case (1 * 21) + 9: return 0x26;   // KEY_9
        case (1 * 21) + 10: return 0x27;  // KEY_0
        case (1 * 21) + 11: return 0x2D;  // KEY_MINUS
        case (1 * 21) + 12: return 0x2E;  // KEY_EQUALS
        case (1 * 21) + 13: return 0x2A;  // KEY_BACKSPACE
        case (1 * 21) + 15: return 0x4A;  // KEY_HOME

        case (2 * 21) + 0: return 0x2B;   // KEY_TAB
        case (2 * 21) + 1: return 0x14;   // KEY_Q
        case (2 * 21) + 2: return 0x1A;   // KEY_W
        case (2 * 21) + 3: return 0x08;   // KEY_E
        case (2 * 21) + 4: return 0x15;   // KEY_R
        case (2 * 21) + 5: return 0x17;   // KEY_T
        case (2 * 21) + 6: return 0x1C;   // KEY_Y
        case (2 * 21) + 7: return 0x18;   // KEY_U
        case (2 * 21) + 8: return 0x0C;   // KEY_I
        case (2 * 21) + 9: return 0x12;   // KEY_O
        case (2 * 21) + 10: return 0x13;  // KEY_P
        case (2 * 21) + 11: return 0x2F;  // KEY_BRACKET_LEFT
        case (2 * 21) + 12: return 0x30;  // KEY_BRACKET_RIGHT
        case (2 * 21) + 13: return 0x31;  // KEY_BACKSLASH
        case (2 * 21) + 15: return 0x4B;  // KEY_PAGE_UP

        case (3 * 21) + 0: return 0x39;   // KEY_CAPS_LOCK
        case (3 * 21) + 1: return 0x04;   // KEY_A
        case (3 * 21) + 2: return 0x16;   // KEY_S
        case (3 * 21) + 3: return 0x07;   // KEY_D
        case (3 * 21) + 4: return 0x09;   // KEY_F
        case (3 * 21) + 5: return 0x0A;   // KEY_G
        case (3 * 21) + 6: return 0x0B;   // KEY_H
        case (3 * 21) + 7: return 0x0D;   // KEY_J
        case (3 * 21) + 8: return 0x0E;   // KEY_K
        case (3 * 21) + 9: return 0x0F;   // KEY_L
        case (3 * 21) + 10: return 0x33;  // KEY_SEMICOLON
        case (3 * 21) + 11: return 0x34;  // KEY_QUOTE
        case (3 * 21) + 13: return 0x28;  // KEY_ENTER
        case (3 * 21) + 15: return 0x4E;  // KEY_PAGE_DOWN

        case (4 * 21) + 0: return 0xE1;   // KEY_LSHIFT
        case (4 * 21) + 2: return 0x1D;   // KEY_Z
        case (4 * 21) + 3: return 0x1B;   // KEY_X
        case (4 * 21) + 4: return 0x06;   // KEY_C
        case (4 * 21) + 5: return 0x19;   // KEY_V
        case (4 * 21) + 6: return 0x05;   // KEY_B
        case (4 * 21) + 7: return 0x11;   // KEY_N
        case (4 * 21) + 8: return 0x10;   // KEY_M
        case (4 * 21) + 9: return 0x36;   // KEY_COMMA
        case (4 * 21) + 10: return 0x37;  // KEY_PERIOD
        case (4 * 21) + 11: return 0x38;  // KEY_SLASH
        case (4 * 21) + 13: return 0xE5;  // KEY_RSHIFT
        case (4 * 21) + 14: return 0x52;  // KEY_ARROW_UP
        case (4 * 21) + 15: return 0x4D;  // KEY_END

        case (5 * 21) + 0: return 0xE0;   // KEY_LCTRL
        case (5 * 21) + 1: return 0xE3;   // KEY_LMETA
        case (5 * 21) + 2: return 0xE2;   // KEY_LALT
        case (5 * 21) + 6: return 0x2C;   // KEY_SPACE
        case (5 * 21) + 10: return 0xE6;  // KEY_RALT
        case (5 * 21) + 11: return 0x409; // KEY_FN (Custom mapping)
        case (5 * 21) + 12: return 0x65;  // Key says "Menu" on it, doesn't seem to do anything.
        case (5 * 21) + 14: return 0x50;  // KEY_ARROW_LEFT
        case (5 * 21) + 15: return 0x51;  // KEY_ARROW_DOWN
        case (5 * 21) + 16: return 0x4F;  // KEY_ARROW_RIGHT
        }
        console.warn("Failed to map DrunkDeer key to HID scancode:", i);
        return 0;
    }
};