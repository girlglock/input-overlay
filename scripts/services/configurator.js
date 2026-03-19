//guh
import { BROWSER_BUTTON_TO_KEY_NAME, BROWSER_CODE_TO_KEY_NAME, COLOR_PICKERS, DEFAULT_LAYOUT_STRINGS, HID_TO_KEY_NAME } from "../consts.js";

export class ConfiguratorMode {
    constructor(utils, urlManager, layoutParser, visualizer) {
        this.utils = utils;
        this.urlManager = urlManager;
        this.visualizer = visualizer;
        this.pickrInstances = {};
        this.urlDebounceTimer = null;

        document.getElementById("configurator").style.display = "block";
        document.getElementById("overlay").classList.remove("show");

        this.initDefaultLayoutValues();
        this.setupBackgroundVideo();
        this.setupCheatSheetToggle();

        setTimeout(() => {
            COLOR_PICKERS.forEach(cp => {
                this.initPickrColorInput(cp.id, cp.defaultColor);
            });
        }, 25);

        const urlParams = new URLSearchParams(window.location.search);
        const hasParams = urlParams.has("cfg") || Array.from(urlParams.keys()).some(key => key !== "ws");

        if (hasParams) {
            this.loadSettingsFromLink(true);
        } else {
            this.applyDefaultSettings();
        }

        this.setupConfigInputs();
        this.setupKeyAddButtons();
        this.setupPreviewInputListeners();
        this.setupAnalogSense();
        this.updateState();
    }

    applyDefaultSettings() {
        const defaultSettings = {
            wsaddress: "localhost",
            wsport: "16899",
            activecolor: "#5cf67d",
            inactivecolor: "#808080",
            backgroundcolor: "#1a1a1ad1",
            activebgcolor: "#47bd61",
            outlinecolor: "#4f4f4f",
            fontcolor: "#ffffff",
            glowradius: "24",
            borderradius: "1",
            pressscale: "110",
            animationspeed: "300",
            fontfamily: "",
            hidemouse: false,
            hidescrollcombo: false,
            boldfont: true,
            analogmode: false,
            gapmodifier: "100",
            outlinescalepressed: "2",
            outlinescaleunpressed: "2",
            customLayoutRow1: DEFAULT_LAYOUT_STRINGS.row1,
            customLayoutRow2: DEFAULT_LAYOUT_STRINGS.row2,
            customLayoutRow3: DEFAULT_LAYOUT_STRINGS.row3,
            customLayoutRow4: DEFAULT_LAYOUT_STRINGS.row4,
            customLayoutRow5: DEFAULT_LAYOUT_STRINGS.row5,
            customLayoutMouse: DEFAULT_LAYOUT_STRINGS.mouse,
            keylegendmode: "fading",
            forcedisableanalog: false,
        };

        this.applySettings(defaultSettings);
    }

    initDefaultLayoutValues() {
        document.getElementById("customLayoutRow1").value = document.getElementById("customLayoutRow1").value || DEFAULT_LAYOUT_STRINGS.row1;
        document.getElementById("customLayoutRow2").value = document.getElementById("customLayoutRow2").value || DEFAULT_LAYOUT_STRINGS.row2;
        document.getElementById("customLayoutRow3").value = document.getElementById("customLayoutRow3").value || DEFAULT_LAYOUT_STRINGS.row3;
        document.getElementById("customLayoutRow4").value = document.getElementById("customLayoutRow4").value || DEFAULT_LAYOUT_STRINGS.row4;
        document.getElementById("customLayoutRow5").value = document.getElementById("customLayoutRow5").value || DEFAULT_LAYOUT_STRINGS.row5;
        document.getElementById("customLayoutMouse").value = document.getElementById("customLayoutMouse").value || DEFAULT_LAYOUT_STRINGS.mouse;
    }

