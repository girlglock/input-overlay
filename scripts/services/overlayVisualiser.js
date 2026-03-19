//guh
export class OverlayVisualiser {
    constructor(utils, layoutParser) {
        this.utils = utils;
        this.layoutParser = layoutParser;
        this.previewElements = null;
        this.activeKeys = new Set();
        this.activeMouseButtons = new Set();
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
    }

    updateElementState(el, keyName, isActive, activeSet) {
        const isMouseButton = keyName.startsWith("mouse_") || keyName === "scroller";

        if (isActive) {
            if (!this.activeElements.has(el)) {
                el.classList.add("active");
                this.activeElements.add(el);
                this.Z_INDEX_COUNTER++;
                el.style.zIndex = this.Z_INDEX_COUNTER.toString();

                if (this.analogMode && keyName.startsWith("key_")) {
                    el.classList.add("analog-key");
                    if (this.keyLegendMode === "inverting") {
                        const primaryLabel = el.querySelector('.key-label-primary');
                        if (primaryLabel) primaryLabel.style.setProperty('color', this.inactiveColor, 'important');
                    }
                } else {
                    const animDur = this.animDuration || '0.15s';
                    el.style.setProperty('transition', `all ${animDur} cubic-bezier(0.4,0,0.2,1)`, 'important');
                    const scale = this.pressScaleValue || 1.05;
                    el.style.setProperty('transform', `scale(${scale})`, 'important');
                }
            }
            activeSet.add(keyName);
        } else {
            el.classList.remove("active");
            el.classList.remove("analog-key");
            this.activeElements.delete(el);

            if (this.analogMode && keyName.startsWith("key_")) {
                const afterStyle = document.getElementById(`analog-depth-${el.dataset.key}`);
                if (afterStyle) {
                    afterStyle.remove();
                }
                el.style.setProperty('transform', 'scale(1)', 'important');
                const primaryLabel = el.querySelector('.key-label-primary');
                if (primaryLabel) {
                    primaryLabel.style.removeProperty('color');
                }
                const invertedLabel = el.querySelector('.key-label-inverted');
                if (invertedLabel) {
                    invertedLabel.style.clipPath = 'inset(100% 0 0 0)';
                }
            } else {
                const animDur = this.animDuration || '0.15s';
                el.style.setProperty('transition', `all ${animDur} cubic-bezier(0.4,0,0.2,1)`, 'important');
                el.style.setProperty('transform', 'scale(1)', 'important');
            }

            const elementsMap = this.previewElements.keyElements.get(keyName) || this.previewElements.mouseElements.get(keyName);
            if (elementsMap) {
                const anyActive = elementsMap.some(elem => this.activeElements.has(elem));
                if (!anyActive) {
                    activeSet.delete(keyName);
                }
            }
        }
    }

    applyStyles(opts, configMode = false) {
        const pressscalevalue = parseInt(opts.pressscale) / 100;
        const animDuration = (0.15 * (100 / parseInt(opts.animationspeed))) + "s";
        const activeColorRgb = this.utils.hexToRgba(opts.activecolor, 1);
        const activeColorForGradient = activeColorRgb.replace(/, [\d.]+?\)/, ", 0.3)");
        const fontWeight = opts.boldfont ? 999 : 1;
        const gapModifier = (opts.gapmodifier / 100).toFixed(2);
        this.pressScaleValue = pressscalevalue;
        this.animDuration = animDuration;
        this.activeColor = opts.activecolor;
        this.activeBgColor = opts.activebgcolor;
        this.glowRadius = opts.glowradius;
        this.inactiveColor = opts.inactivecolor;
        this.fontColor = opts.fontcolor;
        this.outlineScalePressed = parseFloat(opts.outlinescalepressed ?? opts.outlineScalePressed ?? 1);
        this.outlineScaleUnpressed = parseFloat(opts.outlinescaleunpressed ?? opts.outlineScaleUnpressed ?? 1);

        this.keyLegendMode = opts.keylegendmode || "fading";
        this.forceDisableAnalog = opts.forcedisableanalog === true || opts.forcedisableanalog === "true" || opts.forcedisableanalog === "1";

