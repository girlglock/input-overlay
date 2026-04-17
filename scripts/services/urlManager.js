//guh
import { DEFAULT_LAYOUT_STRINGS } from "../consts.js";

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

        if (settings.keylegendmode && settings.keylegendmode !== "fading") {
            addParam("keylegendmode", settings.keylegendmode);
        }
        if (settings.forcedisableanalog === true || settings.forcedisableanalog === "true" || settings.forcedisableanalog === "1") {
            params.push("forcedisableanalog=1");
        }

        addParam("outlinescalepressed", settings.outlinescalepressed);
        addParam("outlinescaleunpressed", settings.outlinescaleunpressed);

        if (settings.fontfamily && settings.fontfamily.trim() !== "") {
            addParam("fontfamily", settings.fontfamily.replace(/ /g, "+"));
        }

        if (settings.boldfont === true || settings.boldfont === "true" || settings.boldfont === "1") {
            params.push("boldfont=1");
        }

        if (settings.hidemouse === true || settings.hidemouse === "true" || settings.hidemouse === "1") {
            params.push("hidemouse=1");
        } if (settings.hidescrollcombo === true || settings.hidescrollcombo === "true" || settings.hidescrollcombo === "1") {
            params.push("hidescrollcombo=1");
        }

        if (settings.mousetrailsensitivity && settings.mousetrailsensitivity !== "100") {
            addParam("mousetrailsensitivity", settings.mousetrailsensitivity);
        }
        if (settings.mousetrailfadeout != null && settings.mousetrailfadeout !== "600") {
            addParam("mousetrailfadeout", settings.mousetrailfadeout);
        }
        if (settings.mousetrailmode && settings.mousetrailmode !== "wrap") {
            addParam("mousetrailmode", settings.mousetrailmode);
        }
        if (settings.mousetraillength && settings.mousetraillength !== "150") {
            addParam("mousetraillength", settings.mousetraillength);
        }
        if (settings.mousetrailm1highlight === true || settings.mousetrailm1highlight === "true" || settings.mousetrailm1highlight === "1") {
            params.push("mousetrailm1highlight=1");
        }
        if (settings.mousepadtexture && settings.mousepadtexture.trim() !== "") {
            addParam("mousepadtexture", encodeURIComponent(settings.mousepadtexture.trim()));
        }
        if (settings.mousepadtexturezoom && parseFloat(settings.mousepadtexturezoom) !== 1) {
            addParam("mousepadtexturezoom", settings.mousepadtexturezoom);
        }
        if (settings.showmousedistance === true || settings.showmousedistance === "true" || settings.showmousedistance === "1") {
            params.push("showmousedistance=1");
        }
        if (settings.mousedistancedpi && settings.mousedistancedpi !== "400") {
            addParam("mousedistancedpi", settings.mousedistancedpi);
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
                    mousetrailsensitivity: decompressedParams.get("mousetrailsensitivity") || "100",
                    mousetrailfadeout: decompressedParams.get("mousetrailfadeout") ?? "600",
                    mousetrailmode: decompressedParams.get("mousetrailmode") || "wrap",
                    mousetraillength: decompressedParams.get("mousetraillength") || "150",
                    mousetrailm1highlight: decompressedParams.get("mousetrailm1highlight") === "1",
                    mousepadtexture: decompressedParams.get("mousepadtexture") ? decodeURIComponent(decompressedParams.get("mousepadtexture")) : "",
                    mousepadtexturezoom: decompressedParams.get("mousepadtexturezoom") || "1",
                    showmousedistance: decompressedParams.get("showmousedistance") === "1",
                    mousedistancedpi: decompressedParams.get("mousedistancedpi") || "400",
                    customLayoutRow1: decompressedParams.has("customLayoutRow1") ? decompressedParams.get("customLayoutRow1") : DEFAULT_LAYOUT_STRINGS.row1,
                    customLayoutRow2: decompressedParams.has("customLayoutRow2") ? decompressedParams.get("customLayoutRow2") : DEFAULT_LAYOUT_STRINGS.row2,
                    customLayoutRow3: decompressedParams.has("customLayoutRow3") ? decompressedParams.get("customLayoutRow3") : DEFAULT_LAYOUT_STRINGS.row3,
                    customLayoutRow4: decompressedParams.has("customLayoutRow4") ? decompressedParams.get("customLayoutRow4") : DEFAULT_LAYOUT_STRINGS.row4,
                    customLayoutRow5: decompressedParams.has("customLayoutRow5") ? decompressedParams.get("customLayoutRow5") : DEFAULT_LAYOUT_STRINGS.row5,
                    customLayoutMouse: decompressedParams.has("customLayoutMouse") ? decompressedParams.get("customLayoutMouse") : DEFAULT_LAYOUT_STRINGS.mouse,

                    gapmodifier: decompressedParams.get("gapmodifier") || "100",
                    outlinescalepressed: decompressedParams.get("outlinescalepressed") || "2",
                    outlinescaleunpressed: decompressedParams.get("outlinescaleunpressed") || "2",

                    keylegendmode: decompressedParams.get("keylegendmode") || "fading",
                    forcedisableanalog: decompressedParams.get("forcedisableanalog") === "1",

                    wsauth: decompressedParams.get("wsauth") || "",
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
            mousetrailsensitivity: params.get("mousetrailsensitivity") || "100",
            mousetrailfadeout: params.get("mousetrailfadeout") ?? "600",
            mousetrailmode: params.get("mousetrailmode") || "wrap",
            mousetraillength: params.get("mousetraillength") || "150",
            mousetrailm1highlight: params.get("mousetrailm1highlight") === "1",
            mousepadtexture: params.get("mousepadtexture") ? decodeURIComponent(params.get("mousepadtexture")) : "",
            mousepadtexturezoom: params.get("mousepadtexturezoom") || "1",
            showmousedistance: params.get("showmousedistance") === "1",
            mousedistancedpi: params.get("mousedistancedpi") || "400",
            customLayoutRow1: params.has("customLayoutRow1") ? params.get("customLayoutRow1") : DEFAULT_LAYOUT_STRINGS.row1,
            customLayoutRow2: params.has("customLayoutRow2") ? params.get("customLayoutRow2") : DEFAULT_LAYOUT_STRINGS.row2,
            customLayoutRow3: params.has("customLayoutRow3") ? params.get("customLayoutRow3") : DEFAULT_LAYOUT_STRINGS.row3,
            customLayoutRow4: params.has("customLayoutRow4") ? params.get("customLayoutRow4") : DEFAULT_LAYOUT_STRINGS.row4,
            customLayoutRow5: params.has("customLayoutRow5") ? params.get("customLayoutRow5") : DEFAULT_LAYOUT_STRINGS.row5,
            customLayoutMouse: params.has("customLayoutMouse") ? params.get("customLayoutMouse") : DEFAULT_LAYOUT_STRINGS.mouse,

            gapmodifier: params.get("gapmodifier") || "100",

            outlinescalepressed: params.get("outlinescalepressed") || "2",
            outlinescaleunpressed: params.get("outlinescaleunpressed") || "2",

            keylegendmode: params.get("keylegendmode") || "fading",
            forcedisableanalog: params.get("forcedisableanalog") === "1",

            wsauth: params.get("wsauth") || "",
        };
    }
}