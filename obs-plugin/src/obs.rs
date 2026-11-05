use std::ffi::c_char;
use std::sync::OnceLock;

#[repr(C)]
pub struct ObsData(());
#[repr(C)]
pub struct ObsProperties(());
#[repr(C)]
pub struct ObsProperty(());
#[repr(C)]
pub struct ObsSource(());
#[repr(C)]
pub struct ObsEffect(());
#[repr(C)]
pub struct ObsEffectParam(());
#[repr(C)]
pub struct ObsTexture(());
#[repr(C)]
pub struct ObsTexrender(());
#[repr(C)]
pub struct ObsStagesurf(());

#[repr(C, align(16))]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
pub struct ObsSourceInfo {
    pub id: *const c_char,
    pub type_: i32,
    pub output_flags: u32,
    pub get_name: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> *const c_char>,
    pub create: Option<unsafe extern "C" fn(*mut ObsData, *mut ObsSource) -> *mut std::ffi::c_void>,
    pub destroy: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub get_width: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> u32>,
    pub get_height: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> u32>,
    pub get_defaults: Option<unsafe extern "C" fn(*mut ObsData)>,
    pub get_properties: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> *mut ObsProperties>,
    pub update: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ObsData)>,
    pub activate: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub deactivate: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub show: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub hide: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub video_tick: Option<unsafe extern "C" fn(*mut std::ffi::c_void, f32)>,
    pub video_render: Option<unsafe extern "C" fn(*mut std::ffi::c_void, *mut ObsEffect)>,
}
unsafe impl Send for ObsSourceInfo {}
unsafe impl Sync for ObsSourceInfo {}

pub const OBS_SOURCE_TYPE_INPUT: i32 = 0;
pub const OBS_SOURCE_VIDEO: u32 = 1 << 0;
pub const OBS_TEXT_DEFAULT: i32 = 0;
pub const OBS_TEXT_INFO: i32 = 3;
pub const OBS_GROUP_NORMAL: i32 = 1;
pub const OBS_COMBO_TYPE_LIST: i32 = 2;
pub const OBS_COMBO_FORMAT_STRING: i32 = 3;
pub const OBS_PATH_FILE: i32 = 0;

pub const GS_COLOR_FORMAT_RGBA: i32 = 3;
pub const GS_TEXTURE_DYNAMIC: u32 = 1 << 1;
pub const GS_ZS_NONE: i32 = 0;
pub const GS_CLEAR_COLOR: u32 = 1 << 0;

pub const GS_BLEND_ZERO: i32 = 0;
pub const GS_BLEND_ONE: i32 = 1;
pub const GS_BLEND_INVSRCALPHA: i32 = 5;

type FnRegisterSourceS = unsafe extern "C" fn(*const ObsSourceInfo, usize);
type FnDataSetDefaultString = unsafe extern "C" fn(*mut ObsData, *const c_char, *const c_char);
type FnDataSetDefaultBool = unsafe extern "C" fn(*mut ObsData, *const c_char, bool);
type FnDataGetString = unsafe extern "C" fn(*mut ObsData, *const c_char) -> *const c_char;
type FnDataGetBool = unsafe extern "C" fn(*mut ObsData, *const c_char) -> bool;
type FnPropertiesCreate = unsafe extern "C" fn() -> *mut ObsProperties;
type FnPropertiesAddText =
    unsafe extern "C" fn(*mut ObsProperties, *const c_char, *const c_char, i32) -> *mut ObsProperty;
type FnPropertiesAddBool =
    unsafe extern "C" fn(*mut ObsProperties, *const c_char, *const c_char) -> *mut ObsProperty;
type FnPropertiesAddPath = unsafe extern "C" fn(
    *mut ObsProperties,
    *const c_char,
    *const c_char,
    i32,
    *const c_char,
    *const c_char,
) -> *mut ObsProperty;
type FnPropertiesAddGroup = unsafe extern "C" fn(
    *mut ObsProperties,
    *const c_char,
    *const c_char,
    i32,
    *mut ObsProperties,
) -> *mut ObsProperty;
type FnPropertiesAddList = unsafe extern "C" fn(
    *mut ObsProperties,
    *const c_char,
    *const c_char,
    i32,
    i32,
) -> *mut ObsProperty;
type FnPropertyListAddString =
    unsafe extern "C" fn(*mut ObsProperty, *const c_char, *const c_char) -> usize;
type FnPropertiesGet = unsafe extern "C" fn(*mut ObsProperties, *const c_char) -> *mut ObsProperty;
type FnPropertySetVisible = unsafe extern "C" fn(*mut ObsProperty, bool);
type ObsPropertyModifiedFn =
    unsafe extern "C" fn(*mut ObsProperties, *mut ObsProperty, *mut ObsData) -> bool;
