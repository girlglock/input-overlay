//guh
export class OverlayVisualiser {
    constructor(utils, layoutParser) {
        this.utils = utils;
        this.layoutParser = layoutParser;
        this.previewElements = null;
        this.activeKeys = new Set();
        this.activeMouseButtons = new Set();
        this.currentScrollCount = 0;
        this.lastScrollDirection = null;
        this.scrollTimeout = null;
        this.Z_INDEX_COUNTER = 100;
    }

    updateElementState(el, keyName, isActive, activeSet) {
        if (isActive) {
            if (!activeSet.has(keyName)) {
                el.classList.add("active");
                activeSet.add(keyName);
                this.Z_INDEX_COUNTER++;
                el.style.zIndex = this.Z_INDEX_COUNTER.toString();
            }
        } else {
            el.classList.remove("active");
            activeSet.delete(keyName);
        }
    }

    applyStyles(opts) {
        const pressscalevalue = parseInt(opts.pressscale) / 100;
        const animDuration = (0.15 * (100 / parseInt(opts.animationspeed))) + "s";

        const activeColorRgb = this.utils.hexToRgba(opts.activecolor, 1);
        const activeColorForGradient = activeColorRgb.replace(/, [\d.]+?\)/, ", 0.3)");
        const fontWeight = opts.boldfont ? 999 : 1;
        const gapModifier = (opts.gapmodifier / 100).toFixed(2);

        this.utils.applyFontStyles(opts.fontfamily);

        let styleEl = document.getElementById("dynamic-styles");
        if (!styleEl) {
            styleEl = document.createElement("style");
            styleEl.id = "dynamic-styles";
            document.head.appendChild(styleEl);
        }

