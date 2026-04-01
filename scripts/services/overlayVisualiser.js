export class OverlayVisualiser {
    constructor(utils, layoutParser) {
        this.utils = utils;
        this.layoutParser = layoutParser;
        this.previewElements = null;
        this.activeKeys = new Set();
        this.activeMouseButtons = new Set();
        this.activeGamepadButtons = new Set();
        this.activeElements = new Set();
        this.scrollerAliases = new Map();
        this.currentScrollCount = 0;
        this.lastScrollDirection = null;
        this.scrollTimeout = null;
        this.Z_INDEX_COUNTER = 100;

        this.analogMode = false;
        this.analogTargetDepths = {};
        this.analogCurrentDepths = {};
        this.analogRafId = null;
        this._analogRafLoop = this._analogRafLoop.bind(this);

        this.mousePadCanvas = null;
        this.mousePadCtx = null;
        this.mousePadTrail = [];
        this.mousePadCursorX = null;
        this.mousePadCursorY = null;
        this.mousePadRafId = null;
        this.MOUSEPAD_TRAIL_MS = 600;
        this.MOUSEPAD_TRAIL_PX = 2.5;
        this.MOUSEPAD_SENSITIVITY = 1.0;
        this.MOUSEPAD_RECENTER = true;
        this._mousePadRafLoop = this._mousePadRafLoop.bind(this);

        this._activeColorRGB = null;

        this._joystickCanvases = {};
        this._joystickRafLoops = {};
    }

    updateElementState(el, keyName, isActive, activeSet) {
        if (isActive) {
            if (this.activeElements.has(el)) { activeSet.add(keyName); return; }

            el.classList.add("active");
            this.activeElements.add(el);
            el.style.zIndex = (++this.Z_INDEX_COUNTER).toString();

            if (this.analogMode && (keyName.startsWith("key_") || keyName === "gp_lt" || keyName === "gp_rt")) {
                el.classList.add("analog-key");
                if (this.keyLegendMode === "inverting") {
                    const primary = el.querySelector(".key-label-primary");
                    if (primary) primary.style.setProperty("color", this.inactiveColor, "important");
                }
            } else if (keyName === "gp_lt" || keyName === "gp_rt") {
                el.classList.add("analog-key");
            } else {
                const t = `all ${this.animDuration || "0.15s"} cubic-bezier(0.4,0,0.2,1)`;
                el.style.setProperty("transition", t, "important");
                el.style.setProperty("transform", `scale(${this.pressScaleValue || 1.05})`, "important");
            }
            activeSet.add(keyName);
        } else {
            el.classList.remove("active", "analog-key");
            this.activeElements.delete(el);

            if (this.analogMode && (keyName.startsWith("key_") || keyName === "gp_lt" || keyName === "gp_rt")) {
                document.getElementById(`analog-depth-${el.dataset.key}`)?.remove();
                el.style.setProperty("transform", "scale(1)", "important");
                el.querySelector(".key-label-primary")?.style.removeProperty("color");
                const inv = el.querySelector(".key-label-inverted");
                if (inv) inv.style.clipPath = "inset(100% 0 0 0)";
            } else if (keyName === "gp_lt" || keyName === "gp_rt") {
                document.getElementById(`analog-depth-${el.dataset.key}`)?.remove();
                el.style.setProperty("transform", "scale(1)", "important");
                el.querySelector(".key-label-primary")?.style.removeProperty("color");
                const inv = el.querySelector(".key-label-inverted");
                if (inv) inv.style.clipPath = "inset(100% 0 0 0)";
            } else {
                const t = `all ${this.animDuration || "0.15s"} cubic-bezier(0.4,0,0.2,1)`;
                el.style.setProperty("transition", t, "important");
                el.style.setProperty("transform", "scale(1)", "important");
            }

            const map = this.previewElements?.keyElements.get(keyName) || this.previewElements?.mouseElements.get(keyName) || this.previewElements?.gamepadElements?.get(keyName);
            if (map && !map.some(e => this.activeElements.has(e))) activeSet.delete(keyName);
        }
    }

    applyStyles(opts) {
        const pressscalevalue = parseInt(opts.pressscale) / 100;
        const animDuration = `${0.15 * (100 / parseInt(opts.animationspeed))}s`;
        const activeColorRgb = this.utils.hexToRgba(opts.activecolor, 1);
        const activeColorForGradient = activeColorRgb.replace(/, [\d.]+?\)/, ", 0.3)");
        const fontWeight = opts.boldfont ? 999 : 1;
        const gapModifier = (opts.gapmodifier / 100).toFixed(2);

        this.pressScaleValue = pressscalevalue;
        this.animDuration = animDuration;
        this.activeColor = opts.activecolor;
        this.activeBgColor = opts.activebgcolor;
        this.backgroundcolor = opts.backgroundcolor;
        this.glowRadius = opts.glowradius;
        this.inactiveColor = opts.inactivecolor;
        this.outlineColor = opts.outlinecolor;
        this.fontColor = opts.fontcolor;
        this.outlineScalePressed = parseFloat(opts.outlinescalepressed ?? opts.outlineScalePressed ?? 1);
        this.outlineScaleUnpressed = parseFloat(opts.outlinescaleunpressed ?? opts.outlineScaleUnpressed ?? 1);
        this.keyLegendMode = opts.keylegendmode || "fading";
        this.forceDisableAnalog = opts.forcedisableanalog === true || opts.forcedisableanalog === "true" || opts.forcedisableanalog === "1";

        this.MOUSEPAD_TRAIL_MS = parseInt(opts.mousetrailfadeout) || 600;
        this.MOUSEPAD_SENSITIVITY = (parseInt(opts.mousetrailsensitivity) || 100) / 100;
        this.MOUSEPAD_RECENTER = opts.mousetrailrecenter !== false && opts.mousetrailrecenter !== "false" && opts.mousetrailrecenter !== "0";

        const hex = opts.activecolor.replace("#", "");
        this._activeColorRGB = [parseInt(hex.slice(0, 2), 16), parseInt(hex.slice(2, 4), 16), parseInt(hex.slice(4, 6), 16)];

        this.utils.applyFontStyles(opts.fontfamily);

        let styleEl = document.getElementById("dynamic-styles");
        if (!styleEl) {
            styleEl = document.createElement("style");
            styleEl.id = "dynamic-styles";
            document.head.appendChild(styleEl);
        }

        const activeTransform = this.analogMode ? "translateY(-2px)" : `translateY(-2px) scale(${pressscalevalue})`;
        const transitionStyle = this.analogMode
            ? `color ${animDuration} cubic-bezier(0.4,0,0.2,1), border-color ${animDuration} cubic-bezier(0.4,0,0.2,1), box-shadow ${animDuration} cubic-bezier(0.4,0,0.2,1), transform 0.05s cubic-bezier(0.4,0,0.2,1)`
            : `all ${animDuration} cubic-bezier(0.4,0,0.2,1)`;

        const fontColorInt = parseInt(opts.fontcolor.replace("#", ""), 16);
        const shadowColor = (fontColorInt > 0xFFFFFF / 2 ? "#000000" : "#ffffff") + "ff";
        const textShadow = `1px 0 1px ${shadowColor}, -1px 0 1px ${shadowColor}, 0 1px 2px ${shadowColor}, 0 -1px 1px ${shadowColor}`;

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
                transition: ${transitionStyle} !important;
                position: relative !important;
                font-weight: ${fontWeight} !important;
                display: flex !important;
                align-items: center !important;
                justify-content: center !important;
                border-width: ${opts.outlinescaleunpressed ?? 1}px !important;
            }
            .key, .mouse-btn { overflow: hidden !important; }
            .scroll-display { overflow: visible !important; }
            .key::after, .mouse-btn::after {
                content: '';
                position: absolute;
                bottom: 0; left: 0; right: 0;
                height: 0%;
                background: ${opts.activebgcolor};
                z-index: -1;
                pointer-events: none;
            }
            .key, .mouse-btn { z-index: 1; }
            .key > *, .mouse-btn > * { position: relative; z-index: 2; }
            .key-label-primary {
                display: flex;
                align-items: center;
                justify-content: center;
                width: 100%; height: 100%;
            }
            .key-label-inverted {
                display: flex;
                align-items: center;
                justify-content: center;
                position: absolute;
                inset: 0;
                color: ${opts.fontcolor} !important;
                clip-path: inset(100% 0 0 0);
                pointer-events: none;
                z-index: 3;
            }
            .key.active, .mouse-btn.active, .scroll-display.active {
                color: ${opts.fontcolor} !important;
                transform: ${activeTransform} !important;
                border-color: ${opts.activecolor} !important;
                box-shadow: 0 2px ${opts.glowradius}px ${opts.activecolor} !important;
                border-width: ${opts.outlinescalepressed ?? 1}px !important;
            }
            .key.active:not(.analog-key), .mouse-btn.active:not(.analog-key), .scroll-display.active:not(.analog-key) {
                background: ${opts.activebgcolor} !important;
            }
            .key.active::before, .mouse-btn.active::before, .scroll-display.active::before {
                background: linear-gradient(135deg, ${activeColorForGradient}, ${activeColorForGradient}) !important;
            }
            .key img, .mouse-btn img, .scroll-display img {
                max-width: 200% !important; max-height: 200% !important;
                width: auto !important; height: auto !important;
                object-fit: contain !important; display: block !important;
                margin: auto !important; pointer-events: none !important;
                position: relative; z-index: 2;
            }
            .scroll-arrow img { max-width: 90% !important; max-height: 90% !important; }
            .mouse-btn.mouse-side { padding: 5px; }
            .mouse-btn.mouse-side span {
                background: ${opts.backgroundcolor} !important;
                border-color: ${opts.outlinecolor} !important;
                color: ${opts.inactivecolor} !important;
                width: 18px !important;
                transition: all ${animDuration} cubic-bezier(0.4,0,0.2,1) !important;
                display: flex !important;
                align-items: center !important;
                justify-content: center !important;
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
                text-shadow: ${textShadow} !important;
            }
            .mouse-section { display: ${opts.hidemouse ? "none" : "flex"} !important; }
            .mouse-section { display: flex !important; }
        `;
    }

    createKeyOrButtonElement(elementDef) {
        const el = document.createElement("div");
        el.className = "key" + (elementDef.class ? " " + elementDef.class : "");
        el.dataset.key = elementDef.key;

        if (elementDef.label != null) {
            const primary = document.createElement("span");
            primary.className = "key-label-primary";
            primary.innerHTML = elementDef.label;
            el.appendChild(primary);

            const inverted = document.createElement("span");
            inverted.className = "key-label-inverted";
            inverted.innerHTML = elementDef.label;
            el.appendChild(inverted);
        }
        return el;
    }

    createScrollDisplay(labels, customClass) {
        const scrollDisplay = document.createElement("div");
        scrollDisplay.className = "scroll-display" + (customClass ? " " + customClass : "");
        scrollDisplay.id = "scrolldisplay";
        scrollDisplay.dataset.button = "mouse_middle";
        scrollDisplay.dataset.defaultLabel = labels[0];
        scrollDisplay.dataset.upLabel = labels[1];
        scrollDisplay.dataset.downLabel = labels[2];

        const arrow = document.createElement("span");
        arrow.className = "scroll-arrow";
        arrow.innerHTML = labels[0];

        const count = document.createElement("span");
        count.className = "scroll-count";

        scrollDisplay.append(arrow, count);
        return { el: scrollDisplay, arrow, count };
    }

    createSideMouseButton(labelM4, labelM5, customClass) {
        const el = document.createElement("div");
        el.className = "mouse-btn mouse-side" + (customClass ? " " + customClass : "");

        const m4El = document.createElement("span");
        m4El.innerHTML = labelM4;
        m4El.dataset.key = "mouse4";

        const m5El = document.createElement("span");
        m5El.innerHTML = labelM5;
        m5El.dataset.key = "mouse5";

        el.append(m5El, m4El);
        return { el, m4El, m5El };
    }

    buildInterface(keyboardContainer, mouseContainer, layoutDef, mouseLayoutDef) {
        if (!keyboardContainer || !mouseContainer || !layoutDef) return null;

        keyboardContainer.innerHTML = "";
        mouseContainer.innerHTML = "";

        const keyElements = new Map();
        const mouseElements = new Map();
        const gamepadElements = new Map();
        const scrollDisplays = [], scrollArrows = [], scrollCounts = [];

        this.scrollerAliases.clear();

        const register = (map, name, el) => {
            let arr = map.get(name);
            if (!arr) { arr = []; map.set(name, arr); }
            arr.push(el);
        };

        const allRows = layoutDef.map(r => ({ isMouse: false, items: r }));
        if (mouseLayoutDef?.length) {
            for (const mouseRow of mouseLayoutDef) allRows.push({ isMouse: true, items: mouseRow });
        }

        const kbFrag = document.createDocumentFragment();
        const msFrag = document.createDocumentFragment();

        for (const row of allRows) {
            const rowEl = document.createElement("div");
            rowEl.className = row.isMouse ? "mouse-row" : "key-row";
            rowEl.style.position = "relative";

            for (let itemIdx = 0; itemIdx < row.items.length; itemIdx++) {
                const item = row.items[itemIdx];
                if (item.type === "gp_joystick") {
                    rowEl.appendChild(this._buildJoystickElement(item));
                } else if (item.type === "mouse_pad") {
                    rowEl.appendChild(this._buildMousePadElement(item));
                } else if (item.type === "scroller") {
                    const disp = this.createScrollDisplay(item.labels, item.class);
                    rowEl.appendChild(disp.el);
                    scrollDisplays.push(disp.el);
                    scrollArrows.push(disp.arrow);
                    scrollCounts.push(disp.count);

                    register(mouseElements, "mouse_middle", disp.el);

                    if (item.keys?.length) {
                        item.keys.forEach((keyName, idx) => {
                            if (keyName === "scroller") return;
                            const map = keyName.startsWith("mouse_") ? mouseElements : keyElements;
                            register(map, keyName, disp.el);
                            this.scrollerAliases.set(keyName, idx === 1 ? -1 : 1);
                        });
                    }
                } else if (item.type === "mouse_side") {
                    const side = this.createSideMouseButton(item.labels[0], item.labels[1], item.class);
                    rowEl.appendChild(side.el);
                    register(mouseElements, "mouse5", side.m5El);
                    register(mouseElements, "mouse4", side.m4El);
                } else {
                    const el = this.createKeyOrButtonElement(item);
                    rowEl.appendChild(el);

                    if (!item.class || (item.class !== "invisible" && item.class !== "dummy")) {
                        let map;
                        if (item.type === "mouse") map = mouseElements;
                        else if ((item.keys || [item.key]).some(k => k.startsWith("gp_"))) map = gamepadElements;
                        else map = keyElements;
                        for (const keyName of (item.keys || [item.key])) register(map, keyName, el);
                    }
                }
            }

            const isPadOnly = row.items.every(i => i.type === "mouse_pad" || i.type === "gp_joystick");
            if (row.isMouse && !isPadOnly) {
                const section = document.createElement("div");
                section.className = "mouse-section";
                section.appendChild(rowEl);
                msFrag.appendChild(section);
            } else {
                kbFrag.appendChild(rowEl);
            }
        }

        keyboardContainer.appendChild(kbFrag);
        mouseContainer.appendChild(msFrag);

        return {
            keyElements, mouseElements, gamepadElements,
            scrollDisplay: scrollDisplays[0] || null,
            scrollDisplays,
            scrollArrow: scrollArrows[0] || null,
            scrollArrows,
            scrollCount: scrollCounts[0] || null,
            scrollCounts
        };
    }

    rebuildInterface(settings) {
        const isOverlay = document.getElementById("overlay").classList.contains("show");
        const previewKeys = document.getElementById(isOverlay ? "keyboard-target" : "preview-keyboard");
        const previewMouse = document.getElementById(isOverlay ? "mouse-target" : "preview-mouse");

        this.previewElements = this.buildInterface(
            previewKeys, previewMouse,
            this.layoutParser.getKeyboardLayoutDef(settings),
            this.layoutParser.getMouseLayoutDef(settings)
        );

        this.restoreActiveStates();
        this.adjustScrollDisplays();
        this.adjustKeyFontSizes(parseFloat(this.outlineScaleUnpressed) || 0);
    }

    restoreActiveStates() {
        if (!this.previewElements) return;
        this._restoreMap(new Set(this.activeKeys), this.previewElements.keyElements, this.activeKeys);
        this._restoreMap(new Set(this.activeMouseButtons), this.previewElements.mouseElements, this.activeMouseButtons);
        if (this.previewElements.gamepadElements)
            this._restoreMap(new Set(this.activeGamepadButtons), this.previewElements.gamepadElements, this.activeGamepadButtons);
    }

    _restoreMap(oldActive, elementMap, currentActive) {
        for (const name of oldActive) {
            const elements = elementMap.get(name);
            if (!elements?.length) continue;
            for (const el of elements) {
                el.style.zIndex = (++this.Z_INDEX_COUNTER).toString();
                this.updateElementState(el, name, true, currentActive);
            }
        }
    }

    adjustScrollDisplays() {
        if (!this.previewElements?.scrollDisplays) return;
        this.lastScrollDirection = null;
        this.currentScrollCount = 0;

        for (const display of this.previewElements.scrollDisplays) {
            const arrow = display.querySelector(".scroll-arrow");
            const count = display.querySelector(".scroll-count");
            arrow.innerHTML = display.dataset.defaultLabel || "-";
            arrow.style.transform = "none";
            count.textContent = "";
            display.classList.remove("active");

            const containerWidth = display.clientWidth - 16;
            const textWidth = this.utils.measureTextWidth(arrow);
            let scale = 1.1;
            if (textWidth * scale > containerWidth) scale = containerWidth / textWidth;
            arrow.style.transform = `scale(${scale})`;
        }
    }

    handleScroll(dir) {
        const els = this.previewElements;
        if (!dir || !els?.scrollDisplays?.length) return;

        if (this.lastScrollDirection !== null && this.lastScrollDirection !== dir) this.currentScrollCount = 0;
        this.lastScrollDirection = dir;
        this.currentScrollCount++;

        const count = this.currentScrollCount;
        const animDur = this.animDuration || "0.15s";

        for (let i = 0; i < els.scrollDisplays.length; i++) {
            const display = els.scrollDisplays[i];
            const arrow = els.scrollArrows[i];
            const countEl = els.scrollCounts[i];

            arrow.innerHTML = dir === -1 ? (display.dataset.upLabel || "↑") : (display.dataset.downLabel || "↓");

            const containerWidth = display.clientWidth - 16;
            const scale = arrow.scrollWidth > containerWidth ? containerWidth / arrow.scrollWidth : 1;
            arrow.style.transform = `scale(${scale})`;

            if (!display.classList.contains("active")) {
                display.style.zIndex = (++this.Z_INDEX_COUNTER).toString();
                if (this.analogMode) {
                    display.style.setProperty("transition", `color ${animDur} cubic-bezier(0.4,0,0.2,1), background ${animDur} cubic-bezier(0.4,0,0.2,1), border-color ${animDur} cubic-bezier(0.4,0,0.2,1), box-shadow ${animDur} cubic-bezier(0.4,0,0.2,1), transform 0.05s cubic-bezier(0.4,0,0.2,1)`, "important");
                    display.style.setProperty("transform", `scale(${this.pressScaleValue || 1.05})`, "important");
                }
            }
            display.classList.add("active");

            requestAnimationFrame(() => {
                countEl.textContent = count + "x";
                countEl.classList.remove("animate", "scroll-up", "scroll-down");
                countEl.classList.add(dir === -1 ? "scroll-up" : "scroll-down");
                void countEl.offsetWidth;
                countEl.classList.add("animate");
            });
        }

        clearTimeout(this.scrollTimeout);
        this.scrollTimeout = setTimeout(() => {
            this.adjustScrollDisplays();
            for (const display of els.scrollDisplays) {
                display.classList.remove("active");
                if (this.analogMode) display.style.setProperty("transform", "scale(1)", "important");
            }
        }, 250);
    }

    adjustKeyFontSizes(unpressedBorderWidth = 0) {
        for (const key of document.querySelectorAll(".key")) {
            key.style.fontSize = "";
            const labelEl = key.querySelector(".key-label-primary") || key;
            const textWidth = this.utils.measureTextWidth(labelEl);
            const keyWidth = parseFloat(window.getComputedStyle(key).getPropertyValue("--key-width")) || 50;
            const containerWidth = keyWidth - (unpressedBorderWidth * 2);
            if (textWidth > containerWidth) this.utils.scaleKeyFontSize(key, containerWidth, textWidth);
        }
    }

    setAnalogDepthTarget(keyName, depth, source) {
        this.analogTargetDepths[keyName] = depth;
        if (this.analogCurrentDepths[keyName] === undefined) this.analogCurrentDepths[keyName] = 0;
        if (!this.analogRafId) this.analogRafId = requestAnimationFrame(this._analogRafLoop);
    }

    _analogRafLoop() {
        this.analogRafId = null;
        if (!this.previewElements) return;

        const LERP = 0.35, SNAP = 0.001;
        let anyActive = false;

        for (const keyName of Object.keys(this.analogTargetDepths)) {
            const target = this.analogTargetDepths[keyName];
            let current = this.analogCurrentDepths[keyName] ?? 0;
            const delta = target - current;

            if (Math.abs(delta) < SNAP) {
                current = target;
            } else {
                current += delta * LERP;
                anyActive = true;
            }
            this.analogCurrentDepths[keyName] = current;
            this._renderAnalogDepth(keyName, current);

            if (current === 0 && target === 0) {
                delete this.analogTargetDepths[keyName];
                delete this.analogCurrentDepths[keyName];
            }
        }

        if (anyActive || Object.keys(this.analogTargetDepths).length > 0)
            this.analogRafId = requestAnimationFrame(this._analogRafLoop);
    }

    _renderAnalogDepth(keyName, depth) {
        if (!this.previewElements) return;
        const elements = this.previewElements.keyElements.get(keyName) || this.previewElements.gamepadElements?.get(keyName);
        if (!elements?.length) return;

        const depthThreshold = 0.15;
        const effectiveDepth = depth < depthThreshold ? 0 : depth;
        const maxScale = this.pressScaleValue || 1.05;
        const scale = 1 + (maxScale - 1) * effectiveDepth;

        const unpressedWidth = this.outlineScaleUnpressed ?? 2;
        const pressedWidth = this.outlineScalePressed ?? 2;
        const glowRadius = this.glowRadius || "24px";
        const keyLegendMode = this.keyLegendMode || "inverting";

        for (const el of elements) {
            const uniqueId = `${keyName}-${el.dataset.key || ""}`;
            let styleEl = document.getElementById(`analog-depth-${uniqueId}`);
            if (!styleEl) {
                styleEl = document.createElement("style");
                styleEl.id = `analog-depth-${uniqueId}`;
                document.head.appendChild(styleEl);
            }

            if (effectiveDepth > 0) el.classList.add("analog-key");
            else if (!el.classList.contains("active")) el.classList.remove("analog-key");

            el.style.setProperty("transform", `scale(${scale})`, "important");

            const isDigitallyPressed = this.activeKeys.has(keyName) || this.activeGamepadButtons?.has(keyName);
            const fillHeight = effectiveDepth * 100;
            const borderWidth = isDigitallyPressed
                ? unpressedWidth + (pressedWidth - unpressedWidth) * Math.min(1, depth * 3)
                : unpressedWidth;
            const outerGlow = isDigitallyPressed && effectiveDepth > 0 ? `0 2px ${glowRadius} ${this.activeColor}` : "none";

            el.style.setProperty("border-width", `${borderWidth}px`, "important");

            const dataKey = el.dataset.key || keyName;
            styleEl.textContent = `
                [data-key="${dataKey}"]::after { height: ${fillHeight}% !important; }
                [data-key="${dataKey}"].analog-key {
                    border-color: ${isDigitallyPressed ? this.activeColor : "inherit"} !important;
                    box-shadow: ${outerGlow} !important;
                }`;

            const primary = el.querySelector(".key-label-primary");
            const inverted = el.querySelector(".key-label-inverted");

            if (keyLegendMode === "fading") {
                if (primary) primary.style.color = this.utils.lerpColor(this.inactiveColor, this.fontColor, Math.min(1, depth));
                if (inverted) inverted.style.clipPath = "inset(100% 0 0 0)";
            } else if (keyLegendMode === "inverting") {
                if (primary) primary.style.setProperty("color", this.inactiveColor, "important");
                if (inverted) inverted.style.clipPath = `inset(${((1 - effectiveDepth) * 100).toFixed(2)}% 0 0 0)`;
            } else {
                if (primary) primary.style.removeProperty("color");
                if (inverted) inverted.style.clipPath = "inset(100% 0 0 0)";
            }
        }
    }

    _parseUClass(uStr, base = 50) {
        if (!uStr) return base;
        const m = uStr.match(/^u(\d+)(?:-(\d+))?$/);
        if (!m) return base;
        const dec = m[2] ? (m[2].length === 1 ? parseInt(m[2]) * 10 : parseInt(m[2])) : 0;
        return parseInt(m[1]) * base + Math.round(dec * base / 100);
    }

    _buildMousePadElement(item) {
        const widthPx = this._parseUClass(item.widthClass, 50);
        const heightPx = this._parseUClass(item.heightClass, 50);
        const heightMod = (heightPx / 50).toFixed(4);
        const heightCss = `calc(50px * ${heightMod})`;

        const anchor = item.anchor || "a-tl";
        const anchorV = anchor[2];
        const anchorH = anchor[3];

        const container = document.createElement("div");
        container.className = "mousepad-container";
        container.style.cssText = [
            "position:relative",
            "width:0", "min-width:0", "max-width:0",
            "height:0", "min-height:0",
            "flex-shrink:0",
            "overflow:visible",
            "pointer-events:none",
            "align-self:flex-start",
        ].join(";");

        const wrap = document.createElement("div");
        wrap.className = "mousepad-wrap key";
        wrap.style.setProperty("--key-width", `${widthPx}px`);
        wrap.style.setProperty("--key-height-modifier", heightMod);
        wrap.style.position = "absolute";
        wrap.style.zIndex = "50";
        wrap.style.width = `${widthPx}px`;
        wrap.style.height = heightCss;
        wrap.style.overflow = "hidden";
        wrap.style.pointerEvents = "none";

        if (anchorV === "t") wrap.style.top = "0";

        const canvas = document.createElement("canvas");
        canvas.className = "mousepad-canvas";
        canvas.id = `mouse_pad`;
        canvas.style.cssText = "display:block;position:absolute;inset:0;width:100%;height:100%;pointer-events:none;";
        wrap.appendChild(canvas);

        this.mousePadCanvas = canvas;
        this.mousePadCtx = canvas.getContext("2d");
        this.mousePadTrail = [];
        this.mousePadCursorX = null;
        this.mousePadCursorY = null;

        this._mousePadResizeObserver?.disconnect();
        this._mousePadResizeObserver = new ResizeObserver(() => this._resizeMousePad());
        this._mousePadResizeObserver.observe(wrap);

        const findRow = (el) => {
            let cur = el?.parentElement;
            while (cur) {
                if (cur.classList.contains("key-row") || cur.classList.contains("mouse-row")) return cur;
                cur = cur.parentElement;
            }
            return null;
        };

        requestAnimationFrame(() => requestAnimationFrame(() => {
            const row = findRow(container);
            const gap = row ? parseFloat(getComputedStyle(row).gap) || 0 : 0;
            const rowH = row ? row.getBoundingClientRect().height : heightPx;

            let left;
            if (anchorH === "l") {
                left = 0;
                container.style.marginRight = `-${gap}px`;
            } else if (anchorH === "r") {
                left = -widthPx;
                container.style.marginLeft = `-${gap}px`;
            } else {
                left = -widthPx / 2;
                container.style.marginLeft = `-${gap / 2}px`;
                container.style.marginRight = `-${gap / 2}px`;
            }
            wrap.style.left = `${left}px`;

            const innerH = row ? row.clientHeight : heightPx;
            if (anchorV === "c") {
                wrap.style.top = `${(innerH - heightPx) / 2}px`;
            } else if (anchorV === "b") {
                wrap.style.top = `${innerH - heightPx}px`;
            }

            this._resizeMousePad();
            if (typeof window.setDynamicScale === "function") window.setDynamicScale();
        }));

        container.appendChild(wrap);
        return container;
    }

    _resizeMousePad() {
        if (!this.mousePadCanvas) return;
        const wrap = this.mousePadCanvas.parentElement;
        if (!wrap) return;
        const logicalW = parseFloat(wrap.style.width) || wrap.offsetWidth;
        const logicalH = parseFloat(wrap.style.height) || wrap.offsetHeight;
        if (!logicalW || !logicalH) return;
        const dpr = window.devicePixelRatio || 1;
        this.mousePadCanvas.width = Math.round(logicalW * dpr);
        this.mousePadCanvas.height = Math.round(logicalH * dpr);
        this.mousePadCanvas.style.width = `${logicalW}px`;
        this.mousePadCanvas.style.height = `${logicalH}px`;
        this.mousePadCanvas.dataset.logicalW = logicalW;
        this.mousePadCanvas.dataset.logicalH = logicalH;
        this.mousePadCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
        this.mousePadCursorX = logicalW / 2;
        this.mousePadCursorY = logicalH / 2;
        this.mousePadTrail = [];
    }

    handleMouseMove(dx, dy) {
        if (!this.mousePadCanvas || !this.mousePadCtx) return;
        const W = parseFloat(this.mousePadCanvas.dataset.logicalW) || 0;
        const H = parseFloat(this.mousePadCanvas.dataset.logicalH) || 0;
        if (!W || !H) return;

        if (this.mousePadCursorX === null) {
            this.mousePadCursorX = W / 2;
            this.mousePadCursorY = H / 2;
        }

        const REF_W = 300, REF_H = 200;
        const scaleX = W / REF_W;
        const scaleY = H / REF_H;
        const BASE_SENSITIVITY = 0.05 * this.MOUSEPAD_SENSITIVITY;
        const prevX = this.mousePadCursorX, prevY = this.mousePadCursorY;

        this.mousePadCursorX = ((this.mousePadCursorX + dx * BASE_SENSITIVITY * scaleX) % W + W) % W;
        this.mousePadCursorY = ((this.mousePadCursorY + dy * BASE_SENSITIVITY * scaleY) % H + H) % H;

        if (Math.abs(this.mousePadCursorX - prevX) > W / 2 || Math.abs(this.mousePadCursorY - prevY) > H / 2)
            this.mousePadTrail.push(null);

        this.mousePadTrail.push({ x: this.mousePadCursorX, y: this.mousePadCursorY, t: performance.now() });
        if (this.mousePadTrail.length > 500) this.mousePadTrail.shift();

        if (!this.mousePadRafId) this.mousePadRafId = requestAnimationFrame(this._mousePadRafLoop);
    }

    _mousePadRafLoop() {
        this.mousePadRafId = null;
        if (!this.mousePadCtx || !this.mousePadCanvas) return;

        const ctx = this.mousePadCtx;
        const W = parseFloat(this.mousePadCanvas.dataset.logicalW) || this.mousePadCanvas.width;
        const H = parseFloat(this.mousePadCanvas.dataset.logicalH) || this.mousePadCanvas.height;
        const now = performance.now();
        const maxAge = this.MOUSEPAD_TRAIL_MS;

        while (this.mousePadTrail.length > 1) {
            const first = this.mousePadTrail[0];
            if (first !== null && (now - first.t) > maxAge) this.mousePadTrail.shift();
            else break;
        }

        ctx.clearRect(0, 0, W, H);

        const trail = this.mousePadTrail;
        const trailPx = this.MOUSEPAD_TRAIL_PX;
        let segStart = null;

        for (let i = 0; i < trail.length; i++) {
            const p = trail[i];
            if (p === null) { segStart = null; continue; }
            if (segStart === null) { segStart = p; continue; }

            const p0 = segStart, p1 = p;
            segStart = p;
            const age = now - (p0.t + p1.t) / 2;
            const alpha = Math.max(0, 1 - age / maxAge);
            if (alpha <= 0) continue;

            ctx.beginPath();
            ctx.moveTo(p0.x, p0.y);
            ctx.lineTo(p1.x, p1.y);
            ctx.strokeStyle = this._mousePadColor(alpha);
            ctx.lineWidth = trailPx * (0.4 + 0.6 * alpha);
            ctx.lineCap = "round";
            ctx.lineJoin = "round";
            ctx.stroke();
        }

        const tip = trail[trail.length - 1];
        if (tip) {
            const tipAlpha = Math.max(0, 1 - (now - tip.t) / maxAge);
            if (tipAlpha > 0) {
                ctx.beginPath();
                ctx.arc(tip.x, tip.y, trailPx * 1.6, 0, Math.PI * 2);
                ctx.fillStyle = this._mousePadColor(Math.min(1, tipAlpha * 1.5));
                ctx.fill();
            }
        }

        if (trail.some(p => p !== null && (now - p.t) < maxAge))
            this.mousePadRafId = requestAnimationFrame(this._mousePadRafLoop);
        else if (this.MOUSEPAD_RECENTER) {
            this.mousePadCursorX = W / 2;
            this.mousePadCursorY = H / 2;
        }
    }

    _mousePadColor(alpha) {
        if (this._activeColorRGB) {
            const [r, g, b] = this._activeColorRGB;
            return `rgba(${r},${g},${b},${alpha.toFixed(3)})`;
        }
        const hex = (this.activeColor || "#8b5cf6").replace("#", "");
        const r = parseInt(hex.slice(0, 2), 16);
        const g = parseInt(hex.slice(2, 4), 16);
        const b = parseInt(hex.slice(4, 6), 16);
        return `rgba(${r},${g},${b},${alpha.toFixed(3)})`;
    }

    _buildJoystickElement(item) {
        const widthPx = this._parseUClass(item.widthClass, 50);
        const heightPx = this._parseUClass(item.heightClass || item.widthClass, 50);
        const heightMod = (heightPx / 50).toFixed(4);
        const anchor = item.anchor || "a-tl";
        const anchorV = anchor[2];
        const anchorH = anchor[3];
        const stickId = item.stickId || item.key;

        const container = document.createElement("div");
        container.className = "joystick-container";
        container.style.cssText = [
            "position:relative", "width:0", "min-width:0", "max-width:0",
            "height:0", "min-height:0", "flex-shrink:0",
            "overflow:visible", "pointer-events:none", "align-self:flex-start",
        ].join(";");

        const wrap = document.createElement("div");
        wrap.className = "joystick-wrap";
        wrap.style.setProperty("--key-width", `${widthPx}px`);
        wrap.style.setProperty("--key-height-modifier", heightMod);
        wrap.style.position = "absolute";
        wrap.style.zIndex = "50";
        wrap.style.width = `${widthPx}px`;
        wrap.style.height = `calc(50px * ${heightMod})`;
        wrap.style.overflow = "visible";
        wrap.style.pointerEvents = "none";
        if (anchorV === "t") wrap.style.top = "0";

        const canvas = document.createElement("canvas");
        canvas.className = "joystick-canvas";
        canvas.dataset.stickId = stickId;
        canvas.style.cssText = "display:block;position:absolute;pointer-events:none;";
        wrap.appendChild(canvas);

        const state = {
            canvas, ctx: canvas.getContext("2d"),
            posX: 0.5, posY: 0.5,
            W: widthPx, H: heightPx,
            rafId: null,
        };
        this._joystickCanvases[stickId] = state;
        const loopFn = this._joystickRafLoop.bind(this, stickId);
        this._joystickRafLoops[stickId] = loopFn;

        const ro = new ResizeObserver(() => this._resizeJoystick(stickId));
        ro.observe(wrap);

        const findRow = el => {
            let cur = el?.parentElement;
            while (cur) {
                if (cur.classList.contains("key-row") || cur.classList.contains("gamepad-row") || cur.classList.contains("mouse-row")) return cur;
                cur = cur.parentElement;
            }
            return null;
        };

        requestAnimationFrame(() => requestAnimationFrame(() => {
            const row = findRow(container);
            const gap = row ? parseFloat(getComputedStyle(row).gap) || 0 : 0;
            const innerH = row ? row.clientHeight : heightPx;

            let left;
            if (anchorH === "l") { left = 0; container.style.marginRight = `-${gap}px`; }
            else if (anchorH === "r") { left = -widthPx; container.style.marginLeft = `-${gap}px`; }
            else { left = -widthPx / 2; container.style.marginLeft = `-${gap / 2}px`; container.style.marginRight = `-${gap / 2}px`; }
            wrap.style.left = `${left}px`;

            if (anchorV === "c") wrap.style.top = `${(innerH - heightPx) / 2}px`;
            else if (anchorV === "b") wrap.style.top = `${innerH - heightPx}px`;

            this._resizeJoystick(stickId);
            if (typeof window.setDynamicScale === "function") window.setDynamicScale();
        }));

        container.appendChild(wrap);
        return container;
    }

    _resizeJoystick(stickId) {
        const state = this._joystickCanvases[stickId];
        if (!state) return;
        const wrap = state.canvas.parentElement;
        if (!wrap) return;
        const logW = parseFloat(wrap.style.width) || wrap.offsetWidth;
        const logH = parseFloat(wrap.style.height) || wrap.offsetHeight;
        if (!logW || !logH) return;

        const GLOW_PAD = Math.round(Math.min(logW, logH) * 0.35);
        const totalLogW = logW + GLOW_PAD * 2;
        const totalLogH = logH + GLOW_PAD * 2;

        const dpr = window.devicePixelRatio || 1;
        state.canvas.width = Math.round(totalLogW * dpr);
        state.canvas.height = Math.round(totalLogH * dpr);
        state.canvas.style.width = `${totalLogW}px`;
        state.canvas.style.height = `${totalLogH}px`;
        state.canvas.style.left = `-${GLOW_PAD}px`;
        state.canvas.style.top = `-${GLOW_PAD}px`;

        state.ctx.setTransform(dpr, 0, 0, dpr, GLOW_PAD * dpr, GLOW_PAD * dpr);

        state.W = logW;
        state.H = logH;
        state.glowPad = GLOW_PAD;
        state.posX = 0.5;
        state.posY = 0.5;
        this._drawJoystick(stickId);
    }

    handleJoystickMove(stickId, axisX, axisY) {
        const state = this._joystickCanvases[stickId];
        if (!state) return;
        state.posX = (axisX + 1) / 2;
        state.posY = (axisY + 1) / 2;
        if (!state.rafId) state.rafId = requestAnimationFrame(this._joystickRafLoops[stickId]);
    }

    _joystickRafLoop(stickId) {
        const state = this._joystickCanvases[stickId];
        if (!state) return;
        state.rafId = null;
        this._drawJoystick(stickId);
    }

    _drawJoystick(stickId) {
        const state = this._joystickCanvases[stickId];
        if (!state?.ctx) return;
        const { ctx, W, H, posX, posY, glowPad = 0 } = state;

        ctx.clearRect(-glowPad, -glowPad, W + glowPad * 2, H + glowPad * 2);

        const dotR = Math.min(W, H) * 0.15;

        const deflection = Math.min(1, Math.sqrt((posX - 0.5) ** 2 + (posY - 0.5) ** 2) * 8);

        const inactiveHex = this.inactiveColor || "#808080";
        const activeHex = this.activeColor || "#8b5cf6";
        const bgHex = this.backgroundcolor || "#1a1a1ad1";
        const outlineHex = this.outlineColor || "#4f4f4f";

        //joy bg
        ctx.beginPath();
        ctx.ellipse(W / 2, H / 2, W / 2, H / 2, 0, 0, Math.PI * 2);
        ctx.fillStyle = bgHex;
        ctx.fill();

        //joy ring
        const unpressedW = this.outlineScaleUnpressed ?? 2;
        const pressedW = this.outlineScalePressed ?? 2;
        const ringBorderW = unpressedW + (pressedW - unpressedW) * deflection * 0.8;
        const ringColor = deflection > 0.05
            ? this.utils.lerpColor(outlineHex, activeHex, deflection * 0.8)
            : outlineHex;
        ctx.beginPath();
        ctx.ellipse(W / 2, H / 2, W / 2 - ringBorderW / 2, H / 2 - ringBorderW / 2, 0, 0, Math.PI * 2);
        ctx.strokeStyle = ringColor;
        ctx.lineWidth = ringBorderW;
        ctx.stroke();

        //dot pos
        const margin = dotR;
        const dotX = margin + posX * (W - margin * 2);
        const dotY = margin + posY * (H - margin * 2);

        //dot color
        const dotHex = this.utils.lerpColor(inactiveHex, activeHex, deflection);
        const dotMatch = dotHex.match(/\d+/g);
        const [r, g, b] = dotMatch ? dotMatch.map(Number) : [139, 92, 246];

        //dot glow
        if (deflection > 0.02) {
            const glowR = Math.min(W, H) * 0.45;
            const grd = ctx.createRadialGradient(dotX, dotY, 0, dotX, dotY, glowR);
            grd.addColorStop(0, `rgba(${r},${g},${b},${0.55 * deflection})`);
            grd.addColorStop(1, `rgba(${r},${g},${b},0)`);
            ctx.fillStyle = grd;
            ctx.fillRect(-glowPad, -glowPad, W + glowPad * 2, H + glowPad * 2);
        }

        //dot
        ctx.beginPath();
        ctx.arc(dotX, dotY, dotR, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${r},${g},${b},0.95)`;
        ctx.fill();
    }

}