type FnPropertySetModifiedCallback =
    unsafe extern "C" fn(*mut ObsProperty, Option<ObsPropertyModifiedFn>);

type FnEnterGraphics = unsafe extern "C" fn();
type FnLeaveGraphics = unsafe extern "C" fn();
type FnTextureCreate =
    unsafe extern "C" fn(u32, u32, i32, u32, *const *const u8, u32) -> *mut ObsTexture;
type FnTextureDestroy = unsafe extern "C" fn(*mut ObsTexture);
type FnTextureSetImage = unsafe extern "C" fn(*mut ObsTexture, *const u8, u32, bool);
type FnEffectGetParamByName =
    unsafe extern "C" fn(*mut ObsEffect, *const c_char) -> *mut ObsEffectParam;
type FnEffectSetTexture = unsafe extern "C" fn(*mut ObsEffectParam, *mut ObsTexture);
type FnDrawSprite = unsafe extern "C" fn(*mut ObsTexture, u32, u32, u32);
type FnBlendStatePush = unsafe extern "C" fn();
type FnBlendStatePop = unsafe extern "C" fn();
type FnBlendFunction = unsafe extern "C" fn(i32, i32);
type FnDataCreate = unsafe extern "C" fn() -> *mut ObsData;
type FnDataRelease = unsafe extern "C" fn(*mut ObsData);
type FnDataSetString = unsafe extern "C" fn(*mut ObsData, *const c_char, *const c_char);
type FnDataSetBool = unsafe extern "C" fn(*mut ObsData, *const c_char, bool);
type FnSourceCreatePrivate = unsafe extern "C" fn(*const c_char, *const c_char, *mut ObsData) -> *mut ObsSource;
type FnSourceRelease = unsafe extern "C" fn(*mut ObsSource);
type FnSourceAddActiveChild = unsafe extern "C" fn(*mut ObsSource, *mut ObsSource) -> bool;
type FnSourceRemoveActiveChild = unsafe extern "C" fn(*mut ObsSource, *mut ObsSource);
type FnSourceGetWidth = unsafe extern "C" fn(*mut ObsSource) -> u32;
type FnSourceGetHeight = unsafe extern "C" fn(*mut ObsSource) -> u32;
type FnSourceVideoRender = unsafe extern "C" fn(*mut ObsSource);
type FnTexrenderCreate = unsafe extern "C" fn(i32, i32) -> *mut ObsTexrender;
type FnTexrenderDestroy = unsafe extern "C" fn(*mut ObsTexrender);
type FnTexrenderReset = unsafe extern "C" fn(*mut ObsTexrender);
type FnTexrenderBegin = unsafe extern "C" fn(*mut ObsTexrender, u32, u32) -> bool;
type FnTexrenderEnd = unsafe extern "C" fn(*mut ObsTexrender);
type FnTexrenderGetTexture = unsafe extern "C" fn(*mut ObsTexrender) -> *mut ObsTexture;
type FnClear = unsafe extern "C" fn(u32, *const Vec4, f32, u8);
type FnOrtho = unsafe extern "C" fn(f32, f32, f32, f32, f32, f32);
type FnStagesurfaceCreate = unsafe extern "C" fn(u32, u32, i32) -> *mut ObsStagesurf;
type FnStagesurfaceDestroy = unsafe extern "C" fn(*mut ObsStagesurf);
type FnStageTexture = unsafe extern "C" fn(*mut ObsStagesurf, *mut ObsTexture);
type FnStagesurfaceMap = unsafe extern "C" fn(*mut ObsStagesurf, *mut *mut u8, *mut u32) -> bool;
type FnStagesurfaceUnmap = unsafe extern "C" fn(*mut ObsStagesurf);