    getCurrentSettings() {
        return {
            wsaddress: document.getElementById("wsaddress").value || "localhost",
            wsport: document.getElementById("wsport").value || "16899",
            wsauth: document.getElementById("wsauth").value || "",
            activecolor: document.getElementById("activecolorhex").value,
            inactivecolor: document.getElementById("inactivecolorhex").value,
            backgroundcolor: document.getElementById("backgroundcolorhex").value,
            activebgcolor: document.getElementById("activebgcolorhex").value,
            outlinecolor: document.getElementById("outlinecolorhex").value,
            fontcolor: document.getElementById("fontcolorhex").value,
            glowradius: document.getElementById("glowradius").value,
            borderradius: document.getElementById("borderradius").value,
            pressscale: document.getElementById("pressscale").value,
            animationspeed: document.getElementById("animationspeed").value,
            fontfamily: document.getElementById("fontfamily").value,
            hidemouse: document.getElementById("hidemouse").checked,
            hidescrollcombo: document.getElementById("hidescrollcombo").checked,
            boldfont: document.getElementById("boldfont") ? document.getElementById("boldfont").checked : false,
            analogmode: document.getElementById("analogmode") ? document.getElementById("analogmode").checked : false,

            gapmodifier: document.getElementById("gapmodifier") ? document.getElementById("gapmodifier").value : "100",
            outlinescalepressed: document.getElementById("outlinescalepressed") ? document.getElementById("outlinescalepressed").value : "2",
            outlinescaleunpressed: document.getElementById("outlinescaleunpressed") ? document.getElementById("outlinescaleunpressed").value : "2",

            customLayoutRow1: document.getElementById("customLayoutRow1") ? document.getElementById("customLayoutRow1").value : "",
            customLayoutRow2: document.getElementById("customLayoutRow2") ? document.getElementById("customLayoutRow2").value : "",
            customLayoutRow3: document.getElementById("customLayoutRow3") ? document.getElementById("customLayoutRow3").value : "",
            customLayoutRow4: document.getElementById("customLayoutRow4") ? document.getElementById("customLayoutRow4").value : "",
            customLayoutRow5: document.getElementById("customLayoutRow5") ? document.getElementById("customLayoutRow5").value : "",
            customLayoutMouse: document.getElementById("customLayoutMouse") ? document.getElementById("customLayoutMouse").value : "",

            keylegendmode: document.getElementById("keylegendmode") ? document.getElementById("keylegendmode").value : "fading",
            forcedisableanalog: document.getElementById("forcedisableanalog") ? document.getElementById("forcedisableanalog").checked : false,
        };
    }

    updateSliderLabel(input) {
        const label = document.getElementById(input.id + "value");
        if (label) {
            let suffix = "";
            if (input.id === "outlinescalepressed" || input.id === "outlinescaleunpressed") {
                label.textContent = input.value + "px";
                return;
            }
            if (input.id.includes("radius")) suffix = "px";
            else if (input.id.includes("scale")) suffix = "x";
            else if (input.id === "opacity" || input.id.includes("speed") || input.id.includes("modifier")) suffix = "%";

            let val = input.value;
            if (input.id.includes("scale") && !input.id.includes("pressscale")) val = (val / 100).toFixed(1);
            else if (input.id === "pressscale") val = (val / 100).toFixed(2);

            label.textContent = val + suffix;
        }
    }

