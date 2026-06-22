//guh
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

        if (settings.glowradius && settings.glowradius !== "24") addParam("glow", settings.glowradius);
        if (settings.borderradius && settings.borderradius !== "8") addParam("radius", settings.borderradius);
        if (settings.pressedradius && settings.pressedradius !== (settings.borderradius || "8")) addParam("pressedradius", settings.pressedradius);
        if (settings.pressscale && settings.pressscale !== "105") addParam("pressscale", settings.pressscale);
        if (settings.animationspeed && settings.animationspeed !== "100") addParam("speed", settings.animationspeed);
        if (settings.scale && settings.scale !== "100") addParam("scale", settings.scale);
        if (settings.opacity && settings.opacity !== "100") addParam("opacity", settings.opacity);

        if (settings.analogdisplaymode && settings.analogdisplaymode !== "fill") {
            addParam("analogdisplaymode", settings.analogdisplaymode);
        }
        if (settings.analogsmoothing === true || settings.analogsmoothing === "true" || settings.analogsmoothing === "1") {
            params.push("analogsmoothing=1");
        }
        if (settings.keylegendmode && settings.keylegendmode !== "fading") {
            addParam("keylegendmode", settings.keylegendmode);
        }
        if (settings.forcedisableanalog === true || settings.forcedisableanalog === "true" || settings.forcedisableanalog === "1") {
            params.push("forcedisableanalog=1");
        }
        if (settings.gamepaddeadzone && settings.gamepaddeadzone !== "3") {
            addParam("gamepaddeadzone", settings.gamepaddeadzone);
        }

        if (settings.outlinescalepressed && settings.outlinescalepressed !== "2") addParam("outlinescalepressed", settings.outlinescalepressed);
        if (settings.outlinescaleunpressed && settings.outlinescaleunpressed !== "2") addParam("outlinescaleunpressed", settings.outlinescaleunpressed);

        if (settings.fontfamily && settings.fontfamily.trim() !== "") {
            addParam("fontfamily", settings.fontfamily.replace(/ /g, "+"));
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
        if (settings.mousepadtextureopacity && parseFloat(settings.mousepadtextureopacity) !== 1) {
            addParam("mousepadtextureopacity", settings.mousepadtextureopacity);
        }
        if (settings.showmousedistance === true || settings.showmousedistance === "true" || settings.showmousedistance === "1") {
            params.push("showmousedistance=1");
        }
        if (settings.mousedistancedpi && settings.mousedistancedpi !== "400") {
            addParam("mousedistancedpi", settings.mousedistancedpi);
        }
        if (settings.resetmousedistanceafterfade === true || settings.resetmousedistanceafterfade === "true" || settings.resetmousedistanceafterfade === "1") {
            params.push("resetmousedistanceafterfade=1");
        }

        if (settings.keyLayout) {
            params.push(`keyLayout=${settings.keyLayout}`);
        }

        return params.join("&");
    }

    getKeyLayoutParam() {
        return this.urlParams.get("keyLayout") || null;
    }

    getOverlaySettings() {
        const params = this.urlParams;
        if (params.has("cfg")) {
            const compressed = params.get("cfg");
            const decompressed = this.decompressSettings(compressed);

            if (decompressed) {
                const decompressedParams = new URLSearchParams(decompressed);
                const rad = decompressedParams.get("radius");
                return {
                    activecolor: this.utils.normalizeColorValue(decompressedParams.get("activecolor")) || "#8b5cf6",
                    inactivecolor: this.utils.normalizeColorValue(decompressedParams.get("inactivecolor")) || "#808080",
                    backgroundcolor: this.utils.normalizeColorValue(decompressedParams.get("backgroundcolor")) || "#1a1a1ad1",
                    activebgcolor: this.utils.normalizeColorValue(decompressedParams.get("activebgcolor")) || "#202020",
                    outlinecolor: this.utils.normalizeColorValue(decompressedParams.get("outlinecolor")) || "#4f4f4f",
                    fontcolor: this.utils.normalizeColorValue(decompressedParams.get("fontcolor")) || "#ffffff",
                    glowradius: decompressedParams.get("glow") || "24",
                    borderradius: rad || "8",
                    pressedradius: decompressedParams.get("pressedradius") || rad || "8",
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
                    mousepadtextureopacity: decompressedParams.get("mousepadtextureopacity") || "1",
                    showmousedistance: decompressedParams.get("showmousedistance") === "1",
                    mousedistancedpi: decompressedParams.get("mousedistancedpi") || "400",
                    resetmousedistanceafterfade: decompressedParams.get("resetmousedistanceafterfade") === "1",
                    gapmodifier: decompressedParams.get("gapmodifier") || "100",
                    outlinescalepressed: decompressedParams.get("outlinescalepressed") || "2",
                    outlinescaleunpressed: decompressedParams.get("outlinescaleunpressed") || "2",
                    analogdisplaymode: decompressedParams.get("analogdisplaymode") || "fill",
                    analogsmoothing: decompressedParams.get("analogsmoothing") === "1",
                    keylegendmode: decompressedParams.get("keylegendmode") || "fading",
                    forcedisableanalog: decompressedParams.get("forcedisableanalog") === "1",
                    gamepaddeadzone: decompressedParams.get("gamepaddeadzone") || "3",
                    wsauth: decompressedParams.get("wsauth") || "",
                    keyLayout: decompressedParams.get("keyLayout") || this.urlParams.get("keyLayout") || null,
                    customLayoutRow1: decompressedParams.get("customLayoutRow1") || null,
                    customLayoutRow2: decompressedParams.get("customLayoutRow2") || null,
                    customLayoutRow3: decompressedParams.get("customLayoutRow3") || null,
                    customLayoutRow4: decompressedParams.get("customLayoutRow4") || null,
                    customLayoutRow5: decompressedParams.get("customLayoutRow5") || null,
                    customLayoutMouse: decompressedParams.get("customLayoutMouse") || null,
                };
            }
        }

        const rad = params.get("radius");
        return {
            activecolor: this.utils.normalizeColorValue(params.get("activecolor")) || "#8b5cf6",
            inactivecolor: this.utils.normalizeColorValue(params.get("inactivecolor")) || "#808080",
            backgroundcolor: this.utils.normalizeColorValue(params.get("backgroundcolor")) || "#1a1a1ad1",
            activebgcolor: this.utils.normalizeColorValue(params.get("activebgcolor")) || "#202020",
            outlinecolor: this.utils.normalizeColorValue(params.get("outlinecolor")) || "#4f4f4f",
            fontcolor: this.utils.normalizeColorValue(params.get("fontcolor")) || "#ffffff",
            glowradius: params.get("glow") || "24",
            borderradius: rad || "8",
            pressedradius: params.get("pressedradius") || rad || "8",
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
            mousepadtextureopacity: params.get("mousepadtextureopacity") || "1",
            showmousedistance: params.get("showmousedistance") === "1",
            mousedistancedpi: params.get("mousedistancedpi") || "400",
            resetmousedistanceafterfade: params.get("resetmousedistanceafterfade") === "1",
            gapmodifier: params.get("gapmodifier") || "100",
            outlinescalepressed: params.get("outlinescalepressed") || "2",
            outlinescaleunpressed: params.get("outlinescaleunpressed") || "2",
            analogdisplaymode: params.get("analogdisplaymode") || "fill",
            analogsmoothing: params.get("analogsmoothing") === "1",
            keylegendmode: params.get("keylegendmode") || "fading",
            forcedisableanalog: params.get("forcedisableanalog") === "1",
            gamepaddeadzone: params.get("gamepaddeadzone") || "3",
            wsauth: params.get("wsauth") || "",
            keyLayout: this.urlParams.get("keyLayout") || null,
            customLayoutRow1: params.get("customLayoutRow1") || null,
            customLayoutRow2: params.get("customLayoutRow2") || null,
            customLayoutRow3: params.get("customLayoutRow3") || null,
            customLayoutRow4: params.get("customLayoutRow4") || null,
            customLayoutRow5: params.get("customLayoutRow5") || null,
            customLayoutMouse: params.get("customLayoutMouse") || null,
        };
    }
}