pub struct ObsApi {
    pub register_source_s: FnRegisterSourceS,
    pub data_set_default_string: FnDataSetDefaultString,
    pub data_set_default_bool: FnDataSetDefaultBool,
    pub data_get_string: FnDataGetString,
    pub data_get_bool: FnDataGetBool,
    pub properties_create: FnPropertiesCreate,
    pub properties_add_text: FnPropertiesAddText,
    pub properties_add_bool: FnPropertiesAddBool,
    pub properties_add_path: FnPropertiesAddPath,
    pub properties_add_group: FnPropertiesAddGroup,
    pub properties_add_list: FnPropertiesAddList,
    pub property_list_add_string: FnPropertyListAddString,
    pub properties_get: FnPropertiesGet,
    pub property_set_visible: FnPropertySetVisible,
    pub property_set_modified_callback: FnPropertySetModifiedCallback,
    pub enter_graphics: FnEnterGraphics,
    pub leave_graphics: FnLeaveGraphics,
    pub texture_create: FnTextureCreate,
    pub texture_destroy: FnTextureDestroy,
    pub texture_set_image: FnTextureSetImage,
    pub effect_get_param_by_name: FnEffectGetParamByName,
    pub effect_set_texture: FnEffectSetTexture,
    pub draw_sprite: FnDrawSprite,
    pub blend_state_push: FnBlendStatePush,
    pub blend_state_pop: FnBlendStatePop,
    pub blend_function: FnBlendFunction,
    pub data_create: FnDataCreate,
    pub data_release: FnDataRelease,
    pub data_set_string: FnDataSetString,
    pub data_set_bool: FnDataSetBool,
    pub source_create_private: FnSourceCreatePrivate,
    pub source_release: FnSourceRelease,
    pub source_add_active_child: FnSourceAddActiveChild,
    pub source_remove_active_child: FnSourceRemoveActiveChild,
    pub source_get_width: FnSourceGetWidth,
    pub source_get_height: FnSourceGetHeight,
    pub source_video_render: FnSourceVideoRender,
    pub texrender_create: FnTexrenderCreate,
    pub texrender_destroy: FnTexrenderDestroy,
    pub texrender_reset: FnTexrenderReset,
    pub texrender_begin: FnTexrenderBegin,
    pub texrender_end: FnTexrenderEnd,
    pub texrender_get_texture: FnTexrenderGetTexture,
    pub clear: FnClear,
    pub ortho: FnOrtho,
    pub stagesurface_create: FnStagesurfaceCreate,
    pub stagesurface_destroy: FnStagesurfaceDestroy,
    pub stage_texture: FnStageTexture,
    pub stagesurface_map: FnStagesurfaceMap,
    pub stagesurface_unmap: FnStagesurfaceUnmap,
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
            let proc =
                unsafe { GetProcAddress(hmod, PCSTR(concat!($name, "\0").as_bytes().as_ptr())) };
            match proc {
                Some(p) => unsafe {
                    std::mem::transmute::<unsafe extern "system" fn() -> isize, $ty>(p)
                },
                None => {
                    tracing::error!("obs api: '{}' not found in '{}'", $name, $module);
                    return false;
                }
            }
        }};
    }

    let api = ObsApi {
        register_source_s: load!("obs.dll", "obs_register_source_s", FnRegisterSourceS),
        data_set_default_string: load!(
            "obs.dll",
            "obs_data_set_default_string",
            FnDataSetDefaultString
        ),
        data_set_default_bool: load!("obs.dll", "obs_data_set_default_bool", FnDataSetDefaultBool),
        data_get_string: load!("obs.dll", "obs_data_get_string", FnDataGetString),
        data_get_bool: load!("obs.dll", "obs_data_get_bool", FnDataGetBool),
        properties_create: load!("obs.dll", "obs_properties_create", FnPropertiesCreate),
        properties_add_text: load!("obs.dll", "obs_properties_add_text", FnPropertiesAddText),
        properties_add_bool: load!("obs.dll", "obs_properties_add_bool", FnPropertiesAddBool),
        properties_add_path: load!("obs.dll", "obs_properties_add_path", FnPropertiesAddPath),
        properties_add_group: load!("obs.dll", "obs_properties_add_group", FnPropertiesAddGroup),
        properties_add_list: load!("obs.dll", "obs_properties_add_list", FnPropertiesAddList),
        property_list_add_string: load!(
            "obs.dll",
            "obs_property_list_add_string",
            FnPropertyListAddString
        ),
        properties_get: load!("obs.dll", "obs_properties_get", FnPropertiesGet),
        property_set_visible: load!("obs.dll", "obs_property_set_visible", FnPropertySetVisible),
        property_set_modified_callback: load!(
            "obs.dll",
            "obs_property_set_modified_callback",
            FnPropertySetModifiedCallback
        ),
        enter_graphics: load!("obs.dll", "obs_enter_graphics", FnEnterGraphics),
        leave_graphics: load!("obs.dll", "obs_leave_graphics", FnLeaveGraphics),
        texture_create: load!("obs.dll", "gs_texture_create", FnTextureCreate),
        texture_destroy: load!("obs.dll", "gs_texture_destroy", FnTextureDestroy),
        texture_set_image: load!("obs.dll", "gs_texture_set_image", FnTextureSetImage),
        effect_get_param_by_name: load!(
            "obs.dll",
            "gs_effect_get_param_by_name",
            FnEffectGetParamByName
        ),
        effect_set_texture: load!("obs.dll", "gs_effect_set_texture", FnEffectSetTexture),
        draw_sprite: load!("obs.dll", "gs_draw_sprite", FnDrawSprite),
        blend_state_push: load!("obs.dll", "gs_blend_state_push", FnBlendStatePush),
        blend_state_pop: load!("obs.dll", "gs_blend_state_pop", FnBlendStatePop),
        blend_function: load!("obs.dll", "gs_blend_function", FnBlendFunction),
        data_create: load!("obs.dll", "obs_data_create", FnDataCreate),
        data_release: load!("obs.dll", "obs_data_release", FnDataRelease),
        data_set_string: load!("obs.dll", "obs_data_set_string", FnDataSetString),
        data_set_bool: load!("obs.dll", "obs_data_set_bool", FnDataSetBool),
        source_create_private: load!("obs.dll", "obs_source_create_private", FnSourceCreatePrivate),
        source_release: load!("obs.dll", "obs_source_release", FnSourceRelease),
        source_add_active_child: load!("obs.dll", "obs_source_add_active_child", FnSourceAddActiveChild),
        source_remove_active_child: load!("obs.dll", "obs_source_remove_active_child", FnSourceRemoveActiveChild),
        source_get_width: load!("obs.dll", "obs_source_get_width", FnSourceGetWidth),
        source_get_height: load!("obs.dll", "obs_source_get_height", FnSourceGetHeight),
        source_video_render: load!("obs.dll", "obs_source_video_render", FnSourceVideoRender),
        texrender_create: load!("obs.dll", "gs_texrender_create", FnTexrenderCreate),
        texrender_destroy: load!("obs.dll", "gs_texrender_destroy", FnTexrenderDestroy),
        texrender_reset: load!("obs.dll", "gs_texrender_reset", FnTexrenderReset),
        texrender_begin: load!("obs.dll", "gs_texrender_begin", FnTexrenderBegin),
        texrender_end: load!("obs.dll", "gs_texrender_end", FnTexrenderEnd),
        texrender_get_texture: load!("obs.dll", "gs_texrender_get_texture", FnTexrenderGetTexture),
        clear: load!("obs.dll", "gs_clear", FnClear),
        ortho: load!("obs.dll", "gs_ortho", FnOrtho),
        stagesurface_create: load!("obs.dll", "gs_stagesurface_create", FnStagesurfaceCreate),
        stagesurface_destroy: load!("obs.dll", "gs_stagesurface_destroy", FnStagesurfaceDestroy),
        stage_texture: load!("obs.dll", "gs_stage_texture", FnStageTexture),
        stagesurface_map: load!("obs.dll", "gs_stagesurface_map", FnStagesurfaceMap),
        stagesurface_unmap: load!("obs.dll", "gs_stagesurface_unmap", FnStagesurfaceUnmap),
    };

    match OBS_API.set(api) {
        Ok(_) => {
            tracing::info!("obs api loaded");
            true
        }
        Err(_) => {
            tracing::warn!("obs api already initialized");
            true
        }
    }
}

