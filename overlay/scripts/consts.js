//shroom

/**
 * @typedef {Object} KeyCode
 * @property {number} rawcode
 * @property {string} [browsercode]
 * @property {number} [hidcode]
 * @property {string} codename
 * @property {string} screenname
 */

/** @type {KeyCode[]} */
const KEY_CODE_LIST = [
    { rawcode: 27, browsercode: "escape", hidcode: 0x29, codename: "key_escape", screenname: "Escape" },
    { rawcode: 49, browsercode: "digit1", hidcode: 0x1e, codename: "key_1", screenname: "1" },
    { rawcode: 50, browsercode: "digit2", hidcode: 0x1f, codename: "key_2", screenname: "2" },
    { rawcode: 51, browsercode: "digit3", hidcode: 0x20, codename: "key_3", screenname: "3" },
    { rawcode: 52, browsercode: "digit4", hidcode: 0x21, codename: "key_4", screenname: "4" },
    { rawcode: 53, browsercode: "digit5", hidcode: 0x22, codename: "key_5", screenname: "5" },
    { rawcode: 54, browsercode: "digit6", hidcode: 0x23, codename: "key_6", screenname: "6" },
    { rawcode: 55, browsercode: "digit7", hidcode: 0x24, codename: "key_7", screenname: "7" },
    { rawcode: 56, browsercode: "digit8", hidcode: 0x25, codename: "key_8", screenname: "8" },
    { rawcode: 57, browsercode: "digit9", hidcode: 0x26, codename: "key_9", screenname: "9" },
    { rawcode: 48, browsercode: "digit0", hidcode: 0x27, codename: "key_0", screenname: "0" },
    { rawcode: 189, browsercode: "minus", hidcode: 0x2d, codename: "key_minus", screenname: "Minus" },
    { rawcode: 187, browsercode: "equal", hidcode: 0x2e, codename: "key_equals", screenname: "Equals" },
    { rawcode: 8, browsercode: "backspace", hidcode: 0x2a, codename: "key_backspace", screenname: "Backspace" },
    { rawcode: 112, browsercode: "f1", hidcode: 0x3a, codename: "key_f1", screenname: "F1" },
    { rawcode: 113, browsercode: "f2", hidcode: 0x3b, codename: "key_f2", screenname: "F2" },
    { rawcode: 114, browsercode: "f3", hidcode: 0x3c, codename: "key_f3", screenname: "F3" },
    { rawcode: 115, browsercode: "f4", hidcode: 0x3d, codename: "key_f4", screenname: "F4" },
    { rawcode: 116, browsercode: "f5", hidcode: 0x3e, codename: "key_f5", screenname: "F5" },
    { rawcode: 117, browsercode: "f6", hidcode: 0x3f, codename: "key_f6", screenname: "F6" },
    { rawcode: 118, browsercode: "f7", hidcode: 0x40, codename: "key_f7", screenname: "F7" },
    { rawcode: 119, browsercode: "f8", hidcode: 0x41, codename: "key_f8", screenname: "F8" },
    { rawcode: 120, browsercode: "f9", hidcode: 0x42, codename: "key_f9", screenname: "F9" },
    { rawcode: 121, browsercode: "f10", hidcode: 0x43, codename: "key_f10", screenname: "F10" },
    { rawcode: 122, browsercode: "f11", hidcode: 0x44, codename: "key_f11", screenname: "F11" },
    { rawcode: 123, browsercode: "f12", hidcode: 0x45, codename: "key_f12", screenname: "F12" },
    { rawcode: 124, browsercode: "f13", hidcode: 0x68, codename: "key_f13", screenname: "F13" },
    { rawcode: 125, browsercode: "f14", hidcode: 0x69, codename: "key_f14", screenname: "F14" },
    { rawcode: 126, browsercode: "f15", hidcode: 0x6a, codename: "key_f15", screenname: "F15" },
    { rawcode: 127, browsercode: "f16", hidcode: 0x6b, codename: "key_f16", screenname: "F16" },
    { rawcode: 128, browsercode: "f17", hidcode: 0x6c, codename: "key_f17", screenname: "F17" },
    { rawcode: 129, browsercode: "f18", hidcode: 0x6d, codename: "key_f18", screenname: "F18" },
    { rawcode: 130, browsercode: "f19", hidcode: 0x6e, codename: "key_f19", screenname: "F19" },
    { rawcode: 131, browsercode: "f20", hidcode: 0x6f, codename: "key_f20", screenname: "F20" },
    { rawcode: 132, browsercode: "f21", hidcode: 0x70, codename: "key_f21", screenname: "F21" },
    { rawcode: 133, browsercode: "f22", hidcode: 0x71, codename: "key_f22", screenname: "F22" },
    { rawcode: 134, browsercode: "f23", hidcode: 0x72, codename: "key_f23", screenname: "F23" },
    { rawcode: 135, browsercode: "f24", hidcode: 0x73, codename: "key_f24", screenname: "F24" },
    { rawcode: 44, browsercode: "printscreen", hidcode: 0x46, codename: "key_printscreen", screenname: "Print Screen" },
    { rawcode: 145, browsercode: "scrolllock", hidcode: 0x47, codename: "key_scrolllock", screenname: "Scroll Lock" },
    { rawcode: 19, browsercode: "pause", hidcode: 0x48, codename: "key_pause", screenname: "Pause" },
    { rawcode: 45, browsercode: "insert", hidcode: 0x49, codename: "key_insert", screenname: "Insert" },
    { rawcode: 46, browsercode: "delete", hidcode: 0x4c, codename: "key_delete", screenname: "Delete" },
    { rawcode: 36, browsercode: "home", hidcode: 0x4a, codename: "key_home", screenname: "Home" },
    { rawcode: 35, browsercode: "end", hidcode: 0x4d, codename: "key_end", screenname: "End" },
    { rawcode: 33, browsercode: "pageup", hidcode: 0x4b, codename: "key_pageup", screenname: "Page Up" },
    { rawcode: 34, browsercode: "pagedown", hidcode: 0x4e, codename: "key_pagedown", screenname: "Page Down" },
    { rawcode: 9, browsercode: "tab", hidcode: 0x2b, codename: "key_tab", screenname: "Tab" },
    { rawcode: 81, browsercode: "keyq", hidcode: 0x14, codename: "key_q", screenname: "Q" },
    { rawcode: 87, browsercode: "keyw", hidcode: 0x1a, codename: "key_w", screenname: "W" },
    { rawcode: 69, browsercode: "keye", hidcode: 0x08, codename: "key_e", screenname: "E" },
    { rawcode: 82, browsercode: "keyr", hidcode: 0x15, codename: "key_r", screenname: "R" },
    { rawcode: 84, browsercode: "keyt", hidcode: 0x17, codename: "key_t", screenname: "T" },
    { rawcode: 89, browsercode: "keyy", hidcode: 0x1c, codename: "key_y", screenname: "Y" },
    { rawcode: 85, browsercode: "keyu", hidcode: 0x18, codename: "key_u", screenname: "U" },
    { rawcode: 73, browsercode: "keyi", hidcode: 0x0c, codename: "key_i", screenname: "I" },
    { rawcode: 79, browsercode: "keyo", hidcode: 0x12, codename: "key_o", screenname: "O" },
    { rawcode: 80, browsercode: "keyp", hidcode: 0x13, codename: "key_p", screenname: "P" },
    { rawcode: 219, browsercode: "bracketleft", hidcode: 0x2f, codename: "key_openbracket", screenname: "Open Bracket" },
    { rawcode: 221, browsercode: "bracketright", hidcode: 0x30, codename: "key_closebracket", screenname: "Close Bracket" },
    { rawcode: 220, browsercode: "backslash", hidcode: 0x31, codename: "key_backslash", screenname: "Backslash" },
    { rawcode: 20, browsercode: "capslock", hidcode: 0x39, codename: "key_capslock", screenname: "Caps Lock" },
    { rawcode: 65, browsercode: "keya", hidcode: 0x04, codename: "key_a", screenname: "A" },
    { rawcode: 83, browsercode: "keys", hidcode: 0x16, codename: "key_s", screenname: "S" },
    { rawcode: 68, browsercode: "keyd", hidcode: 0x07, codename: "key_d", screenname: "D" },
    { rawcode: 70, browsercode: "keyf", hidcode: 0x09, codename: "key_f", screenname: "F" },
    { rawcode: 71, browsercode: "keyg", hidcode: 0x0a, codename: "key_g", screenname: "G" },
    { rawcode: 72, browsercode: "keyh", hidcode: 0x0b, codename: "key_h", screenname: "H" },
    { rawcode: 74, browsercode: "keyj", hidcode: 0x0d, codename: "key_j", screenname: "J" },
    { rawcode: 75, browsercode: "keyk", hidcode: 0x0e, codename: "key_k", screenname: "K" },
    { rawcode: 76, browsercode: "keyl", hidcode: 0x0f, codename: "key_l", screenname: "L" },
    { rawcode: 186, browsercode: "semicolon", hidcode: 0x33, codename: "key_semicolon", screenname: "Semicolon" },
    { rawcode: 222, browsercode: "quote", hidcode: 0x34, codename: "key_apostrophe", screenname: "Apostrophe" },
    { rawcode: 13, browsercode: "enter", hidcode: 0x28, codename: "key_enter", screenname: "Enter" },
    { rawcode: 160, browsercode: "shiftleft", hidcode: 0xe1, codename: "key_leftshift", screenname: "Left Shift" },
    { rawcode: 90, browsercode: "keyz", hidcode: 0x1d, codename: "key_z", screenname: "Z" },
    { rawcode: 88, browsercode: "keyx", hidcode: 0x1b, codename: "key_x", screenname: "X" },
    { rawcode: 67, browsercode: "keyc", hidcode: 0x06, codename: "key_c", screenname: "C" },
    { rawcode: 86, browsercode: "keyv", hidcode: 0x19, codename: "key_v", screenname: "V" },
    { rawcode: 66, browsercode: "keyb", hidcode: 0x05, codename: "key_b", screenname: "B" },
    { rawcode: 78, browsercode: "keyn", hidcode: 0x11, codename: "key_n", screenname: "N" },
    { rawcode: 77, browsercode: "keym", hidcode: 0x10, codename: "key_m", screenname: "M" },
    { rawcode: 188, browsercode: "comma", hidcode: 0x36, codename: "key_comma", screenname: "Comma" },
    { rawcode: 190, browsercode: "period", hidcode: 0x37, codename: "key_period", screenname: "Period" },
    { rawcode: 191, browsercode: "slash", hidcode: 0x38, codename: "key_slash", screenname: "Slash" },
    { rawcode: 161, browsercode: "shiftright", hidcode: 0xe5, codename: "key_rightshift", screenname: "Right Shift" },
    { rawcode: 162, browsercode: "controlleft", hidcode: 0xe0, codename: "key_leftctrl", screenname: "Left Ctrl" },
    { rawcode: 91, browsercode: "metaleft", hidcode: 0xe3, codename: "key_leftwin", screenname: "Left Windows" },
    { rawcode: 164, browsercode: "altleft", hidcode: 0xe2, codename: "key_leftalt", screenname: "Left Alt" },
    { rawcode: 32, browsercode: "space", hidcode: 0x2c, codename: "key_space", screenname: "Space" },
    { rawcode: 165, browsercode: "altright", hidcode: 0xe6, codename: "key_rightalt", screenname: "Right Alt" },
    { rawcode: 92, browsercode: "metaright", hidcode: 0xe7, codename: "key_rightwin", screenname: "Right Windows" },
    { rawcode: 93, browsercode: "contextmenu", codename: "key_menu", screenname: "Menu" },
    { rawcode: 163, browsercode: "controlright", hidcode: 0xe4, codename: "key_rightctrl", screenname: "Right Ctrl" },
    { rawcode: 37, browsercode: "arrowleft", hidcode: 0x50, codename: "key_leftarrow", screenname: "Left Arrow" },
    { rawcode: 38, browsercode: "arrowup", hidcode: 0x52, codename: "key_uparrow", screenname: "Up Arrow" },
    { rawcode: 39, browsercode: "arrowright", hidcode: 0x4f, codename: "key_rightarrow", screenname: "Right Arrow" },
    { rawcode: 40, browsercode: "arrowdown", hidcode: 0x51, codename: "key_downarrow", screenname: "Down Arrow" },
    { rawcode: 144, browsercode: "numlock", hidcode: 0x53, codename: "key_numlock", screenname: "Num Lock" },
    { rawcode: 111, browsercode: "numpaddivide", hidcode: 0x54, codename: "key_numpad_divide", screenname: "Numpad Divide" },
    { rawcode: 106, browsercode: "numpadmultiply", hidcode: 0x55, codename: "key_numpad_multiply", screenname: "Numpad Multiply" },
    { rawcode: 109, browsercode: "numpadsubtract", hidcode: 0x56, codename: "key_numpad_subtract", screenname: "Numpad Subtract" },
    { rawcode: 103, browsercode: "numpad7", hidcode: 0x5f, codename: "key_numpad_7", screenname: "Numpad 7" },
    { rawcode: 104, browsercode: "numpad8", hidcode: 0x60, codename: "key_numpad_8", screenname: "Numpad 8" },
    { rawcode: 105, browsercode: "numpad9", hidcode: 0x61, codename: "key_numpad_9", screenname: "Numpad 9" },
    { rawcode: 107, browsercode: "numpadadd", hidcode: 0x57, codename: "key_numpad_add", screenname: "Numpad Add" },
    { rawcode: 100, browsercode: "numpad4", hidcode: 0x5c, codename: "key_numpad_4", screenname: "Numpad 4" },
    { rawcode: 101, browsercode: "numpad5", hidcode: 0x5d, codename: "key_numpad_5", screenname: "Numpad 5" },
    { rawcode: 102, browsercode: "numpad6", hidcode: 0x5e, codename: "key_numpad_6", screenname: "Numpad 6" },
    { rawcode: 97, browsercode: "numpad1", hidcode: 0x59, codename: "key_numpad_1", screenname: "Numpad 1" },
    { rawcode: 98, browsercode: "numpad2", hidcode: 0x5a, codename: "key_numpad_2", screenname: "Numpad 2" },
    { rawcode: 99, browsercode: "numpad3", hidcode: 0x5b, codename: "key_numpad_3", screenname: "Numpad 3" },
    { rawcode: 108, browsercode: "numpadenter", hidcode: 0x58, codename: "key_numpad_enter", screenname: "Numpad Enter" },
    { rawcode: 96, browsercode: "numpad0", hidcode: 0x62, codename: "key_numpad_0", screenname: "Numpad 0" },
    { rawcode: 110, browsercode: "numpaddecimal", hidcode: 0x63, codename: "key_numpad_decimal", screenname: "Numpad Decimal" },
    { rawcode: 226, hidcode: 0x64, codename: "key_iso_backslash", screenname: "ISO Backslash" }, //iso pipe key next to the smol littlefuckass left shift
];

