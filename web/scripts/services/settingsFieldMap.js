//for upconverting old url params to new ones, so old link configs wont stop working
export const LEGACY_TO_CURRENT = {
    activecolor: "key_glow_color",
    inactivecolor: "key_inactive_font_color",
    backgroundcolor: "key_inactive_bg_color",
    activebgcolor: "key_active_bg_color",
    outlinecolor: "key_outline_color",
    fontcolor: "key_active_font_color",
    glowradius: "key_glow_radius",
    glow: "key_glow_radius",
    borderradius: "key_border_radius_idle",
    radius: "key_border_radius_idle",
    pressedradius: "key_border_radius_pressed",
    pressscale: "key_press_scale",
    animationspeed: "key_animation_speed",
    speed: "key_animation_speed",
    fontfamily: "key_font_family",
    boldfont: "key_bold_font",
    outlinescaleunpressed: "key_outline_width_idle",
    outlinescalepressed: "key_outline_width_pressed",
    gapmodifier: "key_gap_scale",
    hidescrollcombo: "scroll_hide_combo",
    hidemouse: "hide_mouse",
    analogdisplaymode: "analog_display_mode",
    analogsmoothing: "analog_smoothing",
    keylegendmode: "analog_legend_fill_mode",
    forcedisableanalog: "analog_force_disable",
    gamepaddeadzone: "gamepad_deadzone",
    mousetrailsensitivity: "mouse_trail_sensitivity",
    mousetrailfadeout: "mouse_trail_fade_duration",
    mousetrailmode: "mouse_trail_mode",
    mousetraillength: "mouse_trail_length",
    mousetrailm1highlight: "mouse_trail_highlight_m1",
    mousepadtexture: "mouse_pad_texture_url",
    mousepadtexturezoom: "mouse_pad_texture_zoom",
    mousepadtextureopacity: "mouse_pad_texture_opacity",
    showmousedistance: "mouse_show_distance",
    mousedistancedpi: "mouse_distance_dpi",
    resetmousedistanceafterfade: "mouse_reset_distance_after_fade",
};

const CURRENT_TO_LEGACY = {};
for (const [legacy, current] of Object.entries(LEGACY_TO_CURRENT)) {
    (CURRENT_TO_LEGACY[current] ??= []).push(legacy);
}

export function getAliasedParam(get, currentName) {
    const direct = get(currentName);
    if (direct !== undefined && direct !== null && direct !== "") return direct;
    for (const legacy of CURRENT_TO_LEGACY[currentName] ?? []) {
        const v = get(legacy);
        if (v !== undefined && v !== null && v !== "") return v;
    }
    return direct;
}
