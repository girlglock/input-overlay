use std::sync::mpsc::Sender;

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::app::AppMsg;
use crate::ui;

pub struct Tray {
    #[cfg(windows)]
    thread_id: u32,
    #[cfg(windows)]
    thread: Option<std::thread::JoinHandle<()>>,
    #[cfg(target_os = "linux")]
    _tray: Option<TrayIcon>,
}

fn build_tray(tx: Sender<AppMsg>) -> Option<TrayIcon> {
    let settings_item = MenuItem::with_id("settings", "Settings", true, None);
    let quit_item = MenuItem::with_id("quit", "Exit", true, None);
    let menu = Menu::with_items(&[&settings_item, &quit_item]).ok()?;

    let Some((rgba, w, h)) = ui::app_icon() else {
        tracing::warn!("no window icon available for tray");
        return None;
    };
    let icon = Icon::from_rgba(rgba, w, h).ok()?;

    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("Input Overlay")
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
        .ok()?;

    let menu_tx = tx.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let msg = match event.id.as_ref() {
            "settings" => AppMsg::ToggleSettings,
            "quit" => AppMsg::Quit,
            _ => return,
        };
        let _ = menu_tx.send(msg);
        ui::wake();
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let _ = tx.send(AppMsg::ToggleSettings);
            ui::wake();
        }
    }));

    Some(tray)
}

impl Tray {
    #[cfg(windows)]
    pub fn spawn(tx: Sender<AppMsg>) -> Tray {
        use windows_sys::Win32::System::Threading::GetCurrentThreadId;
        use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};

        let (id_tx, id_rx) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("tray".into())
            .spawn(move || {
                let _tray = build_tray(tx);
                let _ = id_tx.send(unsafe { GetCurrentThreadId() });
                unsafe {
                    let mut msg: MSG = std::mem::zeroed();
                    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            })
            .ok();
        let thread_id = id_rx.recv().unwrap_or(0);
        Tray { thread_id, thread }
    }

    #[cfg(target_os = "linux")]
    pub fn spawn(tx: Sender<AppMsg>) -> Tray {
        Tray { _tray: build_tray(tx) }
    }
}

#[cfg(windows)]
impl Drop for Tray {
    fn drop(&mut self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
        unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
