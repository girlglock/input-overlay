use std::ffi::c_char;
use std::sync::OnceLock;

#[repr(C)] pub struct ObsData(());
#[repr(C)] pub struct ObsProperties(());
#[repr(C)] pub struct ObsProperty(());
#[repr(C)] pub struct ObsSource(());

#[repr(C)]
pub struct ObsSourceInfo {
    pub id:             *const c_char,
    pub type_:          i32,
    pub output_flags:   u32,
    pub get_name:       Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> *const c_char>,
    pub create:         Option<unsafe extern "C" fn(*mut ObsData, *mut ObsSource) -> *mut std::ffi::c_void>,
    pub destroy:        Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub get_width:      Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> u32>,
    pub get_height:     Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> u32>,
    pub get_defaults:   Option<unsafe extern "C" fn(*mut ObsData)>,
    pub get_properties: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> *mut ObsProperties>,
    pub update:         Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ObsData)>,
}
unsafe impl Send for ObsSourceInfo {}
unsafe impl Sync for ObsSourceInfo {}

pub const OBS_SOURCE_TYPE_INPUT:    i32 = 0;
pub const OBS_SOURCE_CAP_DISABLED:  u32 = 1 << 3;
pub const OBS_TEXT_DEFAULT:         i32 = 0;
pub const OBS_TEXT_PASSWORD:        i32 = 1;
pub const OBS_TEXT_INFO:            i32 = 3;
pub const OBS_GROUP_NORMAL:         i32 = 1;
pub const OBS_COMBO_TYPE_LIST:      i32 = 2;
pub const OBS_COMBO_FORMAT_STRING:  i32 = 3;

pub type ObsPropertyClickedFn =
    unsafe extern "C" fn(*mut ObsProperties, *mut ObsProperty, *mut std::ffi::c_void) -> bool;

type FnRegisterSourceS     = unsafe extern "C" fn(*const ObsSourceInfo, usize);
type FnSourceCreatePrivate = unsafe extern "C" fn(*const c_char, *const c_char, *mut ObsData) -> *mut ObsSource;
type FnSourceRelease       = unsafe extern "C" fn(*mut ObsSource);
type FnSourceUpdate        = unsafe extern "C" fn(*mut ObsSource, *mut ObsData);
type FnDataCreate          = unsafe extern "C" fn() -> *mut ObsData;
type FnDataRelease         = unsafe extern "C" fn(*mut ObsData);
type FnDataSetString       = unsafe extern "C" fn(*mut ObsData, *const c_char, *const c_char);
type FnDataSetInt          = unsafe extern "C" fn(*mut ObsData, *const c_char, i64);
type FnDataSetBool         = unsafe extern "C" fn(*mut ObsData, *const c_char, bool);
type FnDataSetDefaultString = unsafe extern "C" fn(*mut ObsData, *const c_char, *const c_char);
type FnDataSetDefaultInt   = unsafe extern "C" fn(*mut ObsData, *const c_char, i64);
type FnDataSetDefaultBool  = unsafe extern "C" fn(*mut ObsData, *const c_char, bool);
type FnDataGetString       = unsafe extern "C" fn(*mut ObsData, *const c_char) -> *const c_char;
type FnDataGetInt          = unsafe extern "C" fn(*mut ObsData, *const c_char) -> i64;
type FnDataGetBool         = unsafe extern "C" fn(*mut ObsData, *const c_char) -> bool;
type FnPropertiesCreate    = unsafe extern "C" fn() -> *mut ObsProperties;
type FnPropertiesAddText   = unsafe extern "C" fn(*mut ObsProperties, *const c_char, *const c_char, i32) -> *mut ObsProperty;
type FnPropertiesAddInt    = unsafe extern "C" fn(*mut ObsProperties, *const c_char, *const c_char, i32, i32, i32) -> *mut ObsProperty;
type FnPropertiesAddBool   = unsafe extern "C" fn(*mut ObsProperties, *const c_char, *const c_char) -> *mut ObsProperty;
type FnPropertiesAddGroup  = unsafe extern "C" fn(*mut ObsProperties, *const c_char, *const c_char, i32, *mut ObsProperties) -> *mut ObsProperty;
type FnPropertiesAddButton = unsafe extern "C" fn(*mut ObsProperties, *const c_char, *const c_char, ObsPropertyClickedFn) -> *mut ObsProperty;
type FnPropertiesAddList   = unsafe extern "C" fn(*mut ObsProperties, *const c_char, *const c_char, i32, i32) -> *mut ObsProperty;
type FnPropertyListAddString = unsafe extern "C" fn(*mut ObsProperty, *const c_char, *const c_char) -> usize;
type FnAddToolsMenuItem    = unsafe extern "C" fn(*const c_char, unsafe extern "C" fn(*mut std::ffi::c_void), *mut std::ffi::c_void);
type FnOpenSourceProperties = unsafe extern "C" fn(*mut ObsSource);

