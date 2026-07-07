//guh
const REGEX_MOUSE_PAD = /^mouse_pad:(u[\d-]+):(u[\d-]+)(?::(a-[tbc][lrc]))?$/;
const REGEX_GP_JOYSTICK = /^gp_joystick:(gp_ls|gp_rs):(u[\d-]+)(?::(u[\d-]+))?(?::(a-[tbc][lrc]))?$/;
const REGEX_SCROLLER = /^([\w|]+):"([^"]+)":"([^"]+)":"([^"]+)"(?::([-\w.]+))?$/;
const REGEX_SCROLL_UPDOWN = /^scroll_updown:"([^"]+)":"([^"]+)"(?::([-\w.]+))?$/;
const REGEX_MOUSE_SIDE = /^(mouse_side):"([^"]+)":"([^"]+)"(?::([-\w.]+))?$/;
const REGEX_STANDARD = /^([\w|]+):"([^"]+)"(?::([-\w.]+))?$/;

export class LayoutParser {
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

        if ((m = REGEX_SCROLL_UPDOWN.exec(elementString)))
            return { key: "scroll_updown", labels: [m[1], m[2]], class: m[3] || "", type: "scroll_updown" };

        if ((m = REGEX_MOUSE_SIDE.exec(elementString)))
            return { key: m[1], labels: [m[2], m[3]], class: m[4] || "", type: "mouse_side" };

        if ((m = REGEX_STANDARD.exec(elementString))) {
            const keys = m[1].split("|");
            const label = m[2];
            const customClass = m[3];
            if (keys[0] === "scroll_up")
                return { key: "scroll_up", label, class: customClass || "", type: "scroll_up" };
            if (keys[0] === "scroll_down")
                return { key: "scroll_down", label, class: customClass || "", type: "scroll_down" };
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

}