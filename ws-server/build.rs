use std::path::Path;

fn main() {
    stage_web_assets();
    build_native_ui();
    #[cfg(windows)]
    embed_windows_resources();
}

fn build_native_ui() {
    println!("cargo:rerun-if-changed=native");

    let dst = cmake::Config::new("native")
        .profile("Release")
        .build_target("ui_native")
        .build();

    let libs = dst.join("build").join("libs");
    println!("cargo:rustc-link-search=native={}", libs.display());
    let mut names: Vec<String> = std::fs::read_dir(&libs)
        .expect("native build produced no libs folder")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            name.strip_suffix(".lib")
                .map(str::to_string)
                .or_else(|| name.strip_prefix("lib")?.strip_suffix(".a").map(str::to_string))
        })
        .filter(|n| !n.starts_with("rmlui_debugger"))
        .collect();
    let rank = |n: &str| {
        ["ui_native", "rmlui", "freetype", "glfw"]
            .iter()
            .position(|p| n.starts_with(p))
            .unwrap_or(usize::MAX)
    };
    names.sort_by_key(|n| rank(n));
    for name in names {
        println!("cargo:rustc-link-lib=static={name}");
    }

    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "windows" => {
            for lib in ["opengl32", "user32", "gdi32", "shell32"] {
                println!("cargo:rustc-link-lib={lib}");
            }
        }
        "linux" => {
            for lib in ["GL", "X11", "dl", "pthread", "m", "stdc++"] {
                println!("cargo:rustc-link-lib={lib}");
            }
        }
        _ => {}
    }
}

#[cfg(windows)]
fn embed_windows_resources() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    let mut res = winresource::WindowsResource::new();
    res.set_icon("icons/icon.ico");
    res.set("ProductName", "Input Overlay WS");
    res.set("FileDescription", "Input Overlay WebSocket Server");
    res.compile().expect("failed to embed Windows resources");
}

fn stage_web_assets() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let repo_root = Path::new(&manifest_dir)
        .join("..")
        .canonicalize()
        .expect("failed to resolve repo root");

    let web_src = repo_root.join("web");
    let staging = Path::new(&manifest_dir).join("web-bundle");

    println!("cargo:rerun-if-changed={}", web_src.display());

    if staging.exists() {
        std::fs::remove_dir_all(&staging).unwrap();
    }

    const SKIP: &[&str] = &["media", ".idea"];

    std::fs::create_dir_all(&staging).unwrap();
    for entry in std::fs::read_dir(&web_src).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if SKIP.iter().any(|s| *s == name_str.as_ref()) {
            continue;
        }
        let dst = staging.join(&name);
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dst);
        } else {
            std::fs::copy(entry.path(), dst).unwrap();
        }
    }

    mark_local_instance(&staging);
}

fn mark_local_instance(staging: &Path) {
    let consts_path = staging.join("scripts/consts.js");
    let contents = std::fs::read_to_string(&consts_path).unwrap();
    let patched = contents.replacen(
        "export const LOCAL_INSTANCE = false;",
        "export const LOCAL_INSTANCE = true;",
        1,
    );
    std::fs::write(&consts_path, patched).unwrap();
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dst_path);
        } else {
            std::fs::copy(entry.path(), dst_path).unwrap();
        }
    }
}
