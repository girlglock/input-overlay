// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

pub type SingleInstanceCallback = dyn FnMut(Vec<String>, String) + Send + 'static;

const IDENTIFIER: &str = "com.input-overlay-ws.app";

pub fn init(callback: impl FnMut(Vec<String>, String) + Send + 'static) -> bool {
    platform_impl::init(Box::new(callback))
}

pub fn destroy() {
    platform_impl::destroy()
}

#[cfg(windows)]
mod platform_impl {
    use super::{SingleInstanceCallback, IDENTIFIER};
    use std::ffi::CStr;
    use std::sync::Mutex;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            DataExchange::COPYDATASTRUCT,
            LibraryLoader::GetModuleHandleW,
            Threading::{CreateMutexW, ReleaseMutex},
        },
        UI::WindowsAndMessaging::{
            self as w32wm, CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, PostMessageW,
            PostQuitMessage, RegisterClassExW, SendMessageW, TranslateMessage, CREATESTRUCTW, GWLP_USERDATA,
            GWL_STYLE, MSG, WINDOW_LONG_PTR_INDEX, WM_CLOSE, WM_COPYDATA, WM_CREATE, WM_DESTROY, WNDCLASSEXW,
            WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP,
            WS_VISIBLE,
        },
    };

    const WMCOPYDATA_SINGLE_INSTANCE_DATA: usize = 1542;

    struct Handles {
        mutex: isize,
        hwnd: isize,
    }

    static HANDLES: Mutex<Option<Handles>> = Mutex::new(None);

    struct UserData {
        callback: Box<SingleInstanceCallback>,
    }

    impl UserData {
        unsafe fn from_hwnd_raw(hwnd: HWND) -> *mut Self {
            GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self
        }

        unsafe fn from_hwnd<'a>(hwnd: HWND) -> &'a mut Self {
            &mut *Self::from_hwnd_raw(hwnd)
        }

        fn run_callback(&mut self, args: Vec<String>, cwd: String) {
            (self.callback)(args, cwd)
        }
    }

    pub fn init(callback: Box<SingleInstanceCallback>) -> bool {
        let id = IDENTIFIER;

        let class_name = encode_wide(format!("{id}-sic"));
        let window_name = encode_wide(format!("{id}-siw"));
        let mutex_name = encode_wide(format!("{id}-sim"));

        let hmutex = unsafe { CreateMutexW(std::ptr::null(), true.into(), mutex_name.as_ptr()) };

        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                let hwnd = FindWindowW(class_name.as_ptr(), window_name.as_ptr());

                if !hwnd.is_null() {
                    let cwd = std::env::current_dir().unwrap_or_default();
                    let cwd = cwd.to_str().unwrap_or_default();

                    let args = std::env::args().collect::<Vec<String>>().join("|");

                    let data = format!("{cwd}|{args}\0",);

                    let bytes = data.as_bytes();
                    let cds = COPYDATASTRUCT {
                        dwData: WMCOPYDATA_SINGLE_INSTANCE_DATA,
                        cbData: bytes.len() as _,
                        lpData: bytes.as_ptr() as _,
                    };

                    SendMessageW(hwnd, WM_COPYDATA, 0, &cds as *const _ as _);

                    return false;
                }
            }
        } else {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("single-instance".into())
                .spawn(move || {
                    let userdata = Box::into_raw(Box::new(UserData { callback }));
                    let hwnd = create_event_target_window(&class_name, &window_name, userdata);
                    let _ = tx.send(hwnd as isize);
                    unsafe {
                        let mut msg: MSG = std::mem::zeroed();
                        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                })
                .ok();
            let hwnd = rx.recv().unwrap_or(0);
            *HANDLES.lock().unwrap() = Some(Handles { mutex: hmutex as _, hwnd });
        }

        true
    }

    pub fn destroy() {
        if let Some(handles) = HANDLES.lock().unwrap().take() {
            unsafe {
                ReleaseMutex(handles.mutex as _);
                CloseHandle(handles.mutex as _);
                PostMessageW(handles.hwnd as _, WM_CLOSE, 0, 0);
            }
        }
    }

    unsafe extern "system" fn single_instance_window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CREATE => {
                let create_struct = &*(lparam as *const CREATESTRUCTW);
                let userdata = create_struct.lpCreateParams as *const UserData;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, userdata as _);
                0
            }

            WM_COPYDATA => {
                let cds_ptr = lparam as *const COPYDATASTRUCT;
                if (*cds_ptr).dwData == WMCOPYDATA_SINGLE_INSTANCE_DATA {
                    let userdata = UserData::from_hwnd(hwnd);

                    let data = CStr::from_ptr((*cds_ptr).lpData as _).to_string_lossy();
                    let mut s = data.split('|');
                    let cwd = s.next().unwrap();
                    let args = s.map(|s| s.to_string()).collect();

                    userdata.run_callback(args, cwd.to_string());
                }
                1
            }

            WM_DESTROY => {
                let userdata = UserData::from_hwnd_raw(hwnd);
                drop(Box::from_raw(userdata));
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    fn create_event_target_window(class_name: &[u16], window_name: &[u16], userdata: *const UserData) -> HWND {
        unsafe {
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(single_instance_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: GetModuleHandleW(std::ptr::null()),
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };

            RegisterClassExW(&class);

            let hwnd = CreateWindowExW(
                WS_EX_NOACTIVATE
                | WS_EX_TRANSPARENT
                | WS_EX_LAYERED
                // WS_EX_TOOLWINDOW prevents this window from ever showing up in the taskbar, which
                // we want to avoid. If you remove this style, this window won't show up in the
                // taskbar *initially*, but it can show up at some later point. This can sometimes
                // happen on its own after several hours have passed, although this has proven
                // difficult to reproduce. Alternatively, it can be manually triggered by killing
                // `explorer.exe` and then starting the process back up.
                // It is unclear why the bug is triggered by waiting for several hours.
                | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                window_name.as_ptr(),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                userdata as _,
            );
            SetWindowLongPtrW(
                hwnd,
                GWL_STYLE,
                // The window technically has to be visible to receive WM_PAINT messages (which are used
                // for delivering events during resizes), but it isn't displayed to the user because of
                // the LAYERED style.
                (WS_VISIBLE | WS_POPUP) as isize,
            );
            hwnd
        }
    }

    pub fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
        std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
            .chain(std::iter::once(0))
            .collect()
    }

    #[cfg(target_pointer_width = "32")]
    #[allow(non_snake_case)]
    unsafe fn SetWindowLongPtrW(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
        w32wm::SetWindowLongW(hwnd, index, value as _) as _
    }

    #[cfg(target_pointer_width = "64")]
    #[allow(non_snake_case)]
    unsafe fn SetWindowLongPtrW(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
        w32wm::SetWindowLongPtrW(hwnd, index, value)
    }

    #[cfg(target_pointer_width = "32")]
    #[allow(non_snake_case)]
    unsafe fn GetWindowLongPtrW(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
        w32wm::GetWindowLongW(hwnd, index) as _
    }

    #[cfg(target_pointer_width = "64")]
    #[allow(non_snake_case)]
    unsafe fn GetWindowLongPtrW(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
        w32wm::GetWindowLongPtrW(hwnd, index)
    }
}