export const KEY_CODES = Object.assign(KEY_CODE_LIST, {
    /** @param {number} code */
    byRawcode: (code) => KEY_CODE_LIST.find(k => k.rawcode === code) ?? null,
    /** @param {number} code */
    byHidcode: (code) => KEY_CODE_LIST.find(k => k.hidcode === code) ?? null,
    /** @param {string} code */
    byBrowsercode: (code) => KEY_CODE_LIST.find(k => k.browsercode === code) ?? null,
    /** @param {string} name */
    byCodename: (name) => KEY_CODE_LIST.find(k => k.codename === name) ?? null,
});

/**
 * @typedef {Object} MouseCode
 * @property {number} rawcode
 * @property {number} browsercode
 * @property {string} codename
 * @property {string} screenname
 */

/** @type {MouseCode[]} */
const MOUSE_CODE_LIST = [
    { rawcode: 1, browsercode: 0, codename: "mouse_left", screenname: "Left Click" },
    { rawcode: 2, browsercode: 2, codename: "mouse_right", screenname: "Right Click" },
    { rawcode: 3, browsercode: 1, codename: "mouse_middle", screenname: "Middle Click" },
    { rawcode: 4, browsercode: 3, codename: "mouse_4", screenname: "Mouse 4" },
    { rawcode: 5, browsercode: 4, codename: "mouse_5", screenname: "Mouse 5" },
];

