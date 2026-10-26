//guh
const LABEL_ARITY = {
    scroller: 3,
    scroll_updown: 2,
    scroll_up: 1,
    scroll_down: 1,
    mouse_side: 2,
    mouse_pad: 0,
    gp_joystick_ls: 0,
    gp_joystick_rs: 0,
    input_history: 0,
    "$": 0,
};

export const STYLE_OVERRIDE_KEYS = [
    "key_inactive_bg_color", "key_active_bg_color", "key_outline_color", "key_glow_color",
    "key_active_font_color", "key_inactive_font_color", "key_border_radius_idle", "key_border_radius_pressed",
];

export const SCROLL_TYPES = new Set(["scroller", "scroll_updown", "scroll_up", "scroll_down"]);

export function normalizePipeType(type) {
    if (!type.includes("|")) return type;
    
    const parts = type.split("|");
    const scrollIdx = parts.findIndex(p => SCROLL_TYPES.has(p));
    
    if (scrollIdx <= 0) return type;
    
    return [parts[scrollIdx], ...parts.filter((_, i) => i !== scrollIdx)].join("|");
}

export class KeyLayoutParser {
    getLabelArity(type) {
        const base = type.includes("|") ? type.split("|")[0] : type;
        return LABEL_ARITY[base] ?? 1;
    }

    parseTuple(tuple) {
        if (!Array.isArray(tuple) || tuple.length === 0) return null;

        let arr = tuple;
        let moveToTop = false;
        if (arr.length > 1 && arr[arr.length - 1] === "top") {
            moveToTop = true;
            arr = arr.slice(0, -1);
        }

        const type = String(arr[0]);

        if (type === "input_history") {
            const [cfg, w = 1, h = 1, x = 0, y = 0] = arr.slice(1);
            const [keysStr = "", vFlag = "0", speedStr = "200", hlFlag = "0", revFlag = "0"] = String(cfg ?? "").split(";");
            const historyDef = {
                type, w: +w, h: +h, x: +x, y: +y,
                trackedKeys: keysStr ? keysStr.split(",").filter(Boolean) : [],
                vertical: vFlag === "1",
                scrollSpeed: parseFloat(speedStr) || 200,
                highlightOverlap: hlFlag === "1",
                reverseDirection: revFlag === "1",
            };
            if (moveToTop) historyDef.moveToTop = true;
            return historyDef;
        }

        const arity = this.getLabelArity(type);
        const labels = arr.slice(1, 1 + arity).map(String);
        const dims = arr.slice(1 + arity);
        const [w = 1, h = 1, x = 0, y = 0] = dims;
        const normType = normalizePipeType(type);
        const normArity = this.getLabelArity(normType);
        const def = { type: normType, w: +w, h: +h, x: +x, y: +y };
        if (normArity === 1) def.label = arity === 1 ? (labels[0] ?? "") : "";
        else if (normArity > 1) def.labels = arity === normArity ? labels : [];
        if (normType.includes("|")) def.keys = normType.split("|");
        if (moveToTop) def.moveToTop = true;
        return def;
    }

    parseElementObject(obj) {
        if (!obj || typeof obj !== "object" || Array.isArray(obj)) return null;
        const type = obj.type != null ? normalizePipeType(String(obj.type)) : "";
        if (!type) return null;

        const def = {
            type,
            w: +(obj.w ?? 1),
            h: +(obj.h ?? 1),
            x: +(obj.x ?? 0),
            y: +(obj.y ?? 0),
        };

        if (type === "input_history") {
            def.trackedKeys = Array.isArray(obj.trackedKeys) ? obj.trackedKeys.map(String) : [];
            def.vertical = !!obj.vertical;
            def.scrollSpeed = parseFloat(obj.scrollSpeed) || 200;
            def.highlightOverlap = !!obj.highlightOverlap;
            def.reverseDirection = !!obj.reverseDirection;
            def.perfectGapMs = parseFloat(obj.perfectGapMs) || 0;
        } else {
            const arity = this.getLabelArity(type);
            if (arity === 1) def.label = obj.label != null ? String(obj.label) : "";
            else if (arity > 1) def.labels = Array.isArray(obj.labels) ? obj.labels.map(String) : [];
        }

        if (type.includes("|")) def.keys = type.split("|");
        if (obj.moveToTop) def.moveToTop = true;

        if (obj.style && typeof obj.style === "object" && !Array.isArray(obj.style)) {
            const style = {};
            for (const key of STYLE_OVERRIDE_KEYS) {
                if (obj.style[key] != null && obj.style[key] !== "") style[key] = obj.style[key];
            }
            if (Object.keys(style).length) def.style = style;
        }

        return def;
    }

    parseAll(elements) {
        if (!Array.isArray(elements)) return [];
        return elements
            .map(item => Array.isArray(item) ? this.parseTuple(item) : this.parseElementObject(item))
            .filter(Boolean);
    }

    serializeElement(def) {
        const r = (v) => parseFloat(v.toFixed(4));
        const result = {
            type: def.type,
            w: r(def.w ?? 1), h: r(def.h ?? 1), x: r(def.x ?? 0), y: r(def.y ?? 0),
        };

        if (def.type === "input_history") {
            result.trackedKeys = def.trackedKeys ?? [];
            result.vertical = !!def.vertical;
            result.scrollSpeed = def.scrollSpeed ?? 200;
            result.highlightOverlap = !!def.highlightOverlap;
            result.reverseDirection = !!def.reverseDirection;
            if (def.perfectGapMs) result.perfectGapMs = def.perfectGapMs;
        } else {
            const arity = this.getLabelArity(def.type);
            if (arity === 1) result.label = def.label ?? "";
            else if (arity > 1) result.labels = (def.labels ?? []).slice(0, arity);
        }

        if (def.moveToTop) result.moveToTop = true;

        if (def.style && typeof def.style === "object") {
            const style = {};
            
            for (const key of STYLE_OVERRIDE_KEYS) {
                if (def.style[key] != null && def.style[key] !== "") style[key] = def.style[key];
            }
            
            if (Object.keys(style).length) result.style = style;
        }

        return result;
    }

    serializeAll(defs) {
        return defs.map(d => this.serializeElement(d));
    }

    decompressTuples(str) {
        if (!str || str.startsWith("[") || str.startsWith("{")) return null;
        try {
            const base64 = str.replace(/-/g, "+").replace(/_/g, "/");
            const padding = "=".repeat((4 - base64.length % 4) % 4);
            const binary = atob(base64 + padding);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
            return JSON.parse(pako.inflate(bytes, { to: "string" }));
        } catch (e) {
            console.error("keyLayout decompress error:", e);
            return null;
        }
    }

    needsWebSocket(defs) {
        return defs.some(d => !d.type.startsWith("gp_") && d.type !== "$");
    }
}
