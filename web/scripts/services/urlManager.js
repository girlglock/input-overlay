//guh
import { getAliasedParam } from "./settingsFieldMap.js";

export class UrlManager {
    constructor(utils) {
        this.urlParams = new URLSearchParams(window.location.search);
        this.isOverlayMode = this.urlParams.has("ws");
        this.utils = utils;
    }

    compressSettings(paramsString) {
        try {
            const compressed = pako.deflate(paramsString, { level: 9 });
            const base64 = btoa(String.fromCharCode.apply(null, compressed));
            return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
        } catch (e) {
            console.error("compression error:", e);
            return null;
        }
    }

    decompressSettings(compressed) {
        try {
            const base64 = compressed.replace(/-/g, "+").replace(/_/g, "/");
            const padding = "=".repeat((4 - base64.length % 4) % 4);
            const binary = atob(base64 + padding);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) {
                bytes[i] = binary.charCodeAt(i);
            }
            return pako.inflate(bytes, { to: "string" });
        } catch (e) {
            console.error("decompression error:", e);
            return null;
        }
    }

    buildURLParams(settings) {
        const params = [];

        const addParam = (key, value) => {
            if (value !== undefined && value !== null && value !== "") {
                params.push(`${key}=${value}`);
            }
        };

        if (settings.wsauth) {
            addParam("wsauth", encodeURIComponent(settings.wsauth));
        }

        const colorSettings = {
            key_glow_color: settings.key_glow_color,
            key_inactive_font_color: settings.key_inactive_font_color,
            key_inactive_bg_color: settings.key_inactive_bg_color,
            key_active_bg_color: settings.key_active_bg_color,
            key_outline_color: settings.key_outline_color,
            key_active_font_color: settings.key_active_font_color,
        };

        Object.entries(colorSettings).forEach(([wireKey, value]) => {
            if (value !== undefined && value !== null && value !== "") {
                addParam(wireKey, value.toLowerCase().replace("#", ""));
            }
        });

        if (settings.key_glow_radius && settings.key_glow_radius !== "24") addParam("key_glow_radius", settings.key_glow_radius);
        if (settings.key_border_radius_idle && settings.key_border_radius_idle !== "8") addParam("key_border_radius_idle", settings.key_border_radius_idle);
        if (settings.key_border_radius_pressed && settings.key_border_radius_pressed !== (settings.key_border_radius_idle || "8")) addParam("key_border_radius_pressed", settings.key_border_radius_pressed);
        if (settings.key_press_scale && settings.key_press_scale !== "105") addParam("key_press_scale", settings.key_press_scale);
        if (settings.key_animation_speed && settings.key_animation_speed !== "100") addParam("key_animation_speed", settings.key_animation_speed);
        if (settings.scale && settings.scale !== "100") addParam("scale", settings.scale);
        if (settings.opacity && settings.opacity !== "100") addParam("opacity", settings.opacity);

        if (settings.analog_display_mode && settings.analog_display_mode !== "fill") {
            addParam("analog_display_mode", settings.analog_display_mode);
        }
        if (settings.analog_smoothing === true || settings.analog_smoothing === "true" || settings.analog_smoothing === "1") {
            params.push("analog_smoothing=1");
        }
        if (settings.analog_legend_fill_mode && settings.analog_legend_fill_mode !== "fading") {
            addParam("analog_legend_fill_mode", settings.analog_legend_fill_mode);
        }
        if (settings.analog_force_disable === true || settings.analog_force_disable === "true" || settings.analog_force_disable === "1") {
            params.push("analog_force_disable=1");
        }
        if (settings.gamepad_deadzone && settings.gamepad_deadzone !== "3") {
            addParam("gamepad_deadzone", settings.gamepad_deadzone);
        }
        if (settings.gamepad_no_cef === true || settings.gamepad_no_cef === "true" || settings.gamepad_no_cef === "1") {
            params.push("gamepad_no_cef=1");
        }
        if (settings.key_outline_width_pressed && settings.key_outline_width_pressed !== "2") addParam("key_outline_width_pressed", settings.key_outline_width_pressed);
        if (settings.key_outline_width_idle && settings.key_outline_width_idle !== "2") addParam("key_outline_width_idle", settings.key_outline_width_idle);

        if (settings.key_font_family && settings.key_font_family.trim() !== "") {
            addParam("key_font_family", settings.key_font_family.replace(/ /g, "+"));
        }

        if (settings.key_bold_font === true || settings.key_bold_font === "true" || settings.key_bold_font === "1") {
            params.push("key_bold_font=1");
        }

        if (settings.hide_mouse === true || settings.hide_mouse === "true" || settings.hide_mouse === "1") {
            params.push("hide_mouse=1");
        }
        if (settings.scroll_hide_combo === true || settings.scroll_hide_combo === "true" || settings.scroll_hide_combo === "1") {
            params.push("scroll_hide_combo=1");
        }

        if (settings.mouse_trail_sensitivity && settings.mouse_trail_sensitivity !== "100") {
            addParam("mouse_trail_sensitivity", settings.mouse_trail_sensitivity);
        }
        if (settings.mouse_trail_fade_duration != null && settings.mouse_trail_fade_duration !== "600") {
            addParam("mouse_trail_fade_duration", settings.mouse_trail_fade_duration);
        }
        if (settings.mouse_trail_mode && settings.mouse_trail_mode !== "wrap") {
            addParam("mouse_trail_mode", settings.mouse_trail_mode);
        }
        if (settings.mouse_trail_length && settings.mouse_trail_length !== "150") {
            addParam("mouse_trail_length", settings.mouse_trail_length);
        }
        if (settings.mouse_trail_highlight_m1 === true || settings.mouse_trail_highlight_m1 === "true" || settings.mouse_trail_highlight_m1 === "1") {
            params.push("mouse_trail_highlight_m1=1");
        }
        if (settings.mouse_pad_texture_url && settings.mouse_pad_texture_url.trim() !== "") {
            addParam("mouse_pad_texture_url", encodeURIComponent(settings.mouse_pad_texture_url.trim()));
        }
        if (settings.mouse_pad_texture_zoom && parseFloat(settings.mouse_pad_texture_zoom) !== 1) {
            addParam("mouse_pad_texture_zoom", settings.mouse_pad_texture_zoom);
        }
        if (settings.mouse_pad_texture_opacity && parseFloat(settings.mouse_pad_texture_opacity) !== 1) {
            addParam("mouse_pad_texture_opacity", settings.mouse_pad_texture_opacity);
        }
        if (settings.mouse_show_distance === true || settings.mouse_show_distance === "true" || settings.mouse_show_distance === "1") {
            params.push("mouse_show_distance=1");
        }
        if (settings.mouse_distance_dpi && settings.mouse_distance_dpi !== "400") {
            addParam("mouse_distance_dpi", settings.mouse_distance_dpi);
        }
        if (settings.mouse_reset_distance_after_fade === true || settings.mouse_reset_distance_after_fade === "true" || settings.mouse_reset_distance_after_fade === "1") {
            params.push("mouse_reset_distance_after_fade=1");
        }

        if (settings.keyLayout) {
            params.push(`keyLayout=${settings.keyLayout}`);
        }

        return params.join("&");
    }

    _settingsFromParams(get, keyLayoutFallback) {
        const key_border_radius_idle = get("key_border_radius_idle") || "8";
        return {
            key_glow_color: this.utils.normalizeColorValue(get("key_glow_color")) || "#8b5cf6",
            key_inactive_font_color: this.utils.normalizeColorValue(get("key_inactive_font_color")) || "#808080",
            key_inactive_bg_color: this.utils.normalizeColorValue(get("key_inactive_bg_color")) || "#1a1a1ad1",
            key_active_bg_color: this.utils.normalizeColorValue(get("key_active_bg_color")) || "#202020",
            key_outline_color: this.utils.normalizeColorValue(get("key_outline_color")) || "#4f4f4f",
            key_active_font_color: this.utils.normalizeColorValue(get("key_active_font_color")) || "#ffffff",
            key_glow_radius: get("key_glow_radius") || "24",
            key_border_radius_idle,
            key_border_radius_pressed: get("key_border_radius_pressed") || key_border_radius_idle,
            key_press_scale: get("key_press_scale") || "105",
            key_animation_speed: get("key_animation_speed") || "100",
            scale: get("scale") || "100",
            opacity: get("opacity") || "100",
            key_font_family: get("key_font_family") || "",
            key_bold_font: get("key_bold_font") === "1",
            hide_mouse: get("hide_mouse") === "1",
            scroll_hide_combo: get("scroll_hide_combo") === "1",
            mouse_trail_sensitivity: get("mouse_trail_sensitivity") || "100",
            mouse_trail_fade_duration: get("mouse_trail_fade_duration") ?? "600",
            mouse_trail_mode: get("mouse_trail_mode") || "wrap",
            mouse_trail_length: get("mouse_trail_length") || "150",
            mouse_trail_highlight_m1: get("mouse_trail_highlight_m1") === "1",
            mouse_pad_texture_url: get("mouse_pad_texture_url") ? decodeURIComponent(get("mouse_pad_texture_url")) : "",
            mouse_pad_texture_zoom: get("mouse_pad_texture_zoom") || "1",
            mouse_pad_texture_opacity: get("mouse_pad_texture_opacity") || "1",
            mouse_show_distance: get("mouse_show_distance") === "1",
            mouse_distance_dpi: get("mouse_distance_dpi") || "400",
            mouse_reset_distance_after_fade: get("mouse_reset_distance_after_fade") === "1",
            key_gap_scale: get("key_gap_scale") || "100",
            key_outline_width_pressed: get("key_outline_width_pressed") || "2",
            key_outline_width_idle: get("key_outline_width_idle") || "2",
            analog_display_mode: get("analog_display_mode") || "fill",
            analog_smoothing: get("analog_smoothing") === "1",
            analog_legend_fill_mode: get("analog_legend_fill_mode") || "fading",
            analog_force_disable: get("analog_force_disable") === "1",
            gamepad_deadzone: get("gamepad_deadzone") || "3",
            gamepad_no_cef: get("gamepad_no_cef") === "1",
            wsauth: get("wsauth") || "",
            keyLayout: get("keyLayout") || keyLayoutFallback?.get("keyLayout") || null,
            customLayoutRow1: get("customLayoutRow1") || null,
            customLayoutRow2: get("customLayoutRow2") || null,
            customLayoutRow3: get("customLayoutRow3") || null,
            customLayoutRow4: get("customLayoutRow4") || null,
            customLayoutRow5: get("customLayoutRow5") || null,
            customLayoutMouse: get("customLayoutMouse") || null,
        };
    }

    getOverlaySettings() {
        const params = this.urlParams;
        if (params.has("cfg")) {
            const compressed = params.get("cfg");
            const decompressed = this.decompressSettings(compressed);

            if (decompressed) {
                const decompressedParams = new URLSearchParams(decompressed);
                const get = (key) => getAliasedParam((k) => decompressedParams.get(k), key);
                return this._settingsFromParams(get, this.urlParams);
            }
        }

        const get = (key) => getAliasedParam((k) => params.get(k), key);
        return this._settingsFromParams(get, null);
    }
}