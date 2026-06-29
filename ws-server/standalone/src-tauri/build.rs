use std::path::Path;

fn main() {
    tauri_build::build();
    stage_web_assets();
}

fn stage_web_assets() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let repo_root = Path::new(&manifest_dir)
        .join("../../..")
        .canonicalize()
        .expect("failed to resolve repo root");

    let staging = Path::new(&manifest_dir).join("web-bundle");

    println!("cargo:rerun-if-changed={}", repo_root.display());

    if staging.exists() {
        std::fs::remove_dir_all(&staging).unwrap();
    }

    const SKIP: &[&str] = &[
        "media",
        "ws-server",
        ".git",
        "README.md",
        "LICENSE",
        "CNAME",
    ];

    std::fs::create_dir_all(&staging).unwrap();
    for entry in std::fs::read_dir(&repo_root).unwrap() {
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