    applySettings(settings) {
        if (!settings) return;

        const applyValue = (id, value) => {
            const el = document.getElementById(id);
            if (el) {
                if (el.type === "checkbox") {
                    el.checked = value === "true" || value === "1" || value === true;
                    el.dispatchEvent(new Event("change", { bubbles: true }));
                } else {
                    el.value = value !== undefined && value !== null ? value : "";

                    if (id.includes("colorhex")) {
                        const pickrId = id.replace("hex", "");
                        const pickr = this.pickrInstances[pickrId];
                        if (pickr && value) {
                            try {
                                pickr.setColor(value, true);
                            } catch (error) {
                            }
                        }
                    }
                }
                if (el.type === "range") {
                    this.updateSliderLabel(el);
                    el.dispatchEvent(new Event("input", { bubbles: true }));
                }
            }
        };

        applyValue("wsaddress", settings.wsaddress);
        applyValue("wsport", settings.wsport);
        applyValue("activecolorhex", settings.activecolor);
        applyValue("inactivecolorhex", settings.inactivecolor);
        applyValue("backgroundcolorhex", settings.backgroundcolor);
        applyValue("activebgcolorhex", settings.activebgcolor);
        applyValue("outlinecolorhex", settings.outlinecolor);
        applyValue("fontcolorhex", settings.fontcolor);
        applyValue("glowradius", settings.glow || settings.glowradius);
        applyValue("borderradius", settings.radius || settings.borderradius);
        applyValue("pressscale", settings.pressscale);
        applyValue("animationspeed", settings.speed || settings.animationspeed);
        applyValue("fontfamily", settings.fontfamily);
        applyValue("hidemouse", settings.hidemouse);
        applyValue("hidescrollcombo", settings.hidescrollcombo);
        applyValue("boldfont", settings.boldfont);
        applyValue("outlinescalepressed", settings.outlinescalepressed ?? "2");
        applyValue("outlinescaleunpressed", settings.outlinescaleunpressed ?? "2");

        applyValue("customLayoutRow1", settings.customLayoutRow1 !== undefined ? settings.customLayoutRow1 : "");
        applyValue("customLayoutRow2", settings.customLayoutRow2 !== undefined ? settings.customLayoutRow2 : "");
        applyValue("customLayoutRow3", settings.customLayoutRow3 !== undefined ? settings.customLayoutRow3 : "");
        applyValue("customLayoutRow4", settings.customLayoutRow4 !== undefined ? settings.customLayoutRow4 : "");
        applyValue("customLayoutRow5", settings.customLayoutRow5 !== undefined ? settings.customLayoutRow5 : "");
        applyValue("customLayoutMouse", settings.customLayoutMouse !== undefined ? settings.customLayoutMouse : "");

        applyValue("gapmodifier", settings.gapmodifier);
        applyValue("keylegendmode", settings.keylegendmode);
        applyValue("forcedisableanalog", settings.forcedisableanalog);
    }

    updateState(settings = null) {
        if (!settings) settings = this.getCurrentSettings();

        this.visualizer.applyStyles(settings, true);

        clearTimeout(this.rebuildDebounceTimer);
        this.rebuildDebounceTimer = setTimeout(() => {
            this.visualizer.rebuildInterface(settings);
        }, 100);

        clearTimeout(this.urlDebounceTimer);
        this.urlDebounceTimer = setTimeout(() => {
            this.updateGeneratedLink(settings);
        }, 250);
    }

    updateGeneratedLink(settings) {
        const paramsString = this.urlManager.buildURLParams(settings);
        const baseURL = `${window.location.origin}${window.location.pathname}`;

        const wsParam = `ws=${settings.wsaddress || "localhost"}:${settings.wsport || "16899"}`;
        const linkInput = document.getElementById("generatedlink");

        const compressed = this.urlManager.compressSettings(paramsString);
        if (compressed) {
            const url = baseURL + `?cfg=${compressed}`;
            window.history.replaceState({}, "", `${url}`);
            linkInput.value = `${url}&${wsParam}`;
            console.clear();
            console.log(`compressed link: ${url}`);
            console.log(`uncompressed link: ${baseURL}?${paramsString}`);
        } else {
            window.history.replaceState({}, "", `${baseURL}?${paramsString}`);
            linkInput.value = `${baseURL}?${paramsString}&${wsParam}`;
        }

        const container = linkInput.closest(".link-container") || document.querySelector(".link-container");
        container.classList.add("hint");
        setTimeout(() => container.classList.remove("hint"), 1000);
    }