#[cfg(target_os = "linux")]
mod platform_impl {
    use super::{SingleInstanceCallback, IDENTIFIER};
    use std::sync::Mutex;
    use zbus::{blocking::Connection, interface, names::WellKnownName};

    struct ConnectionHandle(Connection, String);

    static CONNECTION: Mutex<Option<ConnectionHandle>> = Mutex::new(None);

    struct SingleInstanceDBus {
        callback: Mutex<Box<SingleInstanceCallback>>,
    }

    #[interface(name = "org.SingleInstance.DBus")]
    impl SingleInstanceDBus {
        fn execute_callback(&self, argv: Vec<String>, cwd: String) {
            if let Ok(mut callback) = self.callback.lock() {
                callback(argv, cwd);
            }
        }
    }

    pub fn init(callback: Box<SingleInstanceCallback>) -> bool {
        let mut dbus_name = IDENTIFIER.to_string();
        dbus_name.push_str(".SingleInstance");

        let mut dbus_path = dbus_name.replace('.', "/").replace('-', "_");
        if !dbus_path.starts_with('/') {
            dbus_path = format!("/{dbus_path}");
        }

        let single_instance_dbus = SingleInstanceDBus {
            callback: Mutex::new(callback),
        };

        let builder = match zbus::blocking::connection::Builder::session()
            .and_then(|b| b.name(dbus_name.as_str()))
            .map(|b| b.replace_existing_names(false).allow_name_replacements(false))
            .and_then(|b| b.serve_at(dbus_path.as_str(), single_instance_dbus))
        {
            Ok(builder) => builder,
            Err(_) => return true,
        };

        match builder.build() {
            Ok(connection) => {
                *CONNECTION.lock().unwrap() = Some(ConnectionHandle(connection, dbus_name));
            }
            Err(zbus::Error::NameTaken) => {
                if let Ok(connection) = Connection::session() {
                    let _ = connection.call_method(
                        Some(dbus_name.as_str()),
                        dbus_path.as_str(),
                        Some("org.SingleInstance.DBus"),
                        "ExecuteCallback",
                        &(
                            std::env::args().collect::<Vec<String>>(),
                            std::env::current_dir().unwrap_or_default().to_str().unwrap_or_default(),
                        ),
                    );
                }
                return false;
            }
            _ => {}
        }

        true
    }

    pub fn destroy() {
        if let Some(ConnectionHandle(connection, name)) = CONNECTION.lock().unwrap().take() {
            if let Ok(name) = WellKnownName::try_from(name) {
                let _ = connection.release_name(name);
            }
        }
    }
}