export const MOUSE_CODES = Object.assign(MOUSE_CODE_LIST, {
    /** @param {number} code */
    byRawcode: (code) => MOUSE_CODE_LIST.find(k => k.rawcode === code) ?? null,
    /** @param {number} code */
    byBrowsercode: (code) => MOUSE_CODE_LIST.find(k => k.browsercode === code) ?? null,
    /** @param {string} name */
    byCodename: (name) => MOUSE_CODE_LIST.find(k => k.codename === name) ?? null,
});

export const COLOR_PICKERS = [
    { id: "activecolor", defaultColor: "#8b5cf6" },
    { id: "backgroundcolor", defaultColor: "#1a1a1ad1" },
    { id: "activebgcolor", defaultColor: "#202020" },
    { id: "outlinecolor", defaultColor: "#4f4f4f" },
    { id: "fontcolor", defaultColor: "#ffffff" },
    { id: "inactivecolor", defaultColor: "#808080" }
];

export const FONT_FAMILY_LINKS = {
    "abel": "https://fonts.googleapis.com/css2?family=Abel&display=swap",
    "archivo-black": "https://fonts.googleapis.com/css2?family=Archivo+Black&display=swap",
    "arimo": "https://fonts.googleapis.com/css2?family=Arimo:ital,wght@0,400..700;1,400..700&display=swap",
    "bebas-neue": "https://fonts.googleapis.com/css2?family=Bebas+Neue&display=swap",
    "bitcount-prop-single": "https://fonts.googleapis.com/css2?family=Bitcount+Prop+Single:wght@100..900&display=swap",
    "bungee": "https://fonts.googleapis.com/css2?family=Bungee&display=swap",
    "caveat-brush": "https://fonts.googleapis.com/css2?family=Caveat+Brush&display=swap",
    "chewy": "https://fonts.googleapis.com/css2?family=Chewy&display=swap",
    "cinzel": "https://fonts.googleapis.com/css2?family=Cinzel:wght@400..900&display=swap",
    "comfortaa": "https://fonts.googleapis.com/css2?family=Comfortaa:wght@300..700&display=swap",
    "fjalla-one": "https://fonts.googleapis.com/css2?family=Fjalla+One&display=swap",
    "fredoka": "https://fonts.googleapis.com/css2?family=Fredoka:wght@300..700&display=swap",
    "inter": "https://fonts.googleapis.com/css2?family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&display=swap",
    "josefin-sans": "https://fonts.googleapis.com/css2?family=Josefin+Sans:ital,wght@0,100..700;1,100..700&display=swap",
    "knewave": "https://fonts.googleapis.com/css2?family=Knewave&display=swap",
    "lato": "https://fonts.googleapis.com/css2?family=Lato:ital,wght@0,100;0,300;0,400;0,700;0,900;1,100;1,300;1,400;1,700;1,900&display=swap",
    "lexend": "https://fonts.googleapis.com/css2?family=Lexend:wght@100..900&display=swap",
    "lexend-giga": "https://fonts.googleapis.com/css2?family=Lexend+Giga:wght@100..900&display=swap",
    "lilita-one": "https://fonts.googleapis.com/css2?family=Lilita+One&display=swap",
    "montserrat": "https://fonts.googleapis.com/css2?family=Montserrat:ital,wght@0,100..900;1,100..900&display=swap",
    "noto-sans": "https://fonts.googleapis.com/css2?family=Noto+Sans:ital,wght@0,100..900;1,100..900&display=swap",
    "open-sans": "https://fonts.googleapis.com/css2?family=Open+Sans:ital,wght@0,300..800;1,300..800&display=swap",
    "orbitron": "https://fonts.googleapis.com/css2?family=Orbitron:wght@400..900&display=swap",
    "oswald": "https://fonts.googleapis.com/css2?family=Oswald:wght@200..700&display=swap",
    "pacifico": "https://fonts.googleapis.com/css2?family=Pacifico&display=swap",
    "press-start-2p": "https://fonts.googleapis.com/css2?family=Press+Start+2P&display=swap",
    "raleway": "https://fonts.googleapis.com/css2?family=Raleway:ital,wght@0,100..900;1,100..900&display=swap",
    "roboto-condensed": "https://fonts.googleapis.com/css2?family=Roboto+Condensed:ital,wght@0,100..900;1,100..900&display=swap",
    "roboto-mono": "https://fonts.googleapis.com/css2?family=Roboto+Mono:ital,wght@0,100..700;1,100..700&display=swap",
    "roboto": "https://fonts.googleapis.com/css2?family=Roboto:ital,wght@0,100..900;1,100..900&display=swap",
    "share-tech": "https://fonts.googleapis.com/css2?family=Share+Tech&display=swap",
    "smooch-sans": "https://fonts.googleapis.com/css2?family=Smooch+Sans:wght@100..900&display=swap",
    "sora": "https://fonts.googleapis.com/css2?family=Sora:wght@100..800&display=swap",
    "special-elite": "https://fonts.googleapis.com/css2?family=Special+Elite&display=swap",
    "titan-one": "https://fonts.googleapis.com/css2?family=Titan+One&display=swap",
    "ArialPixel": "data:font/woff2;charset=utf-8;base64,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",
    "ubuntu": "https://fonts.googleapis.com/css2?family=Ubuntu:ital,wght@0,300;0,400;0,500;0,700;1,300;1,400;1,500;1,700&display=swap",
    "Borevs": "data:font/woff2;charset=utf-8;base64,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",
    "miku": "data:font/woff2;charset=utf-8;base64,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",
};