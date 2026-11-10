use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const EXTENSIONS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];

#[derive(Debug, Clone, PartialEq)]
pub struct SystemFace {
    pub path: PathBuf,
    pub index: u32,
    families: Vec<String>,
    full_name: String,
    weight: u16,
    italic: bool,
}

pub struct FoundFace<'a> {
    pub face: &'a SystemFace,
    pub real_bold: bool,
}

//"Open Sans", "open-sans" and "opensans" should all match eatchother
fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(windows)]
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    dirs.push(PathBuf::from(windir).join("Fonts"));
    // fonts installed "for current user only" land here instead
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
    }
    dirs
}

#[cfg(target_os = "linux")]
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(&home).join(".local/share/fonts"));
        dirs.push(PathBuf::from(home).join(".fonts"));
    }
    dirs
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        {
            out.push(path);
        }
    }
}

fn read_faces(path: &Path, out: &mut Vec<SystemFace>) {
    let Ok(file) = std::fs::File::open(path) else {
        return;
    };

    let Ok(data) = (unsafe { memmap2::Mmap::map(&file) }) else {
        return;
    };

    let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    for index in 0..count {
        let Ok(face) = ttf_parser::Face::parse(&data, index) else {
            continue;
        };

        let mut families = Vec::new();
        let mut full_name = String::new();
        for name in face.names() {
            let Some(text) = name.to_string() else {
                continue;
            };
            
            match name.name_id {
                ttf_parser::name_id::FAMILY | ttf_parser::name_id::TYPOGRAPHIC_FAMILY => {
                    let n = normalize(&text);
                    if !families.contains(&n) {
                        families.push(n);
                    }
                }
                ttf_parser::name_id::FULL_NAME if full_name.is_empty() => {
                    full_name = normalize(&text);
                }
                _ => {}
            }
        }
        if families.is_empty() && full_name.is_empty() {
            continue;
        }
        out.push(SystemFace {
            path: path.to_path_buf(),
            index,
            families,
            full_name,
            weight: face.weight().to_number(),
            italic: face.is_italic() || face.is_oblique(),
        });
    }
}

fn index() -> &'static [SystemFace] {
    static INDEX: OnceLock<Vec<SystemFace>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let started = std::time::Instant::now();
        let mut files = Vec::new();
        for dir in font_dirs() {
            collect_files(&dir, &mut files);
        }
        let mut faces = Vec::new();
        for file in &files {
            read_faces(file, &mut faces);
        }
        tracing::info!(
            "font: indexed {} system font faces from {} files in {:?}",
            faces.len(),
            files.len(),
            started.elapsed()
        );
        faces
    })
}

// A family name picks the upright face closest to regular (400) or, with
// bold on, to bold (700). Failing that, an exact full name ("Arial Bold",
// "Segoe UI Light") picks that face as-is - family first, since a regular
// face's full name is usually just the family name. real_bold = the picked
// face is already bold, so the renderer shouldn't fake-bold it on top.
fn pick<'a>(faces: &'a [SystemFace], name: &str, bold: bool) -> Option<FoundFace<'a>> {
    let query = normalize(name);
    if query.is_empty() {
        return None;
    }
    let target: i32 = if bold { 700 } else { 400 };
    faces
        .iter()
        .filter(|f| f.families.contains(&query))
        .min_by_key(|f| (f.italic, (f.weight as i32 - target).abs(), f.weight))
        .or_else(|| faces.iter().find(|f| f.full_name == query))
        .map(|face| FoundFace {
            face,
            real_bold: face.weight >= 600,
        })
}

pub fn find(name: &str, bold: bool) -> Option<FoundFace<'static>> {
    pick(index(), name, bold)
}
