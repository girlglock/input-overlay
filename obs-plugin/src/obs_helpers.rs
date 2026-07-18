use crate::obs::{
    ObsApi, ObsData, ObsProperties, ObsProperty, OBS_COMBO_FORMAT_STRING, OBS_COMBO_TYPE_LIST,
    OBS_GROUP_NORMAL, OBS_PATH_FILE, OBS_TEXT_INFO,
};

pub unsafe fn text(
    api: &ObsApi,
    p: *mut ObsProperties,
    id: &[u8],
    label: &[u8],
    kind: i32,
) -> *mut ObsProperty {
    (api.properties_add_text)(p, id.as_ptr() as _, label.as_ptr() as _, kind)
}

pub unsafe fn checkbox(
    api: &ObsApi,
    p: *mut ObsProperties,
    id: &[u8],
    label: &[u8],
) -> *mut ObsProperty {
    (api.properties_add_bool)(p, id.as_ptr() as _, label.as_ptr() as _)
}

pub unsafe fn file_path(
    api: &ObsApi,
    p: *mut ObsProperties,
    id: &[u8],
    label: &[u8],
    filter: &[u8],
) -> *mut ObsProperty {
    (api.properties_add_path)(
        p,
        id.as_ptr() as _,
        label.as_ptr() as _,
        OBS_PATH_FILE,
        filter.as_ptr() as _,
        std::ptr::null(),
    )
}

pub unsafe fn set_visible(api: &ObsApi, prop: *mut ObsProperty, visible: bool) {
    if !prop.is_null() {
        (api.property_set_visible)(prop, visible);
    }
}

pub unsafe fn info(api: &ObsApi, p: *mut ObsProperties, id: &[u8], msg: &[u8]) {
    (api.properties_add_text)(p, id.as_ptr() as _, msg.as_ptr() as _, OBS_TEXT_INFO);
}

pub unsafe fn list(
    api: &ObsApi,
    p: *mut ObsProperties,
    id: &[u8],
    label: &[u8],
) -> *mut ObsProperty {
    (api.properties_add_list)(
        p,
        id.as_ptr() as _,
        label.as_ptr() as _,
        OBS_COMBO_TYPE_LIST,
        OBS_COMBO_FORMAT_STRING,
    )
}

pub unsafe fn list_item(api: &ObsApi, prop: *mut ObsProperty, label: &[u8], val: &[u8]) {
    (api.property_list_add_string)(prop, label.as_ptr() as _, val.as_ptr() as _);
}

pub unsafe fn group(
    api: &ObsApi,
    parent: *mut ObsProperties,
    id: &[u8],
    label: &[u8],
    f: impl FnOnce(*mut ObsProperties),
) {
    let g = (api.properties_create)();
    f(g);
    (api.properties_add_group)(
        parent,
        id.as_ptr() as _,
        label.as_ptr() as _,
        OBS_GROUP_NORMAL,
        g,
    );
}

pub unsafe fn read_str(api: &ObsApi, data: *mut ObsData, name: &[u8]) -> String {
    let ptr = (api.data_get_string)(data, name.as_ptr() as *const _);
    if ptr.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(ptr)
        .to_str()
        .unwrap_or("")
        .to_string()
}
