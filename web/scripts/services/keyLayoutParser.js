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
        const def = { type, w: +w, h: +h, x: +x, y: +y };
        if (arity === 1) def.label = labels[0] ?? "";
        else if (arity > 1) def.labels = labels;
        if (type.includes("|")) def.keys = type.split("|");
        if (moveToTop) def.moveToTop = true;
        return def;
    }

    parseAll(tupleArray) {
        if (!Array.isArray(tupleArray)) return [];
        return tupleArray.map(t => this.parseTuple(t)).filter(Boolean);
    }

    serializeTuple(def) {
        const r = (v) => parseFloat(v.toFixed(4));
        const w = r(def.w ?? 1), h = r(def.h ?? 1), x = r(def.x ?? 0), y = r(def.y ?? 0);
        let tail;
        if (x !== 0 || y !== 0) tail = [w, h, x, y];
        else if (h !== 1) tail = [w, h];
        else if (w !== 1) tail = [w];
        else tail = [];

        let result;
        if (def.type === "input_history") {
            const keysStr = (def.trackedKeys ?? []).join(",");
            const cfg = `${keysStr};${def.vertical ? 1 : 0};${def.scrollSpeed ?? 200};${def.highlightOverlap ? 1 : 0};${def.reverseDirection ? 1 : 0}`;
            result = [def.type, cfg, ...tail];
        } else {
            const arity = this.getLabelArity(def.type);
            const labels = arity === 1 ? [def.label ?? ""]
                : arity > 1 ? (def.labels ?? []).slice(0, arity)
                    : [];
            result = [def.type, ...labels, ...tail];
        }

        if (def.moveToTop) result.push("top");
        return result;
    }

    serializeAll(defs) {
        return defs.map(d => this.serializeTuple(d));
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