pub struct ObsApi {
    pub register_source_s:      FnRegisterSourceS,
    pub source_create_private:  FnSourceCreatePrivate,
    pub source_release:         FnSourceRelease,
    pub source_update:          FnSourceUpdate,
    pub data_create:            FnDataCreate,
    pub data_release:           FnDataRelease,
    pub data_set_string:        FnDataSetString,
    pub data_set_int:           FnDataSetInt,
    pub data_set_bool:          FnDataSetBool,
    pub data_set_default_string: FnDataSetDefaultString,
    pub data_set_default_int:   FnDataSetDefaultInt,
    pub data_set_default_bool:  FnDataSetDefaultBool,
    pub data_get_string:        FnDataGetString,
    pub data_get_int:           FnDataGetInt,
    pub data_get_bool:          FnDataGetBool,
    pub properties_create:      FnPropertiesCreate,
    pub properties_add_text:    FnPropertiesAddText,
    pub properties_add_int:     FnPropertiesAddInt,
    pub properties_add_bool:    FnPropertiesAddBool,
    pub properties_add_group:   FnPropertiesAddGroup,
    pub properties_add_button:  FnPropertiesAddButton,
    pub properties_add_list:    FnPropertiesAddList,
    pub property_list_add_string: FnPropertyListAddString,
    pub add_tools_menu_item:    FnAddToolsMenuItem,
    pub open_source_properties: FnOpenSourceProperties,
}
unsafe impl Send for ObsApi {}
unsafe impl Sync for ObsApi {}

static OBS_API: OnceLock<ObsApi> = OnceLock::new();

pub fn api() -> Option<&'static ObsApi> {
    OBS_API.get()
}

#[cfg(windows)]
pub fn init() -> bool {
    use windows::core::PCSTR;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

    macro_rules! load {
        ($module:literal, $name:literal, $ty:ty) => {{
            let hmod = unsafe {
                match GetModuleHandleA(PCSTR(concat!($module, "\0").as_bytes().as_ptr())) {
                    Ok(m) => m,
                    Err(_) => {
                        tracing::error!("obs api: module '{}' not loaded", $module);
                        return false;
                    }
                }
            };
            let proc = unsafe {
                GetProcAddress(hmod, PCSTR(concat!($name, "\0").as_bytes().as_ptr()))
            };
            match proc {
                Some(p) => unsafe { std::mem::transmute::<_, $ty>(p) },
                None => {
                    tracing::error!("obs api: '{}' not found in '{}'", $name, $module);
                    return false;
                }
            }
        }};
    }

    let api = ObsApi {
        register_source_s:       load!("obs.dll", "obs_register_source_s",        FnRegisterSourceS),
        source_create_private:   load!("obs.dll", "obs_source_create_private",    FnSourceCreatePrivate),
        source_release:          load!("obs.dll", "obs_source_release",           FnSourceRelease),
        source_update:           load!("obs.dll", "obs_source_update",            FnSourceUpdate),
        data_create:             load!("obs.dll", "obs_data_create",              FnDataCreate),
        data_release:            load!("obs.dll", "obs_data_release",             FnDataRelease),
        data_set_string:         load!("obs.dll", "obs_data_set_string",          FnDataSetString),
        data_set_int:            load!("obs.dll", "obs_data_set_int",             FnDataSetInt),
        data_set_bool:           load!("obs.dll", "obs_data_set_bool",            FnDataSetBool),
        data_set_default_string: load!("obs.dll", "obs_data_set_default_string",  FnDataSetDefaultString),
        data_set_default_int:    load!("obs.dll", "obs_data_set_default_int",     FnDataSetDefaultInt),
        data_set_default_bool:   load!("obs.dll", "obs_data_set_default_bool",    FnDataSetDefaultBool),
        data_get_string:         load!("obs.dll", "obs_data_get_string",          FnDataGetString),
        data_get_int:            load!("obs.dll", "obs_data_get_int",             FnDataGetInt),
        data_get_bool:           load!("obs.dll", "obs_data_get_bool",            FnDataGetBool),
        properties_create:       load!("obs.dll", "obs_properties_create",        FnPropertiesCreate),
        properties_add_text:     load!("obs.dll", "obs_properties_add_text",      FnPropertiesAddText),
        properties_add_int:      load!("obs.dll", "obs_properties_add_int",       FnPropertiesAddInt),
        properties_add_bool:     load!("obs.dll", "obs_properties_add_bool",      FnPropertiesAddBool),
        properties_add_group:    load!("obs.dll", "obs_properties_add_group",     FnPropertiesAddGroup),
        properties_add_button:   load!("obs.dll", "obs_properties_add_button",    FnPropertiesAddButton),
        properties_add_list:     load!("obs.dll", "obs_properties_add_list",      FnPropertiesAddList),
        property_list_add_string: load!("obs.dll", "obs_property_list_add_string", FnPropertyListAddString),
        add_tools_menu_item:     load!("obs-frontend-api.dll", "obs_frontend_add_tools_menu_item",   FnAddToolsMenuItem),
        open_source_properties:  load!("obs-frontend-api.dll", "obs_frontend_open_source_properties", FnOpenSourceProperties),
    };

    match OBS_API.set(api) {
        Ok(_)  => { tracing::info!("obs api loaded"); true }
        Err(_) => { tracing::warn!("obs api already initialized"); true }
    }
}

