//guh
import {DEFAULT_LAYOUT_STRINGS} from "../consts.js";

export class LayoutParser {
    constructor() {
        this.DEFAULT_LAYOUT_STRINGS = DEFAULT_LAYOUT_STRINGS;
    }

    parseElementDef(elementString) {
        if (!elementString) return null;

        if (elementString === "invisible" || elementString === "dummy")
            return {class: "dummy"};

        const scrollerMatch = elementString.match(/^(scroller):"([^"]+)":"([^"]+)":"([^"]+)"(?::([-\w.]+))?$/);
        if (scrollerMatch) {
            return {
                key: scrollerMatch[1],
                labels: [scrollerMatch[2], scrollerMatch[3], scrollerMatch[4]],
                class: scrollerMatch[5] || "",
                type: "scroller"
            };
        }

        const sideMatch = elementString.match(/^(mouse_side):"([^"]+)":"([^"]+)"(?::([-\w.]+))?$/);
        if (sideMatch) {
            return {
                key: sideMatch[1],
                labels: [sideMatch[2], sideMatch[3]],
                class: sideMatch[4] || "",
                type: "mouse_side"
            };
        }

        const standardMatch = elementString.match(/^(\w+):"([^"]+)"(?::([-\w.]+))?$/);
        if (standardMatch) {
            const key = standardMatch[1];
            const label = standardMatch[2];
            const customClass = standardMatch[3];

            let type = "key";
            if (key.startsWith("mouse_") || key === "scroller") {
                type = "mouse";
            }

            const elementDef = {
                key: key,
                label: label,
                type: type
            };

            if ((label === "invis") && customClass) {
                elementDef.class = `${customClass} invisible`;
            } else if (label === "invis") {
                elementDef.class = "invisible";
            } else if (customClass) {
                elementDef.class = customClass;
            }


            return elementDef;
        }

        return null;
    }

    parseCustomLayoutInput(inputString) {
        if (!inputString) return [];

        return inputString.split(/\s*,\s*/)
            .map(this.parseElementDef.bind(this))
            .filter(def => def !== null);
    }

    getKeyboardLayoutDef(settings) {
        const customLayout = [];

        const row1 = this.parseCustomLayoutInput(settings.customLayoutRow1);
        const row2 = this.parseCustomLayoutInput(settings.customLayoutRow2);
        const row3 = this.parseCustomLayoutInput(settings.customLayoutRow3);
        const row4 = this.parseCustomLayoutInput(settings.customLayoutRow4);
        const row5 = this.parseCustomLayoutInput(settings.customLayoutRow5);

        if (row1.length > 0) customLayout.push(row1);
        if (row2.length > 0) customLayout.push(row2);
        if (row3.length > 0) customLayout.push(row3);
        if (row4.length > 0) customLayout.push(row4);
        if (row5.length > 0) customLayout.push(row5);

        if (customLayout.length > 0) {
            return customLayout;
        }

        return [
            this.parseCustomLayoutInput(this.DEFAULT_LAYOUT_STRINGS.row1),
            this.parseCustomLayoutInput(this.DEFAULT_LAYOUT_STRINGS.row2),
            this.parseCustomLayoutInput(this.DEFAULT_LAYOUT_STRINGS.row3),
            this.parseCustomLayoutInput(this.DEFAULT_LAYOUT_STRINGS.row4),
            this.parseCustomLayoutInput(this.DEFAULT_LAYOUT_STRINGS.row5)
        ].filter(row => row.length > 0);
    }

    getMouseLayoutDef(settings) {
        const customLayout = this.parseCustomLayoutInput(settings.customLayoutMouse);
        if (customLayout.length > 0) {
            return customLayout;
        }
        return this.parseCustomLayoutInput(this.DEFAULT_LAYOUT_STRINGS.mouse);
    }
}