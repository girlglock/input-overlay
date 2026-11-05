use std::path::PathBuf;

pub fn open_url(url: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .creation_flags(0x08000000)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

pub fn is_admin() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION::default();
        let mut cb = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let ok = GetTokenInformation(token, TokenElevation, Some(&mut elev as *mut _ as *mut _), cb, &mut cb).is_ok();
        let _ = CloseHandle(token);
        ok && elev.TokenIsElevated != 0
    }
    #[cfg(target_os = "linux")]
    {
        true
    }
}

pub fn system_prefers_dark() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::core::w;
        use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
        let mut value: u32 = 1;
        let mut size = std::mem::size_of::<u32>() as u32;
        let ok = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
        .is_ok();
        ok && value == 0
    }
    #[cfg(target_os = "linux")]
    {
        portal_theme_dark().unwrap_or(false)
    }
}

#[cfg(target_os = "linux")]
fn portal_theme_dark() -> zbus::Result<bool> {
    let conn = zbus::blocking::Connection::session()?;
    let reply = conn.call_method(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        Some("org.freedesktop.portal.Settings"),
        "Read",
        &("org.freedesktop.appearance", "color-scheme"),
    )?;
    let value: zbus::zvariant::OwnedValue = reply.body().deserialize()?;
    let mut value: &zbus::zvariant::Value = &value;
    while let zbus::zvariant::Value::Value(inner) = value {
        value = inner;
    }
    Ok(matches!(value, zbus::zvariant::Value::U32(1)))
}

pub fn system_font(family: &str, bold: bool) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        let file = match (family, bold) {
            ("Arial", false) => "arial.ttf",
            ("Arial", true) => "arialbd.ttf",
            ("Trebuchet MS", false) => "trebuc.ttf",
            ("Trebuchet MS", true) => "trebucbd.ttf",
            _ => return None,
        };
        let path = PathBuf::from(windir).join("Fonts").join(file);
        path.is_file().then_some(path)
    }
    #[cfg(target_os = "linux")]
    {
        let pattern = if bold { format!("{family}:bold") } else { family.to_string() };
        let out = std::process::Command::new("fc-match")
            .args(["-f", "%{file}", &pattern])
            .output()
            .ok()?;
        let path = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
        path.is_file().then_some(path)
    }
}

pub fn key_name(glfw_key: i32) -> Option<&'static str> {
    Some(match glfw_key {
        32 => "key_space",
        39 => "key_apostrophe",
        44 => "key_comma",
        45 => "key_minus",
        46 => "key_period",
        47 => "key_slash",
        48 => "key_0",
        49 => "key_1",
        50 => "key_2",
        51 => "key_3",
        52 => "key_4",
        53 => "key_5",
        54 => "key_6",
        55 => "key_7",
        56 => "key_8",
        57 => "key_9",
        59 => "key_semicolon",
        61 => "key_equals",
        65 => "key_a",
        66 => "key_b",
        67 => "key_c",
        68 => "key_d",
        69 => "key_e",
        70 => "key_f",
        71 => "key_g",
        72 => "key_h",
        73 => "key_i",
        74 => "key_j",
        75 => "key_k",
        76 => "key_l",
        77 => "key_m",
        78 => "key_n",
        79 => "key_o",
        80 => "key_p",
        81 => "key_q",
        82 => "key_r",
        83 => "key_s",
        84 => "key_t",
        85 => "key_u",
        86 => "key_v",
        87 => "key_w",
        88 => "key_x",
        89 => "key_y",
        90 => "key_z",
        91 => "key_openbracket",
        92 => "key_backslash",
        93 => "key_closebracket",
        96 => "key_grave",
        161 | 162 => "key_iso_backslash",
        256 => "key_escape",
        257 => "key_enter",
        258 => "key_tab",
        259 => "key_backspace",
        260 => "key_insert",
        261 => "key_delete",
        262 => "key_rightarrow",
        263 => "key_leftarrow",
        264 => "key_downarrow",
        265 => "key_uparrow",
        266 => "key_pageup",
        267 => "key_pagedown",
        268 => "key_home",
        269 => "key_end",
        280 => "key_capslock",
        281 => "key_scrolllock",
        282 => "key_numlock",
        283 => "key_printscreen",
        284 => "key_pause",
        k @ 290..=313 => FKEYS[(k - 290) as usize],
        k @ 320..=329 => NUMPAD[(k - 320) as usize],
        330 => "key_numpad_decimal",
        331 => "key_numpad_divide",
        332 => "key_numpad_multiply",
        333 => "key_numpad_subtract",
        334 => "key_numpad_add",
        335 => "key_numpad_enter",
        340 => "key_leftshift",
        341 => "key_leftctrl",
        342 => "key_leftalt",
        343 => "key_leftwin",
        344 => "key_rightshift",
        345 => "key_rightctrl",
        346 => "key_rightalt",
        347 => "key_rightwin",
        348 => "key_menu",
        _ => return None,
    })
}

const FKEYS: [&str; 24] = [
    "key_f1", "key_f2", "key_f3", "key_f4", "key_f5", "key_f6", "key_f7", "key_f8", "key_f9", "key_f10", "key_f11", "key_f12",
    "key_f13", "key_f14", "key_f15", "key_f16", "key_f17", "key_f18", "key_f19", "key_f20", "key_f21", "key_f22", "key_f23", "key_f24",
];
const NUMPAD: [&str; 10] = [
    "key_numpad_0", "key_numpad_1", "key_numpad_2", "key_numpad_3", "key_numpad_4",
    "key_numpad_5", "key_numpad_6", "key_numpad_7", "key_numpad_8", "key_numpad_9",
];

pub fn mouse_button_name(button: i32) -> Option<&'static str> {
    Some(match button {
        0 => "mouse_left",
        1 => "mouse_right",
        2 => "mouse_middle",
        3 => "mouse_4",
        4 => "mouse_5",
        _ => return None,
    })
}