        this.utils.applyFontStyles(opts.fontfamily);

        let styleEl = document.getElementById("dynamic-styles");
        if (!styleEl) {
            styleEl = document.createElement("style");
            styleEl.id = "dynamic-styles";
            document.head.appendChild(styleEl);
        }

        const activeTransform = this.analogMode
            ? "translateY(-2px)"
            : `translateY(-2px) scale(${pressscalevalue})`;

        const transitionStyle = this.analogMode
            ? `color ${animDuration} cubic-bezier(0.4,0,0.2,1), border-color ${animDuration} cubic-bezier(0.4,0,0.2,1), box-shadow ${animDuration} cubic-bezier(0.4,0,0.2,1), transform 0.05s cubic-bezier(0.4,0,0.2,1)`
            : `all ${animDuration} cubic-bezier(0.4,0,0.2,1)`;

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
            }
            .key, .mouse-btn {
                overflow: hidden !important;
            }
            .scroll-display {
                overflow: visible !important;
            }
            .key::after, .mouse-btn::after {
                content: '';
                position: absolute;
                bottom: 0;
                left: 0;
                right: 0;
                height: 0%;
                background: ${opts.activebgcolor};
                z-index: -1;
                pointer-events: none;
            }
            .key, .mouse-btn {
                z-index: 1;
            }
            .key > *, .mouse-btn > * {
                position: relative;
                z-index: 2;
            }
            .key-label-primary {
                display: flex;
                align-items: center;
                justify-content: center;
                width: 100%;
                height: 100%;
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
                max-width: 200% !important;
                max-height: 200% !important;
                width: auto !important;
                height: auto !important;
                object-fit: contain !important;
                display: block !important;
                margin: auto !important;
                pointer-events: none !important;
                position: relative;
                z-index: 2;
            }
            
            .key, .mouse-btn, .scroll-display {
                display: flex !important;
                align-items: center !important;
                justify-content: center !important;
                border-color: ${opts.outlinecolor} !important;
                border-width: ${opts.outlinescaleunpressed ?? 1}px !important;
            }
            
            .scroll-arrow img {
                max-width: 90% !important;
                max-height: 90% !important;
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
                text-shadow: ${(c=>'1px 0 2px '+c+', -1px 0 2px '+c+', 0 1px 2px '+c+', 0 -1px 2px '+c)(opts.activecolor+'ff')} !important;
            }
            .mouse-section {
                display: ${opts.hidemouse ? "none" : "flex"} !important;
            }
        `;
    }

    createKeyOrButtonElement(elementDef) {
        const el = document.createElement("div");

        let baseClass = "key";

        el.className = baseClass + (elementDef.class ? " " + elementDef.class : "");
        el.dataset.key = elementDef.key;

        if (elementDef.label !== undefined && elementDef.label !== null) {
            const primaryLabel = document.createElement("span");
            primaryLabel.className = "key-label-primary";
            primaryLabel.innerHTML = elementDef.label;
            el.appendChild(primaryLabel);

            const invertedLabel = document.createElement("span");
            invertedLabel.className = "key-label-inverted";
            invertedLabel.innerHTML = elementDef.label;
            el.appendChild(invertedLabel);
        }

        return el;
    }

    createScrollDisplay(labels, customClass) {
        const scrollDisplay = document.createElement("div");
        scrollDisplay.className = "scroll-display" + (customClass ? " " + customClass : "");
        scrollDisplay.id = "scrolldisplay";
        scrollDisplay.dataset.button = "mouse_middle";

        const scrollArrow = document.createElement("span");
        scrollArrow.className = "scroll-arrow";
        scrollArrow.innerHTML = labels[0];

        const scrollCount = document.createElement("span");
        scrollCount.className = "scroll-count";

        scrollDisplay.appendChild(scrollArrow);
        scrollDisplay.appendChild(scrollCount);

        scrollDisplay.dataset.defaultLabel = labels[0];
        scrollDisplay.dataset.upLabel = labels[1];
        scrollDisplay.dataset.downLabel = labels[2];

        return { el: scrollDisplay, arrow: scrollArrow, count: scrollCount };
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
        el.appendChild(m5El);
        el.appendChild(m4El);

        return { el, m4El, m5El };
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

        this.scrollerAliases.clear();

        const allRows = [...layoutDef];
        if (mouseLayoutDef && mouseLayoutDef.length > 0) {
            allRows.push({ isMouse: true, items: mouseLayoutDef });
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

                    if (item.keys && item.keys.length > 0) {
                        item.keys.forEach((keyName, index) => {
                            if (keyName !== "scroller") {
                                const isMouseButton = keyName.startsWith("mouse_");
                                const targetMap = isMouseButton ? mouseElements : keyElements;

                                if (!targetMap.has(keyName)) {
                                    targetMap.set(keyName, []);
                                }
                                targetMap.get(keyName).push(display.el);

                                if (index === 1) {
                                    this.scrollerAliases.set(keyName, -1);
                                } else if (index === 2) {
                                    this.scrollerAliases.set(keyName, 1);
                                } else {
                                    this.scrollerAliases.set(keyName, 1);
                                }
                            }
                        });
                    }
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

                        const keysToRegister = item.keys || [item.key];
                        keysToRegister.forEach(keyName => {
                            if (!targetMap.has(keyName)) {
                                targetMap.set(keyName, []);
                            }
                            targetMap.get(keyName).push(el);
                        });
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
        this.adjustKeyFontSizes(parseFloat(this.outlineScaleUnpressed) || 0);
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

            arrow.innerHTML = display.dataset.defaultLabel || "-";

            const containerWidth = display.clientWidth - 16;
            const textWidth = this.utils.measureTextWidth(arrow);

            let finalScale = 1.1;
            if (textWidth * finalScale > containerWidth) {
                finalScale = containerWidth / textWidth;
            }
            arrow.style.transform = `scale(${finalScale})`;
        });
    }

    adjustKeyFontSizes(unpressedBorderWidth = 0) {
        document.querySelectorAll(".key").forEach(key => {
            key.style.fontSize = "";
            const labelEl = key.querySelector('.key-label-primary') || key;
            const textWidth = this.utils.measureTextWidth(labelEl);
            const styles = window.getComputedStyle(key);
            const keyWidth = parseFloat(styles.getPropertyValue('--key-width')) || 50;
            const containerWidth = keyWidth - (unpressedBorderWidth * 2);

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

            scrollArrow.innerHTML = dir === -1 ? upLabel : downLabel;

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

                if (this.analogMode) {
                    const animDur = this.animDuration || '0.15s';
                    scrollDisplay.style.setProperty('transition', `color ${animDur} cubic-bezier(0.4,0,0.2,1), background ${animDur} cubic-bezier(0.4,0,0.2,1), border-color ${animDur} cubic-bezier(0.4,0,0.2,1), box-shadow ${animDur} cubic-bezier(0.4,0,0.2,1), transform 0.05s cubic-bezier(0.4,0,0.2,1)`, 'important');
                    const scale = this.pressScaleValue || 1.05;
                    scrollDisplay.style.setProperty('transform', `scale(${scale})`, 'important');
                }
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
                if (this.analogMode) {
                    display.style.setProperty('transform', 'scale(1)', 'important');
                }
            });
        }, 250);
    }

    lerpColor(hexA, hexB, t) {
        const parse = (hex) => {
            if (!hex) return [128, 128, 128];
            const h = hex.replace('#', '');
            const full = h.length === 3
                ? h.split('').map(c => parseInt(c + c, 16))
                : [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
            return full;
        };
        const a = parse(hexA);
        const b = parse(hexB);
        const r = Math.round(a[0] + (b[0] - a[0]) * t);
        const g = Math.round(a[1] + (b[1] - a[1]) * t);
        const bl = Math.round(a[2] + (b[2] - a[2]) * t);
        return `rgb(${r},${g},${bl})`;
    }

    setAnalogDepthTarget(keyName, depth) {
        this.analogTargetDepths[keyName] = depth;
        if (this.analogCurrentDepths[keyName] === undefined) {
            this.analogCurrentDepths[keyName] = 0;
        }
        if (!this.analogRafId) {
            this.analogRafId = requestAnimationFrame(this._analogRafLoop);
        }
    }

    //stolen from an innocent browser game
    _analogRafLoop() {
        this.analogRafId = null;
        if (!this.previewElements) return;

        const LERP = 0.35; //60fps
        const SNAP = 0.001;
        let anyActive = false;

        for (const keyName of Object.keys(this.analogTargetDepths)) {
            const target = this.analogTargetDepths[keyName];
            let current = this.analogCurrentDepths[keyName] ?? 0;

            const delta = target - current;
            if (Math.abs(delta) < SNAP) {
                current = target;
            } else {
                current = current + delta * LERP;
                anyActive = true;
            }
            this.analogCurrentDepths[keyName] = current;

            this._renderAnalogDepth(keyName, current);

            if (current === 0 && target === 0) {
                delete this.analogTargetDepths[keyName];
                delete this.analogCurrentDepths[keyName];
            }
        }

        if (anyActive || Object.keys(this.analogTargetDepths).length > 0) {
            this.analogRafId = requestAnimationFrame(this._analogRafLoop);
        }
    }

    _renderAnalogDepth(keyName, depth) {
        if (!this.previewElements) return;
        const elements = this.previewElements.keyElements.get(keyName);
        if (!elements || elements.length === 0) return;

        const depthThreshold = 0.15;
        const effectiveDepth = depth < depthThreshold ? 0 : depth;
        const maxScale = this.pressScaleValue || 1.05;
        const scale = 1 + ((maxScale - 1) * effectiveDepth);

        const unpressedWidth = this.outlineScaleUnpressed ?? 2;
        const pressedWidth = this.outlineScalePressed ?? 2;
        const glowRadius = this.glowRadius || '24px';
        const keyLegendMode = this.keyLegendMode || "fading";

        elements.forEach(el => {
            const uniqueId = `${keyName}-${el.dataset.key || ''}`;
            let styleEl = document.getElementById(`analog-depth-${uniqueId}`);
            if (!styleEl) {
                styleEl = document.createElement("style");
                styleEl.id = `analog-depth-${uniqueId}`;
                document.head.appendChild(styleEl);
            }

            if (effectiveDepth > 0) {
                el.classList.add("analog-key");
            } else if (!el.classList.contains("active")) {
                el.classList.remove("analog-key");
            }

            el.style.setProperty('transform', `scale(${scale})`, 'important');

            const isDigitallyPressed = this.activeKeys.has(keyName);
            const fillHeight = effectiveDepth * 100;
            const borderWidth = isDigitallyPressed
                ? unpressedWidth + (pressedWidth - unpressedWidth) * Math.min(1, depth * 3)
                : unpressedWidth;
            const outerGlow = isDigitallyPressed && effectiveDepth > 0 ? `0 2px ${glowRadius} ${this.activeColor}` : 'none';

            el.style.setProperty('border-width', `${borderWidth}px`, 'important');

            styleEl.textContent = `
                [data-key="${el.dataset.key || keyName}"]::after {
                    height: ${fillHeight}% !important;
                }
                [data-key="${el.dataset.key || keyName}"].analog-key {
                    border-color: ${isDigitallyPressed ? this.activeColor : 'inherit'} !important;
                    box-shadow: ${outerGlow} !important;
                }
            `;

            const primaryLabel = el.querySelector('.key-label-primary');
            const invertedLabel = el.querySelector('.key-label-inverted');

            if (keyLegendMode === "fading") {
                if (primaryLabel) primaryLabel.style.color = this.lerpColor(this.inactiveColor, this.fontColor, Math.min(1, depth));
                if (invertedLabel) invertedLabel.style.clipPath = 'inset(100% 0 0 0)';
            } else if (keyLegendMode === "inverting") {
                if (primaryLabel) primaryLabel.style.setProperty('color', this.inactiveColor, 'important');
                if (invertedLabel) {
                    const clipTop = ((1 - effectiveDepth) * 100).toFixed(2);
                    invertedLabel.style.clipPath = `inset(${clipTop}% 0 0 0)`;
                }
            } else {
                if (primaryLabel) primaryLabel.style.removeProperty('color');
                if (invertedLabel) invertedLabel.style.clipPath = 'inset(100% 0 0 0)';
            }
        });
    }
}