    loadSettingsFromLink(fromCurrentUrl = false) {
        const linkInput = document.getElementById("generatedlink");
        const loadBtn = document.getElementById("loadbtn");

        let urlString = fromCurrentUrl === true
            ? window.location.href
            : linkInput.value;

        if (!urlString || urlString.trim() === "") {
            loadBtn.textContent = "empty";
            loadBtn.classList.add("copied");
            setTimeout(() => {
                loadBtn.textContent = "⟳ load url";
                loadBtn.classList.remove("copied");
            }, 2000);
            return;
        }

        if (!urlString.startsWith("http")) {
            urlString = window.location.origin + urlString;
        }

        try {
            const url = new URL(urlString);
            const params = url.searchParams;
            const settings = {};

            let wsAddress = "localhost";
            let wsPort = "16899";
            if (params.has("ws")) {
                const wsConfig = params.get("ws").split(":");
                wsAddress = wsConfig[0] || "localhost";
                wsPort = wsConfig[1] || "16899";
            }

            if (params.has("cfg")) {
                const compressed = params.get("cfg");
                const decompressed = this.urlManager.decompressSettings(compressed);

                if (decompressed) {
                    const decompressedParams = new URLSearchParams(decompressed);
                    for (const key of decompressedParams.keys()) {
                        let value = decompressedParams.get(key);
                        if (key !== "ws" && value !== null && value !== "") {
                            if (key.includes("color")) {
                                value = this.utils.normalizeColorValue(value);
                            }
                            settings[key] = value;
                        }
                    }
                } else {
                    loadBtn.textContent = "decompress error";
                    loadBtn.classList.add("copied");
                    setTimeout(() => {
                        loadBtn.textContent = "⟳ load url";
                        loadBtn.classList.remove("copied");
                    }, 2000);
                    return;
                }
            } else {
                for (const key of params.keys()) {
                    let value = params.get(key);
                    if (key !== "ws" && value !== null && value !== "") {
                        if (key.includes("color")) {
                            value = this.utils.normalizeColorValue(value);
                        }
                        settings[key] = value;
                    }
                }
            }

            settings.wsaddress = wsAddress;
            settings.wsport = wsPort;

            if (!settings.keylegendmode) {
                settings.keylegendmode = "fading";
            }
            if (settings.forcedisableanalog === undefined || settings.forcedisableanalog === null) {
                settings.forcedisableanalog = false;
            }

            if (Object.keys(settings).length > 0) {
                this.applySettings(settings);
                this.updateState();

                loadBtn.textContent = "loaded";
                loadBtn.classList.add("copied");
                setTimeout(() => {
                    loadBtn.textContent = "⟳ load url";
                    loadBtn.classList.remove("copied");
                }, 2000);
            } else {
                loadBtn.textContent = "no params";
                loadBtn.classList.add("copied");
                setTimeout(() => {
                    loadBtn.textContent = "⟳ load url";
                    loadBtn.classList.remove("copied");
                }, 2000);
            }
        } catch (e) {
            loadBtn.textContent = "error";
            loadBtn.classList.add("copied");
            setTimeout(() => {
                loadBtn.textContent = "⟳ load url";
                loadBtn.classList.remove("copied");
            }, 2000);
        }
    }

