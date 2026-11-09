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
    "$": 0,
};

export class KeyLayoutParser {
    getLabelArity(type) {
        const base = type.includes("|") ? type.split("|")[0] : type;
        return LABEL_ARITY[base] ?? 1;
    }

    parseTuple(tuple) {
        if (!Array.isArray(tuple) || tuple.length === 0) return null;
        const type = String(tuple[0]);
        const arity = this.getLabelArity(type);
        const labels = tuple.slice(1, 1 + arity).map(String);
        const dims = tuple.slice(1 + arity);
        const [w = 1, h = 1, x = 0, y = 0] = dims;
        const def = { type, w: +w, h: +h, x: +x, y: +y };
        if (arity === 1) def.label = labels[0] ?? "";
        else if (arity > 1) def.labels = labels;
        if (type.includes("|")) def.keys = type.split("|");
        return def;
    }

    parseAll(tupleArray) {
        if (!Array.isArray(tupleArray)) return [];
        return tupleArray.map(t => this.parseTuple(t)).filter(Boolean);
    }

    serializeTuple(def) {
        const arity = this.getLabelArity(def.type);
        const labels = arity === 1 ? [def.label ?? ""]
            : arity > 1 ? (def.labels ?? []).slice(0, arity)
                : [];
        const r = (v) => parseFloat(v.toFixed(4));
        const w = r(def.w ?? 1), h = r(def.h ?? 1), x = r(def.x ?? 0), y = r(def.y ?? 0);
        let tail;
        if (x !== 0 || y !== 0) tail = [w, h, x, y];
        else if (h !== 1) tail = [w, h];
        else if (w !== 1) tail = [w];
        else tail = [];
        return [def.type, ...labels, ...tail];
    }

    serializeAll(defs) {
        return defs.map(d => this.serializeTuple(d));
    }

    compressTuples(tupleArray) {
        try {
            const json = JSON.stringify(tupleArray);
            const compressed = pako.deflate(json, { level: 9 });
            const base64 = btoa(String.fromCharCode.apply(null, compressed));
            return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
        } catch (e) {
            console.error("keyLayout compress error:", e);
            return null;
        }
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
