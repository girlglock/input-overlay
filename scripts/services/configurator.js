//guh
import {BROWSER_BUTTON_TO_KEY_NAME, BROWSER_CODE_TO_KEY_NAME, COLOR_PICKERS, DEFAULT_LAYOUT_STRINGS} from "../consts.js";

export class ConfiguratorMode {
    constructor(utils, urlManager, layoutParser, visualizer) {
        this.utils = utils;
        this.urlManager = urlManager;
        this.visualizer = visualizer;
        this.pickrInstances = {};

        document.getElementById("configurator").style.display = "block";
        document.getElementById("overlay").classList.remove("show");

        this.initDefaultLayoutValues();
        this.setupBackgroundVideo();
        this.setupCheatSheetToggle();

        COLOR_PICKERS.forEach(cp => {
            this.initPickrColorInput(cp.id, cp.defaultColor);
        });

        this.loadSettingsFromLink(true);
        this.setupConfigInputs();
        this.setupPreviewInputListeners();
        this.updateState();
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
            scale: document.getElementById("scale").value,
            opacity: document.getElementById("opacity").value,
            fontfamily: document.getElementById("fontfamily").value,
            hidemouse: document.getElementById("hidemouse").checked,
            hidescrollcombo: document.getElementById("hidescrollcombo").checked,
            boldfont: document.getElementById("boldfont") ? document.getElementById("boldfont").checked : false,

            gapmodifier: document.getElementById("gapmodifier") ? document.getElementById("gapmodifier").value : "100",

            customLayoutRow1: document.getElementById("customLayoutRow1") ? document.getElementById("customLayoutRow1").value : "",
            customLayoutRow2: document.getElementById("customLayoutRow2") ? document.getElementById("customLayoutRow2").value : "",
            customLayoutRow3: document.getElementById("customLayoutRow3") ? document.getElementById("customLayoutRow3").value : "",
            customLayoutRow4: document.getElementById("customLayoutRow4") ? document.getElementById("customLayoutRow4").value : "",
            customLayoutRow5: document.getElementById("customLayoutRow5") ? document.getElementById("customLayoutRow5").value : "",
            customLayoutMouse: document.getElementById("customLayoutMouse") ? document.getElementById("customLayoutMouse").value : "",
        };
    }

    updateSliderLabel(input) {
        const label = document.getElementById(input.id + "value");
        if (label) {
            let suffix = "";
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
            if (el && value !== undefined) {
                if (el.type === "checkbox") {
                    el.checked = value === "true" || value === "1" || value === true;
                } else {
                    el.value = value;
                    if (id.includes("colorhex")) {
                        const pickrId = id.replace("hex", "");
                        const pickr = this.pickrInstances[pickrId];
                        if (pickr) {
                            try {
                                pickr.setColor(value, true);
                            } catch (error) {
                            }
                        }
                    }
                }
                if (el.type === "range") {
                    this.updateSliderLabel(el);
                    el.dispatchEvent(new Event("input", {bubbles: true}));
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
        applyValue("scale", settings.scale);
        applyValue("opacity", settings.opacity);
        applyValue("fontfamily", settings.fontfamily);
        applyValue("hidemouse", settings.hidemouse);
        applyValue("hidescrollcombo", settings.hidescrollcombo);
        applyValue("boldfont", settings.boldfont);
        applyValue("customLayoutRow1", settings.customLayoutRow1);
        applyValue("customLayoutRow2", settings.customLayoutRow2);
        applyValue("customLayoutRow3", settings.customLayoutRow3);
        applyValue("customLayoutRow4", settings.customLayoutRow4);
        applyValue("customLayoutRow5", settings.customLayoutRow5);
        applyValue("customLayoutMouse", settings.customLayoutMouse);

        applyValue("gapmodifier", settings.gapmodifier);
    }

    updateState(settings = null) {
        if (!settings) settings = this.getCurrentSettings();

        this.visualizer.applyStyles(settings);
        this.visualizer.rebuildInterface(settings);
        this.visualizer.applyContainerTransformations(settings);
        this.updateGeneratedLink(settings);
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
                if (input.type === "range") {
                    this.updateSliderLabel(input);
                } else if (input.classList.contains("color-hex-input")) {
                    return;
                }
                this.updateState();
            });
        });
        document.getElementById("copybtn").addEventListener("click", this.copyLink.bind(this));
        document.getElementById("loadbtn").addEventListener("click", this.loadSettingsFromLink.bind(this));
    }

    setupPreviewInputListeners() {
        document.addEventListener("keydown", e => this.handlePreviewInput(e, "key_pressed"), {capture: true});
        document.addEventListener("keyup", e => this.handlePreviewInput(e, "key_released"), {capture: true});
        document.addEventListener("mousedown", e => this.handlePreviewInput(e, "mouse_pressed"));
        document.addEventListener("mouseup", e => this.handlePreviewInput(e, "mouse_released"));
        document.addEventListener("wheel", e => this.handlePreviewInput(e, "mouse_wheel"), {passive: true});
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
}