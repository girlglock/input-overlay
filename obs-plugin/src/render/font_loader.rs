use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use fontdue::{Font, FontSettings};
use super::{system_fonts, texture};
pub const FONTS_DIR_NAME: &str = "input-overlay-fonts";
const EXTENSIONS: [&str; 2] = ["ttf", "otf"];

pub struct LoadedFont {
    pub font: Font,
    pub real_bold: bool,
}

pub fn fonts_dir() -> Option<PathBuf> {
    Some(plugin_binary_path()?.parent()?.join(FONTS_DIR_NAME))
}

#[cfg(windows)]
fn plugin_binary_path() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };

    let mut module = HMODULE::default();
    unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(plugin_binary_path as *const () as *const u16),
            &mut module,
        )
        .ok()?;
    }
    let mut buf = vec![0u16; 32768];
    let len = unsafe { GetModuleFileNameW(module, &mut buf) } as usize;
    (len > 0).then(|| PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])))
}

#[cfg(target_os = "linux")]
fn plugin_binary_path() -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;

    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    let found = unsafe { libc::dladdr(plugin_binary_path as *const libc::c_void, &mut info) };
    if found == 0 || info.dli_fname.is_null() {
        return None;
    }
    let name = unsafe { std::ffi::CStr::from_ptr(info.dli_fname) };
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(name.to_bytes())))
}

fn resolve(family: &str, file_path: &str, bold: bool, dir: &Path) -> Option<(PathBuf, bool)> {
    let file_path = file_path.trim();
    if !file_path.is_empty() {
        if texture::is_network_path(file_path) {
            tracing::warn!("font: refusing network path {file_path}");
            return None;
        }
        let p = Path::new(file_path);
        return Some((if p.is_absolute() { p.to_path_buf() } else { dir.join(p) }, false));
    }

    let family = family.trim();
    if family.is_empty() || family.contains(['/', '\\', ':']) {
        return None;
    }
    if bold {
        for ext in EXTENSIONS {
            let p = dir.join(format!("{family}-bold.{ext}"));
            if p.is_file() {
                return Some((p, true));
            }
        }
    }
    EXTENSIONS
        .iter()
        .map(|ext| dir.join(format!("{family}.{ext}")))
        .chain(std::iter::once(dir.join(family)))
        .find(|p| p.is_file())
        .map(|p| (p, false))
}

pub fn spawn_load(family: &str, file_path: &str, bold: bool) -> Option<Receiver<Option<LoadedFont>>> {
    if family.trim().is_empty() && file_path.trim().is_empty() {
        return None;
    }
    
    let (family, file_path) = (family.to_string(), file_path.to_string());
    let (tx, rx) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("OverlayFont".into())
        .spawn(move || {
            let _ = tx.send(load(&family, &file_path, bold));
        })
        .is_ok();
    spawned.then_some(rx)
}

fn load(family: &str, file_path: &str, bold: bool) -> Option<LoadedFont> {
    let dir = fonts_dir();
    
    if dir.is_none() {
        tracing::error!("font: couldn't locate the plugin binary to find {FONTS_DIR_NAME}/");
    }

    let (path, index, real_bold) =
        match dir.as_deref().and_then(|d| resolve(family, file_path, bold, d)) {
            Some((path, real_bold)) => (path, 0, real_bold),
            None => match system_fonts::find(family, bold) {
                Some(found) => (found.face.path.clone(), found.face.index, found.real_bold),
                None => {
                    tracing::error!(
                        "font: '{family}' not found in {FONTS_DIR_NAME}/ or the installed system fonts - falling back to the default font"
                    );
                    return None;
                }
            },
        };
    tracing::info!("font: '{family}' -> {} (face {index})", path.display());
    let bytes = std::fs::read(&path)
        .inspect_err(|e| tracing::error!("font: failed to read {}: {e}", path.display()))
        .ok()?;
    let settings = FontSettings {
        collection_index: index,
        ..FontSettings::default()
    };
    let font = Font::from_bytes(bytes, settings)
        .inspect_err(|e| tracing::error!("font: failed to parse {}: {e}", path.display()))
        .ok()?;
    Some(LoadedFont { font, real_bold })
}