#[cfg(target_os = "linux")]
pub fn init() -> bool {
    fn try_dlopen(names: &[&str]) -> *mut libc::c_void {
        for &name in names {
            if let Ok(cname) = std::ffi::CString::new(name) {
                let h = unsafe { libc::dlopen(cname.as_ptr(), libc::RTLD_NOLOAD | libc::RTLD_NOW) };
                if !h.is_null() { return h; }
            }
        }
        std::ptr::null_mut()
    }

    let hobs = try_dlopen(&["libobs.so.0", "libobs.so"]);
    if hobs.is_null() {
        tracing::error!("obs api: libobs not found in process");
        return false;
    }
    let hfe = try_dlopen(&["libobs-frontend-api.so.0", "libobs-frontend-api.so"]);
    if hfe.is_null() {
        tracing::error!("obs api: libobs-frontend-api not found in process");
        return false;
    }

    macro_rules! load {
        ($handle:expr, $name:literal, $ty:ty) => {{
            let sym = unsafe {
                libc::dlsym($handle, concat!($name, "\0").as_ptr() as *const libc::c_char)
            };
            if sym.is_null() {
                tracing::error!("obs api: '{}' not found", $name);
                return false;
            }
            unsafe { std::mem::transmute::<*mut libc::c_void, $ty>(sym) }
        }};
    }

    let api = ObsApi {
        register_source_s:       load!(hobs, "obs_register_source_s",        FnRegisterSourceS),
        source_create_private:   load!(hobs, "obs_source_create_private",    FnSourceCreatePrivate),
        source_release:          load!(hobs, "obs_source_release",           FnSourceRelease),
        source_update:           load!(hobs, "obs_source_update",            FnSourceUpdate),
        data_create:             load!(hobs, "obs_data_create",              FnDataCreate),
        data_release:            load!(hobs, "obs_data_release",             FnDataRelease),
        data_set_string:         load!(hobs, "obs_data_set_string",          FnDataSetString),
        data_set_int:            load!(hobs, "obs_data_set_int",             FnDataSetInt),
        data_set_bool:           load!(hobs, "obs_data_set_bool",            FnDataSetBool),
        data_set_default_string: load!(hobs, "obs_data_set_default_string",  FnDataSetDefaultString),
        data_set_default_int:    load!(hobs, "obs_data_set_default_int",     FnDataSetDefaultInt),
        data_set_default_bool:   load!(hobs, "obs_data_set_default_bool",    FnDataSetDefaultBool),
        data_get_string:         load!(hobs, "obs_data_get_string",          FnDataGetString),
        data_get_int:            load!(hobs, "obs_data_get_int",             FnDataGetInt),
        data_get_bool:           load!(hobs, "obs_data_get_bool",            FnDataGetBool),
        properties_create:       load!(hobs, "obs_properties_create",        FnPropertiesCreate),
        properties_add_text:     load!(hobs, "obs_properties_add_text",      FnPropertiesAddText),
        properties_add_int:      load!(hobs, "obs_properties_add_int",       FnPropertiesAddInt),
        properties_add_bool:     load!(hobs, "obs_properties_add_bool",      FnPropertiesAddBool),
        properties_add_group:    load!(hobs, "obs_properties_add_group",     FnPropertiesAddGroup),
        properties_add_button:   load!(hobs, "obs_properties_add_button",    FnPropertiesAddButton),
        properties_add_list:     load!(hobs, "obs_properties_add_list",      FnPropertiesAddList),
        property_list_add_string: load!(hobs, "obs_property_list_add_string", FnPropertyListAddString),
        add_tools_menu_item:     load!(hfe,  "obs_frontend_add_tools_menu_item",   FnAddToolsMenuItem),
        open_source_properties:  load!(hfe,  "obs_frontend_open_source_properties", FnOpenSourceProperties),
    };

    match OBS_API.set(api) {
        Ok(_)  => { tracing::info!("obs api loaded"); true }
        Err(_) => { tracing::warn!("obs api already initialized"); true }
    }
}
