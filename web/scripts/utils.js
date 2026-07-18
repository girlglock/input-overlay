//guh
import { FONT_FAMILY_LINKS } from "./consts.js";

export class Utils {
    constructor() {
        this._hexCache = new Map();
    }

    _maskAddress(text) {
        return text.replace(
            /\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b/g,
            (_, a, b, c, d) => `***.***.***.${d}`
        ).replace(
            /\b(localhost|[\w-]+\.[\w.-]+)\b(?=:\d)/g,
            "***"
        );
    }

    hexToRgba(hex, alpha = 1) {
        if (!hex || !hex.startsWith("#")) return `rgba(0, 0, 0, ${alpha})`;
        hex = hex.toLowerCase();

        const cacheKey = hex + alpha;
        if (this._hexCache.has(cacheKey)) return this._hexCache.get(cacheKey);

        let result;
        if (hex.length === 9) {
            const r = parseInt(hex.substring(1, 3), 16);
            const g = parseInt(hex.substring(3, 5), 16);
            const b = parseInt(hex.substring(5, 7), 16);
            const a = parseInt(hex.substring(7, 9), 16) / 255;
            result = `rgba(${r}, ${g}, ${b}, ${a.toFixed(2)})`;
        } else {
            const r = parseInt(hex.substring(1, 3), 16);
            const g = parseInt(hex.substring(3, 5), 16);
            const b = parseInt(hex.substring(5, 7), 16);
            result = `rgba(${r}, ${g}, ${b}, ${alpha})`;
        }

        if (this._hexCache.size > 256) this._hexCache.clear();
        this._hexCache.set(cacheKey, result);
        return result;
    }

    lerpColor(hexA, hexB, t) {
        const parse = (hex) => {
            if (!hex) return [128, 128, 128];
            const h = hex.replace("#", "");
            return h.length === 3
                ? h.split("").map(c => parseInt(c + c, 16))
                : [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
        };
        const a = parse(hexA), b = parse(hexB);
        return `rgb(${Math.round(a[0] + (b[0] - a[0]) * t)},${Math.round(a[1] + (b[1] - a[1]) * t)},${Math.round(a[2] + (b[2] - a[2]) * t)})`;
    }

    applyFontStyles(fontKey) {
        const STYLE_ID = "dynamic-key-font-style";

        if (!fontKey) {
            document.head.querySelectorAll("link[href*='fonts.googleapis']").forEach(el => el.remove());
            document.getElementById(STYLE_ID)?.remove();
            return;
        }

        let fontUrl = FONT_FAMILY_LINKS[fontKey];

        if (!fontUrl && (fontKey.startsWith("https://fonts.googleapis.com/") ||
            fontKey.startsWith("data:font/") || fontKey.startsWith("data:application/font"))) {
            fontUrl = fontKey;
        }

        if (!fontUrl) {
            document.head.querySelectorAll("link[href*='fonts.googleapis']").forEach(el => el.remove());
            document.getElementById(STYLE_ID)?.remove();
            return;
        }

        const getOrCreateStyle = () => {
            let styleTag = document.getElementById(STYLE_ID);
            if (!styleTag) {
                styleTag = document.createElement("style");
                styleTag.id = STYLE_ID;
                document.head.appendChild(styleTag);
            }
            return styleTag;
        };

        if (fontUrl.startsWith("data:font/") || fontUrl.startsWith("data:application/font")) {
            const formatMatch = fontUrl.match(/data:[^/]+\/([^;]+);base64,/);
            const format = formatMatch ? formatMatch[1] : "truetype";

            document.head.querySelectorAll("link[href*='fonts.googleapis']").forEach(el => el.remove());
            getOrCreateStyle().textContent = `
            @font-face {
                font-family: "custom-b64-font";
                src: url("${fontUrl}") format("${format}");
                font-weight: normal;
                font-style: normal;
            }
            .scroll-display, .key { font-family: "custom-b64-font", sans-serif !important; }`;
            return;
        }

        const familyMatch = fontUrl.match(/family=([^&]+)/);
        const cssFontName = familyMatch
            ? familyMatch[1].split(":")[0].replace(/\+/g, " ")
            : "serif";

        document.head.querySelectorAll("link[rel='stylesheet'][href*='fonts.googleapis']").forEach(el => el.remove());

        const link = document.createElement("link");
        link.href = fontUrl;
        link.rel = "stylesheet";
        document.head.appendChild(link);

        getOrCreateStyle().textContent = `
            .scroll-display, .key { font-family: "${cssFontName}", sans-serif !important; }`;
    }

    measureTextWidth(element) {
        const tempSpan = document.createElement("span");
        const styles = window.getComputedStyle(element);
        tempSpan.style.cssText = "position:absolute;visibility:hidden;white-space:nowrap;";
        tempSpan.style.fontSize = styles.fontSize;
        tempSpan.style.fontWeight = styles.fontWeight;
        tempSpan.style.fontFamily = styles.fontFamily;
        tempSpan.textContent = element.textContent;
        document.body.appendChild(tempSpan);
        const width = tempSpan.offsetWidth;
        document.body.removeChild(tempSpan);
        return width;
    }

    scaleKeyFontSize(element, containerWidth, textWidth) {
        const currentFontSize = parseFloat(window.getComputedStyle(element).fontSize);
        element.style.fontSize = `${currentFontSize * (containerWidth / textWidth)}px`;
    }

    normalizeColorValue(value) {
        if (!value) return value;
        if (value.startsWith("#")) return value.toLowerCase();
        if (value.startsWith("%23")) return "#" + value.substring(3).toLowerCase();
        return "#" + value.toLowerCase();
    }
}