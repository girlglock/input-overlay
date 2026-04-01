//guh
import { DEFAULT_LAYOUT_STRINGS } from "../consts.js";

const REGEX_MOUSE_PAD  = /^mouse_pad:(u[\d-]+):(u[\d-]+)(?::(a-[tbc][lrc]))?$/;
const REGEX_GP_JOYSTICK = /^gp_joystick:(gp_ls|gp_rs):(u[\d-]+)(?::(u[\d-]+))?(?::(a-[tbc][lrc]))?$/;
const REGEX_SCROLLER   = /^([\w|]+):"([^"]+)":"([^"]+)":"([^"]+)"(?::([-\w.]+))?$/;
const REGEX_MOUSE_SIDE = /^(mouse_side):"([^"]+)":"([^"]+)"(?::([-\w.]+))?$/;
const REGEX_STANDARD   = /^([\w|]+):"([^"]+)"(?::([-\w.]+))?$/;

export class LayoutParser {
    constructor() {
        this.DEFAULT_LAYOUT_STRINGS = DEFAULT_LAYOUT_STRINGS;
    }

    parseElementDef(elementString) {
        if (!elementString) return null;
        elementString = elementString.trim();

        if (elementString === "dummy") return { type: "dummy" };
        if (elementString === "br") return { type: "br" };
        if (elementString === "invisible") return { class: "invisible" };

        let m;

        if ((m = REGEX_MOUSE_PAD.exec(elementString)))
            return { key: "mouse_pad", type: "mouse_pad", widthClass: m[1], heightClass: m[2], anchor: m[3] || "a-tl" };

        if ((m = REGEX_GP_JOYSTICK.exec(elementString)))
            return {
                key: m[1],          // gp_ls or gp_rs
                type: "gp_joystick",
                stickId: m[1],
                widthClass: m[2],
                heightClass: m[3] || m[2],
                anchor: m[4] || "a-tl",
            };

        if ((m = REGEX_SCROLLER.exec(elementString)) && m[1].includes("scroller")) {
            const keys = m[1].split("|");
            return { key: keys[0], keys, labels: [m[2], m[3], m[4]], class: m[5] || "", type: "scroller" };
        }

        if ((m = REGEX_MOUSE_SIDE.exec(elementString)))
            return { key: m[1], labels: [m[2], m[3]], class: m[4] || "", type: "mouse_side" };

        if ((m = REGEX_STANDARD.exec(elementString))) {
            const keys = m[1].split("|");
            const label = m[2];
            const customClass = m[3];
            const type = (keys[0].startsWith("mouse_") || keys[0] === "scroller") ? "mouse" : "key";

            let cls;
            if (label === "invis") cls = customClass ? `${customClass} invisible` : "invisible";
            else if (customClass) cls = customClass;

            return { key: keys[0], keys, label, type, ...(cls ? { class: cls } : {}) };
        }

        return null;
    }

    parseCustomLayoutInput(inputString) {
        if (!inputString) return [];
        return inputString.split(/\s*,\s*/).map(s => this.parseElementDef(s)).filter(Boolean);
    }

    splitByBr(items) {
        const rows = [];
        let current = [];
        for (const item of items) {
            if (item.type === "br") {
                if (current.length) rows.push(current);
                current = [];
            } else if (item.type !== "dummy") {
                current.push(item);
            }
        }
        if (current.length) rows.push(current);
        return rows.length ? rows : [[]];
    }

    getKeyboardLayoutDef(settings) {
        const customLayout = [];
        let userProvided = false;
        const rowKeys = ["customLayoutRow1", "customLayoutRow2", "customLayoutRow3", "customLayoutRow4", "customLayoutRow5"];

        for (const key of rowKeys) {
            if (!settings[key]) continue;
            userProvided = true;
            const rows = this.splitByBr(this.parseCustomLayoutInput(settings[key]));
            for (const r of rows) if (r.length) customLayout.push(r);
        }

        if (userProvided) return customLayout;

        return ["row1", "row2", "row3", "row4", "row5"]
            .map(k => this.parseCustomLayoutInput(this.DEFAULT_LAYOUT_STRINGS[k]))
            .filter(r => r.length);
    }

    getMouseLayoutDef(settings) {
        const parsed = this.parseCustomLayoutInput(settings.customLayoutMouse);
        if (parsed.length) return this.splitByBr(parsed);
        return this.splitByBr(this.parseCustomLayoutInput(this.DEFAULT_LAYOUT_STRINGS.mouse));
    }

    needsWebSocket(settings) {
        const rowKeys = ["customLayoutRow1", "customLayoutRow2", "customLayoutRow3", "customLayoutRow4", "customLayoutRow5"];
        const mouseDef = this.parseCustomLayoutInput(settings.customLayoutMouse || "");

        const hasMouseEl = mouseDef.some(item =>
            item.type === "mouse" || item.type === "scroller" || item.type === "mouse_side" || item.type === "mouse_pad"
        );
        if (hasMouseEl) return true;

        for (const rowKey of rowKeys) {
            const row = this.parseCustomLayoutInput(settings[rowKey] || "");
            const hasKey = row.some(item =>
                item.type === "key" && !(item.keys || [item.key]).every(k => k.startsWith("gp_") || k.startsWith("none"))
            );
            if (hasKey){
                console.log("has key using ws");
                return true;
            }
        }
        console.log("no key not using ws");
        return false;
    }
}