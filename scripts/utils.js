//guh
import {FONT_FAMILY_LINKS} from "./consts.js";

export class Utils {
    hexToRgba(hex, alpha = 1) {
        if (!hex || !hex.startsWith("#")) return `rgba(0, 0, 0, ${alpha})`;
        hex = hex.toLowerCase();

        if (hex.length === 9) {
            const r = parseInt(hex.substring(1, 3), 16);
            const g = parseInt(hex.substring(3, 5), 16);
            const b = parseInt(hex.substring(5, 7), 16);
            const a = parseInt(hex.substring(7, 9), 16) / 255;
            return `rgba(${r}, ${g}, ${b}, ${a.toFixed(2)})`;
        }

        const r = parseInt(hex.substring(1, 3), 16);
        const g = parseInt(hex.substring(3, 5), 16);
        const b = parseInt(hex.substring(5, 7), 16);
        return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    }

    applyFontStyles(fontKey) {
        const STYLE_ID = "dynamic-key-font-style";

        if (!fontKey) {
            document.head.querySelectorAll("link[href*='fonts.googleapis']").forEach(el => el.remove());

            const oldStyle = document.getElementById(STYLE_ID);
            if (oldStyle) oldStyle.remove();

            return;
        }

        let fontUrl = FONT_FAMILY_LINKS[fontKey];

        if (!fontUrl && fontKey.startsWith("https://fonts.googleapis.com/")) {
            fontUrl = fontKey;
        } else if (!fontUrl) {
            document.head.querySelectorAll("link[href*='fonts.googleapis']").forEach(el => el.remove());

            const oldStyle = document.getElementById(STYLE_ID);
            if (oldStyle) oldStyle.remove();

            return;
        }

        const link = document.createElement("link");
        link.href = fontUrl;
        link.rel = "stylesheet";

        let cssFontName = "serif";
        const familyMatch = fontUrl.match(/family=([^&]+)/);

        if (familyMatch && familyMatch[1]) {
            let rawFontName = familyMatch[1].split(":")[0];
            cssFontName = rawFontName.replace(/\+/g, " ");
        }

        document.head.querySelectorAll("link[href*='fonts.googleapis']").forEach(el => {
            if (el.rel === "stylesheet") el.remove();
        });

        document.head.appendChild(link);

        let styleTag = document.getElementById(STYLE_ID);

        if (!styleTag) {
            styleTag = document.createElement("style");
            styleTag.id = STYLE_ID;
            document.head.appendChild(styleTag);
        }

        styleTag.textContent = `
            .scroll-display,
            .key {
                font-family: "${cssFontName}", sans-serif !important;
            }
        `;
    }

    measureTextWidth(element) {
        const tempSpan = document.createElement("span");
        const styles = window.getComputedStyle(element);

        tempSpan.style.cssText = "position: absolute; visibility: hidden; white-space: nowrap;";
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
        const scaleFactor = containerWidth / textWidth;
        const newFontSize = currentFontSize * scaleFactor;
        element.style.fontSize = `${newFontSize}px`;
    }

    normalizeColorValue(value) {
        if (!value) return value;

        if (value.startsWith("#")) {
            return value.toLowerCase();
        }

        if (value.startsWith("%23")) {
            return "#" + value.substring(3).toLowerCase();
        }

        return "#" + value.toLowerCase();
    }
}
