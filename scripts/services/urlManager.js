//guh
import {DEFAULT_LAYOUT_STRINGS} from "../consts.js";

export class UrlManager {
    constructor(utils) {
        this.urlParams = new URLSearchParams(window.location.search);
        this.isOverlayMode = this.urlParams.has("ws");
        this.utils = utils;
    }

    compressSettings(paramsString) {
        try {
            const compressed = pako.deflate(paramsString, {level: 9});
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
            return pako.inflate(bytes, {to: "string"});
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

        const colorSettings = {
            activecolor: settings.activecolor,
            inactivecolor: settings.inactivecolor,
            backgroundcolor: settings.backgroundcolor,
            activebgcolor: settings.activebgcolor,
            outlinecolor: settings.outlinecolor,
            fontcolor: settings.fontcolor
        };

        Object.entries(colorSettings).forEach(([key, value]) => {
            if (value !== undefined && value !== null && value !== "") {
                addParam(key, value.toLowerCase().replace("#", ""));
            }
        });

        addParam("glow", settings.glowradius);
        addParam("radius", settings.borderradius);
        addParam("pressscale", settings.pressscale);
        addParam("speed", settings.animationspeed);
        addParam("scale", settings.scale);
        addParam("opacity", settings.opacity);

        addParam("gapmodifier", settings.gapmodifier);

        if (settings.fontfamily && settings.fontfamily.trim() !== "") {
            addParam("fontfamily", settings.fontfamily.toLowerCase().replace(/ /g, "+"));
        }

        if (settings.boldfont === true || settings.boldfont === "true" || settings.boldfont === "1") {
            params.push("boldfont=1");
        }

        if (settings.hidemouse === true || settings.hidemouse === "true" || settings.hidemouse === "1") {
            params.push("hidemouse=1");
        }

        if (settings.hidescrollcombo === true || settings.hidescrollcombo === "true" || settings.hidescrollcombo === "1") {
            params.push("hidescrollcombo=1");
        }

        this.addCustomLayoutParams(params, settings);

        return params.join("&");
    }

    addCustomLayoutParams(params, settings) {
        const addParam = (key, value) => {
            if (value !== undefined && value !== null && value !== "") {
                params.push(`${key}=${value.replace(/, /g, ",")}`);
            }
        };

        const layoutRows = [
            "customLayoutRow1", "customLayoutRow2", "customLayoutRow3",
            "customLayoutRow4", "customLayoutRow5", "customLayoutMouse"
        ];

        layoutRows.forEach(row => addParam(row, settings[row]));
    }

    getOverlaySettings() {
        const params = this.urlParams;
        if (params.has("cfg")) {
            const compressed = params.get("cfg");
            const decompressed = this.decompressSettings(compressed);

            if (decompressed) {
                const decompressedParams = new URLSearchParams(decompressed);
                return {
                    activecolor: this.utils.normalizeColorValue(decompressedParams.get("activecolor")) || "#8b5cf6",
                    inactivecolor: this.utils.normalizeColorValue(decompressedParams.get("inactivecolor")) || "#808080",
                    backgroundcolor: this.utils.normalizeColorValue(decompressedParams.get("backgroundcolor")) || "#1a1a1ad1",
                    activebgcolor: this.utils.normalizeColorValue(decompressedParams.get("activebgcolor")) || "#202020",
                    outlinecolor: this.utils.normalizeColorValue(decompressedParams.get("outlinecolor")) || "#4f4f4f",
                    fontcolor: this.utils.normalizeColorValue(decompressedParams.get("fontcolor")) || "#ffffff",
                    glowradius: decompressedParams.get("glow") || "24",
                    borderradius: decompressedParams.get("radius") || "8",
                    pressscale: decompressedParams.get("pressscale") || "105",
                    animationspeed: decompressedParams.get("speed") || "100",
                    scale: decompressedParams.get("scale") || "100",
                    opacity: decompressedParams.get("opacity") || "100",
                    fontfamily: decompressedParams.get("fontfamily") || "",
                    boldfont: decompressedParams.get("boldfont") === "1",
                    hidemouse: decompressedParams.get("hidemouse") === "1",
                    hidescrollcombo: decompressedParams.get("hidescrollcombo") === "1",
                    customLayoutRow1: decompressedParams.get("customLayoutRow1") || DEFAULT_LAYOUT_STRINGS.row1,
                    customLayoutRow2: decompressedParams.get("customLayoutRow2") || DEFAULT_LAYOUT_STRINGS.row2,
                    customLayoutRow3: decompressedParams.get("customLayoutRow3") || DEFAULT_LAYOUT_STRINGS.row3,
                    customLayoutRow4: decompressedParams.get("customLayoutRow4") || DEFAULT_LAYOUT_STRINGS.row4,
                    customLayoutRow5: decompressedParams.get("customLayoutRow5") || DEFAULT_LAYOUT_STRINGS.row5,
                    customLayoutMouse: decompressedParams.get("customLayoutMouse") || DEFAULT_LAYOUT_STRINGS.mouse,

                    gapmodifier: decompressedParams.get("gapmodifier") || "100",
                };
            }
        }

        return {
            activecolor: this.utils.normalizeColorValue(params.get("activecolor")) || "#8b5cf6",
            inactivecolor: this.utils.normalizeColorValue(params.get("inactivecolor")) || "#808080",
            backgroundcolor: this.utils.normalizeColorValue(params.get("backgroundcolor")) || "#1a1a1ad1",
            activebgcolor: this.utils.normalizeColorValue(params.get("activebgcolor")) || "#202020",
            outlinecolor: this.utils.normalizeColorValue(params.get("outlinecolor")) || "#4f4f4f",
            fontcolor: this.utils.normalizeColorValue(params.get("fontcolor")) || "#ffffff",
            glowradius: params.get("glow") || "24",
            borderradius: params.get("radius") || "8",
            pressscale: params.get("pressscale") || "105",
            animationspeed: params.get("speed") || "100",
            scale: params.get("scale") || "100",
            opacity: params.get("opacity") || "100",
            fontfamily: params.get("fontfamily") || "",
            boldfont: params.get("boldfont") === "1",
            hidemouse: params.get("hidemouse") === "1",
            hidescrollcombo: params.get("hidescrollcombo") === "1",
            customLayoutRow1: params.get("customLayoutRow1") || DEFAULT_LAYOUT_STRINGS.row1,
            customLayoutRow2: params.get("customLayoutRow2") || DEFAULT_LAYOUT_STRINGS.row2,
            customLayoutRow3: params.get("customLayoutRow3") || DEFAULT_LAYOUT_STRINGS.row3,
            customLayoutRow4: params.get("customLayoutRow4") || DEFAULT_LAYOUT_STRINGS.row4,
            customLayoutRow5: params.get("customLayoutRow5") || DEFAULT_LAYOUT_STRINGS.row5,
            customLayoutMouse: params.get("customLayoutMouse") || DEFAULT_LAYOUT_STRINGS.mouse,

            gapmodifier: params.get("gapmodifier") || "100",
        };
    }
}