    initPickrColorInput(pickrId, defaultColor) {
        const pickrEl = document.getElementById(pickrId);
        const hexInput = document.getElementById(pickrId + "hex");

        if (!pickrEl || !hexInput) return;

        const pickr = Pickr.create({
            el: pickrEl,
            theme: "classic",
            default: hexInput.value || defaultColor,
            components: {
                preview: true,
                opacity: true,
                hue: true,
                interaction: {
                    hex: true,
                    rgba: true,
                    hsva: true,
                    input: true,
                    clear: false,
                    save: true
                }
            },
            strings: {
                save: "Apply"
            },
            swatches: []
        });

        this.pickrInstances[pickrId] = pickr;

        pickr.on("change", (color) => {
            const hexA = color.toHEXA().toString();
            hexInput.value = hexA.toLowerCase();
            pickr.applyColor();
            this.updateState();
        });

        hexInput.addEventListener("input", (e) => {
            let val = e.target.value.toLowerCase().replace(/[^0-9a-f#]/g, "");
            if (!val.startsWith("#")) val = "#" + val;
            if (val.length > 9) val = val.substring(0, 9);
            e.target.value = val;

            if (val.length === 7 || val.length === 9) {
                try {
                    pickr.setColor(val, true);
                } catch (error) {
                }
                this.updateState();
            }
        });

        try {
            pickr.setColor(hexInput.value || defaultColor, true);
        } catch (error) {
        }
    }

    setupConfigInputs() {
        const inputs = document.querySelectorAll(".config-input");
        inputs.forEach(input => {
            input.addEventListener("input", () => {
                if (input.type === "range")
                    this.updateSliderLabel(input);
                else if (input.classList.contains("color-hex-input"))
                    return;

                this.updateState();
            });
        });

        document.getElementById("copybtn").addEventListener("click", this.copyLink.bind(this));
        document.getElementById("loadbtn").addEventListener("click", this.loadSettingsFromLink.bind(this));

        const layoutPresets = document.getElementById("layoutPresets");
        if (layoutPresets) {
            layoutPresets.addEventListener("change", (e) => {
                const presetUrl = e.target.value;
                if (presetUrl) {
                    const linkInput = document.getElementById("generatedlink");
                    linkInput.value = presetUrl;
                    this.loadSettingsFromLink(false);
                    setTimeout(() => {
                        e.target.selectedIndex = 0;
                    }, 100);
                }
            });
        }
    }

    setupPreviewInputListeners() {
        document.addEventListener("keydown", e => this.handlePreviewInput(e, "key_pressed"), { capture: true });
        document.addEventListener("keyup", e => this.handlePreviewInput(e, "key_released"), { capture: true });
        document.addEventListener("mousedown", e => this.handlePreviewInput(e, "mouse_pressed"));
        document.addEventListener("mouseup", e => this.handlePreviewInput(e, "mouse_released"));
        document.addEventListener("wheel", e => this.handlePreviewInput(e, "mouse_wheel"), { passive: true });
    }

    handlePreviewInput(event, type) {
        if (!this.visualizer.previewElements) return;

        const isTypingField = event.target.matches("input[type='text'], input[type='number'], textarea, .color-hex-input");

        if (type === "key_pressed" || type === "key_released") {
            let keyName = BROWSER_CODE_TO_KEY_NAME[event.code.toLowerCase()];
            let elements = this.visualizer.previewElements.keyElements.get(keyName);

            if (!elements && event.key) {
                const keyLabel = event.key.toUpperCase();
                for (const [key, els] of this.visualizer.previewElements.keyElements.entries()) {
                    if (els.some(el => el.textContent === keyLabel)) {
                        keyName = key;
                        elements = els;
                        break;
                    }
                }
            }

            if (elements && elements.length > 0) {
                elements.forEach(el => {
                    this.visualizer.updateElementState(el, keyName, type === "key_pressed", this.visualizer.activeKeys);
                });

                if (!isTypingField) {
                    event.preventDefault();
                } else if (keyName === "key_tab" || keyName === "key_escape") {
                    event.preventDefault();
                }
            }
        } else if (type === "mouse_pressed" || type === "mouse_released") {
            const btnName = BROWSER_BUTTON_TO_KEY_NAME[event.button];
            if (btnName) {
                const elements = this.visualizer.previewElements.mouseElements.get(btnName);
                if (elements && elements.length > 0) {
                    elements.forEach(el => {
                        this.visualizer.updateElementState(el, btnName, type === "mouse_pressed", this.visualizer.activeMouseButtons);
                    });
                }
            }
        } else if (type === "mouse_wheel") {
            const dir = Math.sign(event.deltaY);
            if (this.visualizer.previewElements.scrollDisplays && this.visualizer.previewElements.scrollDisplays.length > 0) {
                this.visualizer.handleScroll(dir);
            }
        }
    }

    setupBackgroundVideo() {
        const video = document.getElementById("bgvideo");
        const source = document.getElementById("bgsource");

        if (video && source) {
            const randomIndex = Math.floor(Math.random() * 2) + 1;
            source.src = `./media/preview_gameplay${randomIndex}.mp4`;
            video.load();
            video.play();
        }
    }

    setupCheatSheetToggle() {
        const allDetails = document.querySelectorAll(".fullscreen-details");

        allDetails.forEach(detailsTag => {
            const closeBtn = detailsTag.querySelector(".close-btn");
            if (!closeBtn) return;

            closeBtn.addEventListener("click", e => {
                e.preventDefault();
                detailsTag.open = false;
            });

            const updateCloseButtonVisibility = () => {
                closeBtn.style.display = detailsTag.open ? "block" : "none";
            };

            updateCloseButtonVisibility();
            detailsTag.addEventListener("toggle", updateCloseButtonVisibility);
        });
    }

    async copyLink() {
        const linkInput = document.getElementById("generatedlink");
        const copyBtn = document.getElementById("copybtn");
        try {
            await navigator.clipboard.writeText(linkInput.value);
            copyBtn.textContent = "copied";
            copyBtn.classList.add("copied");
            setTimeout(() => {
                copyBtn.textContent = "⎘ copy url";
                copyBtn.classList.remove("copied");
            }, 2000);
        } catch (err) {
            linkInput.select();
            document.execCommand("copy");
        }
    }

    setupAnalogSense() {
        if (typeof window.analogsense === "undefined") return;

        const btn = document.getElementById("analogconnectbtn");
        const statusEl = document.getElementById("analogstatus");
        if (!btn || !statusEl) return;

        this.analogSenseActiveKeys = new Set();
        this.analogSensePrevDepths = {};

        const DIGITAL_THRESHOLD = 0.5;

        const handleAnalogReport = (activeKeys) => {
            if (!this.visualizer.previewElements) return;
            if (document.getElementById("forcedisableanalog")?.checked) return;

            const forcedisable = document.getElementById("forcedisableanalog")?.checked;
            if (forcedisable) return;

            const currentScancodes = new Set(activeKeys.map(k => String(k.scancode)));

            activeKeys.forEach(({ scancode, value }) => {
                const keyName = HID_TO_KEY_NAME[scancode];
                if (!keyName) return;

                if (!this.visualizer.forceDisableAnalog) {
                    if (!this.visualizer.analogMode) {
                        this.visualizer.analogMode = true;
                        this.visualizer.applyStyles(this.getCurrentSettings(), true);
                    }
                    this.visualizer.setAnalogDepthTarget(keyName, value);
                }

                const wasAbove = (this.analogSensePrevDepths[scancode] ?? 0) >= DIGITAL_THRESHOLD;
                const isAbove = value >= DIGITAL_THRESHOLD;

                if (isAbove && !wasAbove) {
                    const elements = this.visualizer.previewElements.keyElements.get(keyName);
                    if (elements) {
                        elements.forEach(el => {
                            this.visualizer.updateElementState(el, keyName, true, this.visualizer.activeKeys);
                        });
                    }
                    this.analogSenseActiveKeys.add(scancode);
                } else if (!isAbove && wasAbove) {
                    const elements = this.visualizer.previewElements.keyElements.get(keyName);
                    if (elements) {
                        elements.forEach(el => {
                            this.visualizer.updateElementState(el, keyName, false, this.visualizer.activeKeys);
                        });
                    }
                    this.analogSenseActiveKeys.delete(scancode);
                }

                this.analogSensePrevDepths[scancode] = value;
            });

            this.analogSenseActiveKeys.forEach(scancode => {
                if (!currentScancodes.has(String(scancode))) {
                    const keyName = HID_TO_KEY_NAME[scancode];
                    if (keyName) {
                        if ((this.analogSensePrevDepths[scancode] ?? 0) >= DIGITAL_THRESHOLD) {
                            const elements = this.visualizer.previewElements.keyElements.get(keyName);
                            if (elements) {
                                elements.forEach(el => {
                                    this.visualizer.updateElementState(el, keyName, false, this.visualizer.activeKeys);
                                });
                            }
                        }
                        this.visualizer.setAnalogDepthTarget(keyName, 0);
                    }
                    delete this.analogSensePrevDepths[scancode];
                    this.analogSenseActiveKeys.delete(scancode);
                }
            });
        };

        const setConnectedState = (name) => {
            statusEl.textContent = `connected: ${name}`;
            statusEl.style.color = "#5cf67d";
            btn.textContent = "disconnect";
            btn.classList.add("connected");
        };

        const disconnectDevice = () => {
            if (this.analogSenseProvider) {
                this.analogSenseProvider.stopListening();
                this.analogSenseProvider = null;
            }

            this.analogSenseActiveKeys.clear();
            this.analogSensePrevDepths = {};

            if (this.visualizer.analogMode) {
                this.visualizer.analogMode = false;
                if (this.visualizer.analogRafId) {
                    cancelAnimationFrame(this.visualizer.analogRafId);
                    this.visualizer.analogRafId = null;
                }
                this.visualizer.analogTargetDepths = {};
                this.visualizer.analogCurrentDepths = {};

                if (this.visualizer.previewElements) {
                    this.visualizer.previewElements.keyElements.forEach((elements, keyName) => {
                        elements.forEach(el => {
                            if (el.classList.contains("active") || el.classList.contains("analog-key")) {
                                this.visualizer.updateElementState(el, keyName, false, this.visualizer.activeKeys);
                                el.classList.remove("analog-key");
                                el.style.removeProperty("--analog-depth");
                                const primaryLabel = el.querySelector(".key-label-primary");
                                if (primaryLabel) primaryLabel.style.removeProperty("color");
                                const invertedLabel = el.querySelector(".key-label-inverted");
                                if (invertedLabel) invertedLabel.style.clipPath = "inset(100% 0 0 0)";
                            }
                        });
                    });
                }

                this.visualizer.applyStyles(this.getCurrentSettings(), true);
            }

            statusEl.textContent = "disconnected";
            statusEl.style.color = "#808080";
            btn.textContent = "connect analog";
            btn.classList.remove("connected");
        };

        const connectDevice = async (provider) => {
            if (this.analogSenseProvider) {
                this.analogSenseProvider.stopListening();
            }
            this.analogSenseProvider = provider;
            provider.startListening(handleAnalogReport);
            const name = provider.getProductName();
            setConnectedState(name);
        };

        analogsense.getDevices().then(devices => {
            if (devices.length > 0) {
                connectDevice(devices[0]);
            }
        });

        btn.addEventListener("click", async () => {
            if (this.analogSenseProvider) {
                disconnectDevice();
                return;
            }

            try {
                const device = await analogsense.requestDevice();
                if (device) {
                    await connectDevice(device);
                } else {
                    statusEl.textContent = "no compatible keyboard found";
                    statusEl.style.color = "#f65c5c";
                }
            } catch (e) {
                if (e.name !== "SecurityError") {
                    statusEl.textContent = `error: ${e.message}`;
                    statusEl.style.color = "#f65c5c";
                }
            }
        });
    }

    setupKeyAddButtons() {
        const popup = document.getElementById("keyAddPopup");
        const keySelect = document.getElementById("popupKeySelect");
        const labelInput = document.getElementById("popupKeyLabel");
        const widthSlider = document.getElementById("popupWidthSlider");
        const widthValue = document.getElementById("popupWidthValue");
        const addBtn = document.getElementById("popupAddBtn");
        const cancelBtn = document.getElementById("popupCancelBtn");
        const scrollerLabels = document.getElementById("popupScrollerLabels");
        const mouseSideLabels = document.getElementById("popupMouseSideLabels");

        let currentTargetRow = null;
        let originalValue = "";
        let isUpdating = false;

        const updateKeyString = () => {
            if (isUpdating) return;

            const keyName = keySelect.value;
            let keyString = "";

            if (keyName === "scroller") {
                const defaultLabel = document.getElementById("popupScrollerDefault").value || "M3";
                const upLabel = document.getElementById("popupScrollerUp").value || "🡅";
                const downLabel = document.getElementById("popupScrollerDown").value || "🡇";
                const widthClass = this.getWidthClass(parseInt(widthSlider.value));
                keyString = widthClass ?
                    `scroller:"${defaultLabel}":"${upLabel}":"${downLabel}":${widthClass}` :
                    `scroller:"${defaultLabel}":"${upLabel}":"${downLabel}"`;
            } else if (keyName === "mouse_side") {
                const m5Label = document.getElementById("popupMouseSideM5").value || "M5";
                const m4Label = document.getElementById("popupMouseSideM4").value || "M4";
                const widthClass = this.getWidthClass(parseInt(widthSlider.value));
                keyString = widthClass ?
                    `mouse_side:"${m5Label}":"${m4Label}":${widthClass}` :
                    `mouse_side:"${m5Label}":"${m4Label}"`;
            } else if (keyName === "invisible") {
                const widthClass = this.getWidthClass(parseInt(widthSlider.value));
                keyString = widthClass ? `$none:"invis":${widthClass}` : keyName;
            } else if (keyName === "dummy") {
                keyString = "dummy";
            } else {
                const label = labelInput.value || keyName.split("_")[1].toUpperCase();
                const widthClass = this.getWidthClass(parseInt(widthSlider.value));
                keyString = widthClass ?
                    `${keyName}:"${label}":${widthClass}` :
                    `${keyName}:"${label}"`;
            }

            const targetInput = document.getElementById(`customLayout${currentTargetRow}`);
            if (targetInput) {
                targetInput.value = originalValue ? `${originalValue}, ${keyString}` : keyString;
                targetInput.dispatchEvent(new Event("input", { bubbles: true }));
            }
        };

        widthSlider.addEventListener("input", () => {
            const val = parseInt(widthSlider.value);
            const units = (val / 100).toFixed(2);
            widthValue.textContent = `${units}u`;
            updateKeyString();
        });

        keySelect.addEventListener("change", () => {
            const selectedKey = keySelect.value;

            scrollerLabels.style.display = "none";
            mouseSideLabels.style.display = "none";
            labelInput.parentElement.style.display = "block";

            if (selectedKey === "scroller") {
                labelInput.parentElement.style.display = "none";
                scrollerLabels.style.display = "block";
                document.getElementById("popupScrollerDefault").value = "M3";
                document.getElementById("popupScrollerUp").value = "🡅";
                document.getElementById("popupScrollerDown").value = "🡇";
            } else if (selectedKey === "mouse_side") {
                labelInput.parentElement.style.display = "none";
                mouseSideLabels.style.display = "block";
                document.getElementById("popupMouseSideM5").value = "M5";
                document.getElementById("popupMouseSideM4").value = "M4";
            } else if (selectedKey === "invisible" || selectedKey === "dummy") {
                labelInput.value = "invisible";
            } else {
                const optionText = keySelect.options[keySelect.selectedIndex].text;
                labelInput.value = optionText;
            }

            updateKeyString();
        });

        labelInput.addEventListener("input", updateKeyString);

        document.getElementById("popupScrollerDefault").addEventListener("input", updateKeyString);
        document.getElementById("popupScrollerUp").addEventListener("input", updateKeyString);
        document.getElementById("popupScrollerDown").addEventListener("input", updateKeyString);
        document.getElementById("popupMouseSideM5").addEventListener("input", updateKeyString);
        document.getElementById("popupMouseSideM4").addEventListener("input", updateKeyString);

        const buttonMappings = [
            { buttonId: "addKey1", rowId: "Row1" },
            { buttonId: "addKey2", rowId: "Row2" },
            { buttonId: "addKey3", rowId: "Row3" },
            { buttonId: "addKey4", rowId: "Row4" },
            { buttonId: "addKey5", rowId: "Row5" },
            { buttonId: "addKeyMouse", rowId: "Mouse" }
        ];

        buttonMappings.forEach(({ buttonId, rowId }) => {
            const btn = document.getElementById(buttonId);
            if (btn) {
                btn.addEventListener("click", (e) => {
                    isUpdating = false;
                    currentTargetRow = rowId;

                    const targetInput = document.getElementById(`customLayout${rowId}`);
                    originalValue = targetInput ? targetInput.value.trim() : "";

                    const rect = btn.getBoundingClientRect();
                    const popupWidth = 340;
                    const popupHeight = 400;

                    let left = rect.left - popupWidth;
                    let top = rect.top;

                    if (left < 10) left = rect.right + 10;
                    if (left + popupWidth > window.innerWidth - 10) left = Math.max(10, (window.innerWidth - popupWidth) / 2);
                    if (top + popupHeight > window.innerHeight - 10) top = Math.max(10, window.innerHeight - popupHeight - 10);
                    if (top < 10) top = 10;

                    popup.style.display = "block";
                    popup.style.left = `${left}px`;
                    popup.style.top = `${top}px`;

                    keySelect.value = "key_a";
                    labelInput.value = "A";
                    widthSlider.value = 100;
                    widthValue.textContent = "1.00u";
                    scrollerLabels.style.display = "none";
                    mouseSideLabels.style.display = "none";
                    labelInput.parentElement.style.display = "block";

                    updateKeyString();
                });
            }
        });

        addBtn.addEventListener("click", () => {
            popup.style.display = "none";
        });

        cancelBtn.addEventListener("click", () => {
            isUpdating = true;
            const targetInput = document.getElementById(`customLayout${currentTargetRow}`);
            if (targetInput) {
                targetInput.value = originalValue;
                targetInput.dispatchEvent(new Event("input", { bubbles: true }));
            }
            popup.style.display = "none";
        });

        popup.addEventListener("click", (e) => {
            if (e.target === popup) {
                isUpdating = true;
                const targetInput = document.getElementById(`customLayout${currentTargetRow}`);
                if (targetInput) {
                    targetInput.value = originalValue;
                    targetInput.dispatchEvent(new Event("input", { bubbles: true }));
                }
                popup.style.display = "none";
            }
        });
    }

    getWidthClass(value) {
        if (value === 100) return "";
        const units = value / 100;
        const intPart = Math.floor(units);
        let decStr = Math.round((units - intPart) * 100).toString().padStart(2, "0");
        if (decStr.endsWith("0")) decStr = decStr.slice(0, -1);
        if (decStr === "" || decStr === "0") return `u${intPart}`;
        return `u${intPart}-${decStr}`;
    }
}