#[cfg(target_os = "linux")]
pub fn init() -> bool {
    fn try_dlopen(names: &[&str]) -> *mut libc::c_void {
        for &name in names {
            if let Ok(cname) = std::ffi::CString::new(name) {
                let h = unsafe { libc::dlopen(cname.as_ptr(), libc::RTLD_NOLOAD | libc::RTLD_NOW) };
                if !h.is_null() {
                    return h;
                }
            }
        }
        std::ptr::null_mut()
    }

    let hobs = try_dlopen(&["libobs.so.0", "libobs.so"]);
    if hobs.is_null() {
        tracing::error!("obs api: libobs not found in process");
        return false;
    }

    macro_rules! load {
        ($handle:expr, $name:literal, $ty:ty) => {{
            let sym = unsafe {
                libc::dlsym(
                    $handle,
                    concat!($name, "\0").as_ptr() as *const libc::c_char,
                )
            };
            if sym.is_null() {
                tracing::error!("obs api: '{}' not found", $name);
                return false;
            }
            unsafe { std::mem::transmute::<*mut libc::c_void, $ty>(sym) }
        }};
    }

    let api = ObsApi {
        register_source_s: load!(hobs, "obs_register_source_s", FnRegisterSourceS),
        data_set_default_string: load!(hobs, "obs_data_set_default_string", FnDataSetDefaultString),
        data_set_default_bool: load!(hobs, "obs_data_set_default_bool", FnDataSetDefaultBool),
        data_get_string: load!(hobs, "obs_data_get_string", FnDataGetString),
        data_get_bool: load!(hobs, "obs_data_get_bool", FnDataGetBool),
        properties_create: load!(hobs, "obs_properties_create", FnPropertiesCreate),
        properties_add_text: load!(hobs, "obs_properties_add_text", FnPropertiesAddText),
        properties_add_bool: load!(hobs, "obs_properties_add_bool", FnPropertiesAddBool),
        properties_add_path: load!(hobs, "obs_properties_add_path", FnPropertiesAddPath),
        properties_add_group: load!(hobs, "obs_properties_add_group", FnPropertiesAddGroup),
        properties_add_list: load!(hobs, "obs_properties_add_list", FnPropertiesAddList),
        property_list_add_string: load!(
            hobs,
            "obs_property_list_add_string",
            FnPropertyListAddString
        ),
        properties_get: load!(hobs, "obs_properties_get", FnPropertiesGet),
        property_set_visible: load!(hobs, "obs_property_set_visible", FnPropertySetVisible),
        property_set_modified_callback: load!(
            hobs,
            "obs_property_set_modified_callback",
            FnPropertySetModifiedCallback
        ),
        enter_graphics: load!(hobs, "obs_enter_graphics", FnEnterGraphics),
        leave_graphics: load!(hobs, "obs_leave_graphics", FnLeaveGraphics),
        texture_create: load!(hobs, "gs_texture_create", FnTextureCreate),
        texture_destroy: load!(hobs, "gs_texture_destroy", FnTextureDestroy),
        texture_set_image: load!(hobs, "gs_texture_set_image", FnTextureSetImage),
        effect_get_param_by_name: load!(
            hobs,
            "gs_effect_get_param_by_name",
            FnEffectGetParamByName
        ),
        effect_set_texture: load!(hobs, "gs_effect_set_texture", FnEffectSetTexture),
        draw_sprite: load!(hobs, "gs_draw_sprite", FnDrawSprite),
        blend_state_push: load!(hobs, "gs_blend_state_push", FnBlendStatePush),
        blend_state_pop: load!(hobs, "gs_blend_state_pop", FnBlendStatePop),
        blend_function: load!(hobs, "gs_blend_function", FnBlendFunction),
        data_create: load!(hobs, "obs_data_create", FnDataCreate),
        data_release: load!(hobs, "obs_data_release", FnDataRelease),
        data_set_string: load!(hobs, "obs_data_set_string", FnDataSetString),
        data_set_bool: load!(hobs, "obs_data_set_bool", FnDataSetBool),
        source_create_private: load!(hobs, "obs_source_create_private", FnSourceCreatePrivate),
        source_release: load!(hobs, "obs_source_release", FnSourceRelease),
        source_add_active_child: load!(hobs, "obs_source_add_active_child", FnSourceAddActiveChild),
        source_remove_active_child: load!(hobs, "obs_source_remove_active_child", FnSourceRemoveActiveChild),
        source_get_width: load!(hobs, "obs_source_get_width", FnSourceGetWidth),
        source_get_height: load!(hobs, "obs_source_get_height", FnSourceGetHeight),
        source_video_render: load!(hobs, "obs_source_video_render", FnSourceVideoRender),
        texrender_create: load!(hobs, "gs_texrender_create", FnTexrenderCreate),
        texrender_destroy: load!(hobs, "gs_texrender_destroy", FnTexrenderDestroy),
        texrender_reset: load!(hobs, "gs_texrender_reset", FnTexrenderReset),
        texrender_begin: load!(hobs, "gs_texrender_begin", FnTexrenderBegin),
        texrender_end: load!(hobs, "gs_texrender_end", FnTexrenderEnd),
        texrender_get_texture: load!(hobs, "gs_texrender_get_texture", FnTexrenderGetTexture),
        clear: load!(hobs, "gs_clear", FnClear),
        ortho: load!(hobs, "gs_ortho", FnOrtho),
        stagesurface_create: load!(hobs, "gs_stagesurface_create", FnStagesurfaceCreate),
        stagesurface_destroy: load!(hobs, "gs_stagesurface_destroy", FnStagesurfaceDestroy),
        stage_texture: load!(hobs, "gs_stage_texture", FnStageTexture),
        stagesurface_map: load!(hobs, "gs_stagesurface_map", FnStagesurfaceMap),
        stagesurface_unmap: load!(hobs, "gs_stagesurface_unmap", FnStagesurfaceUnmap),
    };

    match OBS_API.set(api) {
        Ok(_) => {
            tracing::info!("obs api loaded");
            true
        }
        Err(_) => {
            tracing::warn!("obs api already initialized");
            true
        }
    }
}
