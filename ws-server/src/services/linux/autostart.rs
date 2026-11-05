pub fn is_enabled() -> bool {
    autostart_path().is_some_and(|p| p.exists())
}

pub fn set_enabled(enabled: bool, exe_path: &std::path::Path, args: &[&str]) -> bool {
    let Some(path) = autostart_path() else {
        return false;
    };
    if enabled {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let exec = std::iter::once(exe_path.to_string_lossy().as_ref())
            .chain(args.iter().copied())
            .map(quote_exec_arg)
            .collect::<Vec<_>>()
            .join(" ");
        let work = exe_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Input Overlay WS\nExec={exec}\nPath={work}\nX-GNOME-Autostart-enabled=true\n"
        );
        std::fs::write(&path, content).is_ok()
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => true,
            Err(_) if !path.exists() => true,
            Err(e) => {
                tracing::warn!("autostart remove failed: {e}");
                false
            }
        }
    }
}

//desktop entry spec: quote args with reserved chars, escape " ` $ \ inside quotes
fn quote_exec_arg(arg: &str) -> String {
    const RESERVED: &[char] = &[
        ' ', '\t', '\n', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(', ')', '`',
    ];
    if !arg.is_empty() && !arg.contains(RESERVED) {
        return arg.replace('%', "%%");
    }
    let mut out = String::from("\"");
    for c in arg.chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '%' => out.push_str("%%"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn autostart_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(std::path::PathBuf::from(home).join(".config/autostart/input-overlay-ws.desktop"))
}

#[cfg(test)]
mod tests {
    use super::quote_exec_arg;

    #[test]
    fn exec_args_are_quoted() {
        assert_eq!(quote_exec_arg("/opt/io/input-overlay-ws"), "/opt/io/input-overlay-ws");
        assert_eq!(quote_exec_arg("--notray"), "--notray");
        assert_eq!(quote_exec_arg("/home/a b/io ws"), "\"/home/a b/io ws\"");
        assert_eq!(quote_exec_arg("/x/$y\"z"), "\"/x/\\$y\\\"z\"");
        assert_eq!(quote_exec_arg("/x/100%"), "/x/100%%");
    }
}