        styleEl.textContent = `
            :root {
                --active-color: ${opts.activecolor};
                --font-weight: ${fontWeight};
                --gap-modifier: ${gapModifier};
            }
            .key, .mouse-btn, .scroll-display {
                border-radius: ${opts.borderradius}px !important;
                color: ${opts.inactivecolor} !important;
                background: ${opts.backgroundcolor} !important;
                border-color: ${opts.outlinecolor} !important;
                transition: all ${animDuration} cubic-bezier(0.4,0,0.2,1) !important;
                position: relative !important;
                font-weight: ${fontWeight} !important;
            }
            .key.active, .mouse-btn.active, .scroll-display.active {
                border-color: ${opts.activecolor} !important;
                box-shadow: 0 2px ${opts.glowradius}px ${opts.activecolor} !important;
                color: ${opts.fontcolor} !important;
                transform: translateY(-2px) scale(${pressscalevalue}) !important;
                background: ${opts.activebgcolor} !important;
            }
            .key.active::before, .mouse-btn.active::before, .scroll-display.active::before {
                background: linear-gradient(135deg, ${activeColorForGradient}, ${activeColorForGradient}) !important;
            }

            .mouse-btn.mouse-side {
                padding: 5px;
            }
            .mouse-btn.mouse-side span {
                background: ${opts.backgroundcolor} !important;
                border-color: ${opts.outlinecolor} !important;
                color: ${opts.inactivecolor} !important;
                width: 18px !important;
                transition: all ${animDuration} cubic-bezier(0.4,0,0.2,1) !important;
            }
            .mouse-btn.mouse-side span.active {
                border-color: ${opts.activecolor} !important;
                box-shadow: 0 0 ${opts.glowradius}px ${opts.activecolor} !important;
                color: ${opts.fontcolor} !important;
                background: ${opts.activebgcolor} !important;
                transform: scale(${pressscalevalue}) !important;
            }

            .scroll-count {
                color: ${opts.fontcolor} !important;
                display: ${opts.hidescrollcombo ? "none" : "flex"} !important;
                font-weight: ${fontWeight} !important;
            }
            .mouse-section {
                display: ${opts.hidemouse ? "none" : "flex"} !important;
            }
        `;
    }

    applyTransformations(targetElement, settings) {
        const scaleVal = parseInt(settings.scale) / 100;
        const opacityVal = parseInt(settings.opacity) / 100;

        targetElement.style.transform = `scale(${scaleVal})`;
        targetElement.style.opacity = opacityVal.toString();
        targetElement.style.transformOrigin = document.getElementById("overlay").classList.contains("show") ? "top left" : "center";
    }

    applyContainerTransformations(settings) {
        const previewContainer = document.querySelector(".preview-container") || document.getElementById("inner-overlay-container");
        if (previewContainer) {
            this.applyTransformations(previewContainer, settings);
        }
    }

    createKeyOrButtonElement(elementDef) {
        const el = document.createElement("div");

        let baseClass = "key";

        el.className = baseClass + (elementDef.class ? " " + elementDef.class : "");
        el.textContent = elementDef.label;
        el.dataset.key = elementDef.key;

        return el;
    }

    createScrollDisplay(labels, customClass) {
        const scrollDisplay = document.createElement("div");
        scrollDisplay.className = "scroll-display" + (customClass ? " " + customClass : "");
        scrollDisplay.id = "scrolldisplay";
        scrollDisplay.dataset.button = "mouse_middle";

        const scrollArrow = document.createElement("span");
        scrollArrow.className = "scroll-arrow";
        scrollArrow.textContent = labels[0];

        const scrollCount = document.createElement("span");
        scrollCount.className = "scroll-count";

        scrollDisplay.appendChild(scrollArrow);
        scrollDisplay.appendChild(scrollCount);

        scrollDisplay.dataset.defaultLabel = labels[0];
        scrollDisplay.dataset.upLabel = labels[1];
        scrollDisplay.dataset.downLabel = labels[2];

        return {el: scrollDisplay, arrow: scrollArrow, count: scrollCount};
    }

    createSideMouseButton(labelM4, labelM5, customClass) {
        const el = document.createElement("div");
        el.className = "mouse-btn mouse-side" + (customClass ? " " + customClass : "");

        const m4El = document.createElement("span");
        m4El.textContent = labelM4;
        m4El.dataset.key = "mouse4";

        const m5El = document.createElement("span");
        m5El.textContent = labelM5;
        m5El.dataset.key = "mouse5";
        el.appendChild(m5El);
        el.appendChild(m4El);

        return {el, m4El, m5El};
    }

    buildInterface(keyboardContainer, mouseContainer, layoutDef, mouseLayoutDef) {
        if (!keyboardContainer) {
            return null;
        }

        if (!mouseContainer) {
            return null;
        }

        if (!layoutDef) {
            return null;
        }

        keyboardContainer.innerHTML = "";
        mouseContainer.innerHTML = "";

        const keyElements = new Map();
        const mouseElements = new Map();
        let scrollDisplays = [];
        let scrollArrows = [];
        let scrollCounts = [];

        const allRows = [...layoutDef];
        if (mouseLayoutDef && mouseLayoutDef.length > 0) {
            allRows.push({isMouse: true, items: mouseLayoutDef});
        }

        allRows.forEach((row) => {
            const items = row.isMouse ? row.items : row;
            const rowEl = document.createElement("div");
            rowEl.className = row.isMouse ? "mouse-row" : "key-row";

            items.forEach(item => {
                if (item.type === "scroller") {
                    const display = this.createScrollDisplay(item.labels, item.class);
                    rowEl.appendChild(display.el);
                    scrollDisplays.push(display.el);
                    scrollArrows.push(display.arrow);
                    scrollCounts.push(display.count);

                    if (!mouseElements.has("mouse_middle")) {
                        mouseElements.set("mouse_middle", []);
                    }
                    mouseElements.get("mouse_middle").push(display.el);
                } else if (item.type === "mouse_side") {
                    const sideBtn = this.createSideMouseButton(item.labels[0], item.labels[1], item.class);
                    rowEl.appendChild(sideBtn.el);

                    if (!mouseElements.has("mouse5")) {
                        mouseElements.set("mouse5", []);
                    }
                    mouseElements.get("mouse5").push(sideBtn.m5El);

                    if (!mouseElements.has("mouse4")) {
                        mouseElements.set("mouse4", []);
                    }
                    mouseElements.get("mouse4").push(sideBtn.m4El);
                } else {
                    const el = this.createKeyOrButtonElement(item);
                    rowEl.appendChild(el);

                    if (!item.class || (item.class !== "invisible" && item.class !== "dummy")) {
                        const targetMap = item.type === "mouse" ? mouseElements : keyElements;

                        if (!targetMap.has(item.key)) {
                            targetMap.set(item.key, []);
                        }
                        targetMap.get(item.key).push(el);
                    }
                }
            });

            if (row.isMouse) {
                const mouseSection = document.createElement("div");
                mouseSection.className = "mouse-section";
                mouseSection.appendChild(rowEl);
                mouseContainer.appendChild(mouseSection);
            } else {
                keyboardContainer.appendChild(rowEl);
            }
        });

        return {
            keyElements,
            mouseElements,
            scrollDisplay: scrollDisplays[0] || null,
            scrollDisplays: scrollDisplays,
            scrollArrow: scrollArrows[0] || null,
            scrollArrows: scrollArrows,
            scrollCount: scrollCounts[0] || null,
            scrollCounts: scrollCounts
        };
    }

    rebuildInterface(settings) {
        const isOverlayMode = document.getElementById("overlay").classList.contains("show");

        const previewKeys = isOverlayMode
            ? document.getElementById("keyboard-target")
            : document.getElementById("preview-keyboard");

        const previewMouse = isOverlayMode
            ? document.getElementById("mouse-target")
            : document.getElementById("preview-mouse");

        const layouts = {
            keyboard: this.layoutParser.getKeyboardLayoutDef(settings),
            mouse: this.layoutParser.getMouseLayoutDef(settings)
        };

        this.previewElements = this.buildInterface(
            previewKeys,
            previewMouse,
            layouts.keyboard,
            layouts.mouse
        );

        this.restoreActiveStates();
        this.adjustScrollDisplays();
        this.adjustKeyFontSizes();
    }

    restoreActiveStates() {
        if (!this.previewElements) return;
        const oldActiveKeys = new Set(this.activeKeys);
        const oldActiveMouseButtons = new Set(this.activeMouseButtons);

        this.restoreActiveElements(oldActiveKeys, this.previewElements.keyElements, this.activeKeys);
        this.restoreActiveElements(oldActiveMouseButtons, this.previewElements.mouseElements, this.activeMouseButtons);
    }

    restoreActiveElements(oldActive, elementMap, currentActive) {
        oldActive.forEach(name => {
            const elements = elementMap.get(name);
            if (elements && elements.length > 0) {
                elements.forEach(el => {
                    el.style.zIndex = (this.Z_INDEX_COUNTER++).toString();
                    this.updateElementState(el, name, true, currentActive);
                });
            }
        });
    }

    adjustScrollDisplays() {
        if (!this.previewElements || !this.previewElements.scrollDisplays) return;

        this.previewElements.scrollDisplays.forEach(display => {
            const arrow = display.querySelector(".scroll-arrow");
            const count = display.querySelector(".scroll-count");

            arrow.style.transform = "none";
            count.textContent = "";
            display.classList.remove("active");
            this.lastScrollDirection = null;
            this.currentScrollCount = 0;

            arrow.textContent = display.dataset.defaultLabel || "-";

            const containerWidth = display.clientWidth - 16;
            const textWidth = this.utils.measureTextWidth(arrow);

            let finalScale = 1.1;
            if (textWidth * finalScale > containerWidth) {
                finalScale = containerWidth / textWidth;
            }
            arrow.style.transform = `scale(${finalScale})`;
        });
    }

    adjustKeyFontSizes() {
        document.querySelectorAll(".key").forEach(key => {
            key.style.fontSize = "";
            const textWidth = this.utils.measureTextWidth(key);
            const containerWidth = key.clientWidth - 24;

            if (textWidth > containerWidth) {
                this.utils.scaleKeyFontSize(key, containerWidth, textWidth);
            }
        });
    }

    handleScroll(dir) {
        const els = this.previewElements;
        if (dir === 0 || !els.scrollDisplays || els.scrollDisplays.length === 0) return;

        if (this.lastScrollDirection !== null && this.lastScrollDirection !== dir) {
            this.currentScrollCount = 0;
        }
        this.lastScrollDirection = dir;
        this.currentScrollCount++;

        els.scrollDisplays.forEach((scrollDisplay, index) => {
            const scrollArrow = els.scrollArrows[index];
            const scrollCount = els.scrollCounts[index];

            const upLabel = scrollDisplay.dataset.upLabel || "↑";
            const downLabel = scrollDisplay.dataset.downLabel || "↓";

            scrollArrow.textContent = dir === -1 ? upLabel : downLabel;

            const containerWidth = scrollDisplay.clientWidth - 16;
            const textWidth = scrollArrow.scrollWidth;

            const finalScaleActive =
                textWidth > containerWidth ? containerWidth / textWidth : 1;

            scrollArrow.style.transform = `scale(${finalScaleActive})`;

            if (scrollDisplay.dataset.button !== "mouse_middle") {
                scrollDisplay.dataset.button = "mouse_middle";
            }

            if (!scrollDisplay.classList.contains("active")) {
                this.Z_INDEX_COUNTER++;
                scrollDisplay.style.zIndex = this.Z_INDEX_COUNTER.toString();
            }
            scrollDisplay.classList.add("active");

            requestAnimationFrame(() => {
                scrollCount.textContent = this.currentScrollCount + "x";
                scrollCount.classList.remove("animate");

                if (dir === -1) {
                    scrollCount.classList.remove("scroll-down");
                    scrollCount.classList.add("scroll-up");
                } else {
                    scrollCount.classList.remove("scroll-up");
                    scrollCount.classList.add("scroll-down");
                }

                void scrollCount.offsetWidth;
                scrollCount.classList.add("animate");
            });
        });

        clearTimeout(this.scrollTimeout);
        this.scrollTimeout = setTimeout(() => {
            this.adjustScrollDisplays();
            els.scrollDisplays.forEach(display => {
                display.classList.remove("active");
            });
        }, 250);
    }
}