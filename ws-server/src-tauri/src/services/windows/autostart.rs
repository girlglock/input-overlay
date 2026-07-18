#[cfg(windows)]
const TASK_NAME: &str = "InputOverlayWS";

#[cfg(windows)]
pub fn is_enabled() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("schtasks")
            .args(["/Query", "/TN", TASK_NAME])
            .creation_flags(0x08000000) //CREATE_NO_WINDOW
            .output();
        matches!(out, Ok(o) if o.status.success())
    }
    #[cfg(not(windows))]
    false
}

#[cfg(windows)]
pub fn set_enabled(enabled: bool, exe_path: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        if enabled {
            create_task(exe_path)
        } else {
            delete_task()
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (enabled, exe_path);
        false
    }
}

#[cfg(windows)]
fn create_task(exe_path: &std::path::Path) -> bool {
    use std::os::windows::process::CommandExt;

    let exe = exe_path.to_string_lossy();
    let work = exe_path
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers><LogonTrigger><Enabled>true</Enabled></LogonTrigger></Triggers>
  <Principals><Principal><LogonType>InteractiveToken</LogonType><RunLevel>HighestAvailable</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions><Exec><Command>{exe}</Command><WorkingDirectory>{work}</WorkingDirectory></Exec></Actions>
</Task>"#
    );

    let tmp_path = std::env::temp_dir().join("iov_autostart.xml");
    {
        //UTF-16 LE with BOMoclat
        let utf16: Vec<u16> = std::iter::once(0xFEFFu16)
            .chain(xml.encode_utf16())
            .collect();
        let bytes: Vec<u8> = utf16.iter().flat_map(|c| c.to_le_bytes()).collect();
        if std::fs::write(&tmp_path, bytes).is_err() {
            return false;
        }
    }

    let ok = std::process::Command::new("schtasks")
        .args([
            "/Create",
            "/F",
            "/TN",
            TASK_NAME,
            "/XML",
            tmp_path.to_str().unwrap_or(""),
        ])
        .creation_flags(0x08000000)
        .status()
        .is_ok_and(|s| s.success());

    let _ = std::fs::remove_file(&tmp_path);
    ok
}

#[cfg(windows)]
fn delete_task() -> bool {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("schtasks")
        .args(["/Delete", "/F", "/TN", TASK_NAME])
        .creation_flags(0x08000000)
        .status()
        .is_ok_and(|s| s.success())
}
