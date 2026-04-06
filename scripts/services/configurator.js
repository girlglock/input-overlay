//guh
import { BROWSER_BUTTON_TO_KEY_NAME, BROWSER_CODE_TO_KEY_NAME, COLOR_PICKERS, DEFAULT_LAYOUT_STRINGS, HID_TO_KEY_NAME } from "../consts.js";
import { GamepadManager } from "./gamepadManager.js";

function flashBtn(btn, label, original, ms = 2000) {
    btn.textContent = label;
    btn.classList.add("copied");
    setTimeout(() => { btn.textContent = original; btn.classList.remove("copied"); }, ms);
}

export class ConfiguratorMode {
    constructor(utils, urlManager, layoutParser, visualizer) {
        this.utils = utils;
        this.urlManager = urlManager;
        this.visualizer = visualizer;
        this.pickrInstances = {};
        this.urlDebounceTimer = null;
        this.rebuildThrottleTimer = null;
        this.rebuildLastFired = 0;
        this.rebuildPending = null;

        document.getElementById("configurator").style.display = "flex";
        document.getElementById("overlay").classList.remove("show");

        this.initDefaultLayoutValues();
        this.setupBackgroundVideo();
        this.setupCheatSheetToggle();

        setTimeout(() => COLOR_PICKERS.forEach(cp => this.initPickrColorInput(cp.id, cp.defaultColor)), 25);

        const urlParams = new URLSearchParams(window.location.search);
        const hasParams = urlParams.has("cfg") || Array.from(urlParams.keys()).some(k => k !== "ws");

        if (hasParams) this.loadSettingsFromLink(true);
        else this.applyDefaultSettings();

        this.setupConfigInputs();
        this.setupKeyAddButtons();
        this.setupPreviewInputListeners();
        this.setupAnalogSense();
        this.updateState();

        //tiny delay for gamepads because im lazy
        setTimeout(() => {
            this.gamepadManager = new GamepadManager(this.visualizer);
        }, 100);
    }

    applyDefaultSettings() {
        this.applySettings({
            wsaddress: "localhost", wsport: "4455",
            activecolor: "#5cf67d", inactivecolor: "#808080",
            backgroundcolor: "#1a1a1ad1", activebgcolor: "#47bd61",
            outlinecolor: "#4f4f4f", fontcolor: "#ffffff",
            glowradius: "24", borderradius: "1",
            pressscale: "110", animationspeed: "300",
            fontfamily: "ArialPixel",
            hidemouse: false, hidescrollcombo: false, boldfont: true,
            analogmode: false, gapmodifier: "100",
            outlinescalepressed: "2", outlinescaleunpressed: "2",
            customLayoutRow1: DEFAULT_LAYOUT_STRINGS.row1,
            customLayoutRow2: DEFAULT_LAYOUT_STRINGS.row2,
            customLayoutRow3: DEFAULT_LAYOUT_STRINGS.row3,
            customLayoutRow4: DEFAULT_LAYOUT_STRINGS.row4,
            customLayoutRow5: DEFAULT_LAYOUT_STRINGS.row5,
            customLayoutMouse: DEFAULT_LAYOUT_STRINGS.mouse,
            keylegendmode: "fading", forcedisableanalog: false,
            mousetrailsensitivity: "100",
            mousetrailfadeout: "600",
            mousetrailmode: "wrap",
            mousetraillength: "150",
            mousetrailm1highlight: false,
            mousepadtexture: "",
            showmousedistance: false,
            mousedistancedpi: "400",
        });
    }

    initDefaultLayoutValues() {
        const rowIds = ["customLayoutRow1", "customLayoutRow2", "customLayoutRow3", "customLayoutRow4", "customLayoutRow5", "customLayoutMouse"];
        const keys = ["row1", "row2", "row3", "row4", "row5", "mouse"];
        rowIds.forEach((id, i) => {
            const el = document.getElementById(id);
            if (el && !el.value) el.value = DEFAULT_LAYOUT_STRINGS[keys[i]];
        });
    }

    getCurrentSettings() {
        const get = (id) => document.getElementById(id);
        const val = (id) => get(id)?.value ?? "";
        const chk = (id) => get(id)?.checked ?? false;

        return {
            wsaddress: val("wsaddress") || "localhost",
            wsport: val("wsport") || "4455",
            wsauth: val("wsauth"),
            activecolor: val("activecolorhex"),
            inactivecolor: val("inactivecolorhex"),
            backgroundcolor: val("backgroundcolorhex"),
            activebgcolor: val("activebgcolorhex"),
            outlinecolor: val("outlinecolorhex"),
            fontcolor: val("fontcolorhex"),
            glowradius: val("glowradius"),
            borderradius: val("borderradius"),
            pressscale: val("pressscale"),
            animationspeed: val("animationspeed"),
            fontfamily: val("fontfamily"),
            hidemouse: chk("hidemouse"),
            hidescrollcombo: chk("hidescrollcombo"),
            boldfont: chk("boldfont"),
            analogmode: chk("analogmode"),
            gapmodifier: val("gapmodifier") || "100",
            outlinescalepressed: val("outlinescalepressed") || "2",
            outlinescaleunpressed: val("outlinescaleunpressed") || "2",
            customLayoutRow1: val("customLayoutRow1"),
            customLayoutRow2: val("customLayoutRow2"),
            customLayoutRow3: val("customLayoutRow3"),
            customLayoutRow4: val("customLayoutRow4"),
            customLayoutRow5: val("customLayoutRow5"),
            customLayoutMouse: val("customLayoutMouse"),
            keylegendmode: val("keylegendmode") || "inverting",
            forcedisableanalog: chk("forcedisableanalog"),
            mousetrailsensitivity: val("mousetrailsensitivity") || "100",
            mousetrailfadeout: val("mousetrailfadeout") !== "" ? val("mousetrailfadeout") : "600",
            mousetrailmode: val("mousetrailmode") || "wrap",
            mousetraillength: val("mousetraillength") || "150",
            mousetrailm1highlight: chk("mousetrailm1highlight"),
            mousepadtexture: val("mousepadtexture"),
            showmousedistance: chk("showmousedistance"),
            mousedistancedpi: val("mousedistancedpi") || "400",
        };
    }

    updateSliderLabel(input) {
        const label = document.getElementById(input.id + "value");
        if (!label) return;

        const id = input.id;
        if (id === "outlinescalepressed" || id === "outlinescaleunpressed") {
            label.textContent = input.value + "px"; return;
        }
        if (id === "mousetrailsensitivity") {
            label.textContent = (input.value / 100).toFixed(1) + "x"; return;
        }
        if (id === "mousetrailfadeout") {
            label.textContent = input.value + "ms"; return;
        }
        if (id === "mousetraillength") {
            label.textContent = input.value + "pts"; return;
        }
        if (id === "mousedistancedpi") {
            label.textContent = input.value + " DPI"; return;
        }

        let suffix = "", val = input.value;
        if (id.includes("radius")) suffix = "px";
        else if (id.includes("scale")) { suffix = "x"; val = id === "pressscale" ? (val / 100).toFixed(2) : (val / 100).toFixed(1); }
        else if (id === "opacity" || id.includes("speed") || id.includes("modifier")) suffix = "%";

        label.textContent = val + suffix;
    }

    applySettings(settings) {
        if (!settings) return;

        const applyValue = (id, value) => {
            const el = document.getElementById(id);
            if (!el) return;

            if (el.type === "checkbox") {
                el.checked = value === "true" || value === "1" || value === true;
                el.dispatchEvent(new Event("change", { bubbles: true }));
            } else {
                el.value = value ?? "";

                if (id.includes("colorhex")) {
                    const pickr = this.pickrInstances[id.replace("hex", "")];
                    if (pickr && value) { try { pickr.setColor(value, true); } catch { /* ignore */ } }
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
        applyValue("customLayoutRow1", settings.customLayoutRow1 ?? "");
        applyValue("customLayoutRow2", settings.customLayoutRow2 ?? "");
        applyValue("customLayoutRow3", settings.customLayoutRow3 ?? "");
        applyValue("customLayoutRow4", settings.customLayoutRow4 ?? "");
        applyValue("customLayoutRow5", settings.customLayoutRow5 ?? "");
        applyValue("customLayoutMouse", settings.customLayoutMouse ?? "");
        applyValue("gapmodifier", settings.gapmodifier);
        applyValue("keylegendmode", settings.keylegendmode);
        applyValue("forcedisableanalog", settings.forcedisableanalog);
        applyValue("mousetrailsensitivity", settings.mousetrailsensitivity ?? "100");
        applyValue("mousetrailfadeout", settings.mousetrailfadeout ?? "600");
        applyValue("mousetrailmode", settings.mousetrailmode ?? "wrap");
        applyValue("mousetraillength", settings.mousetraillength ?? "150");
        applyValue("mousetrailm1highlight", settings.mousetrailm1highlight ?? false);
        applyValue("mousepadtexture", settings.mousepadtexture ?? "");
        applyValue("showmousedistance", settings.showmousedistance ?? false);
        applyValue("mousedistancedpi", settings.mousedistancedpi ?? "400");
    }

    updateState(settings = null) {
        if (!settings) settings = this.getCurrentSettings();
        this.visualizer.applyStyles(settings, true);

        const THROTTLE_MS = 100;
        const now = performance.now();
        this.rebuildPending = settings;

        clearTimeout(this.rebuildThrottleTimer);
        this.rebuildThrottleTimer = setTimeout(() => {
            this.visualizer.rebuildInterface(this.rebuildPending);
            this.rebuildLastFired = performance.now();
            this.rebuildPending = null;
        }, Math.max(0, THROTTLE_MS - (now - this.rebuildLastFired)));

        clearTimeout(this.urlDebounceTimer);
        this.urlDebounceTimer = setTimeout(() => this.updateGeneratedLink(settings), 250);
    }

    updateGeneratedLink(settings) {
        const paramsString = this.urlManager.buildURLParams(settings);
        const safeSettings = { ...settings, wsauth: "" };
        const safeParamsString = this.urlManager.buildURLParams(safeSettings);
        const base = `${window.location.origin}${window.location.pathname}`;
        const wsParam = `ws=${settings.wsaddress || "localhost"}:${settings.wsport || "4455"}`;
        const linkInput = document.getElementById("generatedlink");

        const compressed = this.urlManager.compressSettings(paramsString);
        const safeCompressed = this.urlManager.compressSettings(safeParamsString);
        if (compressed) {
            const safeUrl = `${base}?cfg=${safeCompressed}`;
            window.history.replaceState({}, "", safeUrl);
            linkInput.value = `${base}?cfg=${compressed}&${wsParam}`;
            console.clear();
            console.log(`compressed params: ${compressed}`);
            console.log(`uncompressed params: ${paramsString}`);
        } else {
            window.history.replaceState({}, "", `${base}?${safeParamsString}`);
            linkInput.value = `${base}?${paramsString}&${wsParam}`;
        }

        const container = linkInput.closest(".link-container") || document.querySelector(".link-container");
        container.classList.add("hint");
        setTimeout(() => container.classList.remove("hint"), 1000);

        const isNonLocal = (settings.wsaddress || "localhost") !== "localhost";
        const copyBtn = document.getElementById("copybtn");
        const downloadBtn = document.getElementById("downloadbtn");
        const authWarning = document.getElementById("authwarning");

        if (isNonLocal) {
            if (copyBtn) copyBtn.style.display = "none";
            if (downloadBtn) downloadBtn.style.display = "";
        } else {
            if (copyBtn) copyBtn.style.display = "";
            if (downloadBtn) downloadBtn.style.display = "none";
        }

        const hasAuth = !!(settings.wsauth && settings.wsauth.trim());
        if (authWarning) authWarning.style.display = hasAuth ? "" : "none";
    }

    loadSettingsFromLink(fromCurrentUrl = false) {
        const linkInput = document.getElementById("generatedlink");
        const loadBtn = document.getElementById("loadbtn");
        const flash = (msg) => flashBtn(loadBtn, msg, "⟳ load url");

        let urlString = fromCurrentUrl === true ? window.location.href : linkInput.value;
        if (!urlString?.trim()) { flash("empty"); return; }
        if (!urlString.startsWith("http")) urlString = window.location.origin + urlString;

        try {
            const url = new URL(urlString);
            const params = url.searchParams;
            const settings = {};

            let wsAddress = "localhost", wsPort = "4455";
            if (params.has("ws")) {
                const ws = params.get("ws").split(":");
                wsAddress = ws[0] || "localhost";
                wsPort = ws[1] || "4455";
            }

            const sourceParams = params.has("cfg")
                ? (() => {
                    const dec = this.urlManager.decompressSettings(params.get("cfg"));
                    if (!dec) { flash("decompress error"); return null; }
                    return new URLSearchParams(dec);
                })()
                : params;

            if (!sourceParams) return;

            for (const key of sourceParams.keys()) {
                if (key === "ws") continue;
                let value = sourceParams.get(key);
                if (value == null || value === "") continue;
                if (key.includes("color")) value = this.utils.normalizeColorValue(value);
                settings[key] = value;
            }

            settings.wsaddress = wsAddress;
            settings.wsport = wsPort;
            if (!settings.keylegendmode) settings.keylegendmode = "fading";
            if (settings.forcedisableanalog == null) settings.forcedisableanalog = false;

            if (!Object.keys(settings).length) { flash("no params"); return; }

            this.applySettings(settings);
            this.updateState();
            flash("loaded");
        } catch {
            flashBtn(loadBtn, "error", "⟳ load url");
        }
    }

    initPickrColorInput(pickrId, defaultColor) {
        const pickrEl = document.getElementById(pickrId);
        const hexInput = document.getElementById(pickrId + "hex");
        if (!pickrEl || !hexInput) return;

        const pickr = Pickr.create({
            el: pickrEl, theme: "classic",
            default: hexInput.value || defaultColor,
            components: {
                preview: true, opacity: true, hue: true,
                interaction: { hex: true, rgba: true, hsva: true, input: true, clear: false, save: true }
            },
            strings: { save: "Apply" },
            swatches: []
        });

        this.pickrInstances[pickrId] = pickr;

        pickr.on("change", (color) => {
            hexInput.value = color.toHEXA().toString().toLowerCase();
            pickr.applyColor();
            this.updateState();
        });

        hexInput.addEventListener("input", (e) => {
            let val = e.target.value.toLowerCase().replace(/[^0-9a-f#]/g, "");
            if (!val.startsWith("#")) val = "#" + val;
            if (val.length > 9) val = val.slice(0, 9);
            e.target.value = val;
            if (val.length === 7 || val.length === 9) {
                try { pickr.setColor(val, true); } catch { /* ignore */ }
                this.updateState();
            }
        });

        try { pickr.setColor(hexInput.value || defaultColor, true); } catch { /* ignore */ }
    }

    setupConfigInputs() {
        for (const input of document.querySelectorAll(".config-input")) {
            input.addEventListener("input", () => {
                if (input.type === "range") this.updateSliderLabel(input);
                else if (input.classList.contains("color-hex-input")) return;
                this.updateState();
            });
        }

        const wsauthEl = document.getElementById("wsauth");
        const savedAuth = localStorage.getItem("overlay_wsauth");
        if (savedAuth && !wsauthEl.value) wsauthEl.value = savedAuth;
        wsauthEl.addEventListener("input", () => localStorage.setItem("overlay_wsauth", wsauthEl.value));

        const distanceCheckbox = document.getElementById("showmousedistance");
        const dpiSlider = document.getElementById("mousedistancedpi");
        const dpiLabel = document.getElementById("mousedistancedpivalue");
        const syncDpiState = () => {
            const enabled = distanceCheckbox?.checked ?? false;
            if (dpiSlider) { dpiSlider.disabled = !enabled; dpiSlider.style.opacity = enabled ? "1" : "0.5"; }
            if (dpiLabel) dpiLabel.style.opacity = enabled ? "1" : "0.4";
        };
        distanceCheckbox?.addEventListener("change", syncDpiState);
        syncDpiState();

        document.getElementById("copybtn").addEventListener("click", this.copyLink.bind(this));
        document.getElementById("copysharebtn").addEventListener("click", this.copyShareLink.bind(this));
        document.getElementById("loadbtn").addEventListener("click", this.loadSettingsFromLink.bind(this));
        document.getElementById("downloadbtn")?.addEventListener("click", (e) => {
            e.preventDefault();
            this.downloadOverlayHTML();
        });

        document.getElementById("layoutPresets")?.addEventListener("change", (e) => {
            const presetUrl = e.target.value;
            if (presetUrl) {
                document.getElementById("generatedlink").value = presetUrl;
                this.loadSettingsFromLink(false);
                setTimeout(() => { e.target.selectedIndex = 0; }, 100);
            }
        });

        document.getElementById("download-local-tip-btn")?.addEventListener("click", (e) => {
            e.preventDefault();
            this.downloadOverlayHTML();
        });
    }

    setupPreviewInputListeners() {
        document.addEventListener("keydown", e => this.handlePreviewInput(e, "key_pressed"), { capture: true });
        document.addEventListener("keyup", e => this.handlePreviewInput(e, "key_released"), { capture: true });
        document.addEventListener("mousedown", e => this.handlePreviewInput(e, "mouse_pressed"));
        document.addEventListener("mouseup", e => this.handlePreviewInput(e, "mouse_released"));
        document.addEventListener("wheel", e => this.handlePreviewInput(e, "mouse_wheel"), { passive: true });
        document.addEventListener("mousemove", e => this.handlePreviewInput(e, "mouse_moved"));
    }

    handlePreviewInput(event, type) {
        const els = this.visualizer.previewElements;
        if (!els) return;

        if (type === "key_pressed" || type === "key_released") {
            const isTyping = event.target.matches("input[type='text'], input[type='number'], textarea, .color-hex-input");
            let keyName = BROWSER_CODE_TO_KEY_NAME[event.code.toLowerCase()];
            let elements = els.keyElements.get(keyName);

            if (!elements && event.key) {
                const label = event.key.toUpperCase();
                for (const [k, elList] of els.keyElements) {
                    if (elList.some(el => el.textContent === label)) { keyName = k; elements = elList; break; }
                }
            }

            if (elements?.length) {
                const isPress = type === "key_pressed";
                for (const el of elements) this.visualizer.updateElementState(el, keyName, isPress, this.visualizer.activeKeys);
                if (!isTyping || keyName === "key_tab" || keyName === "key_escape") event.preventDefault();
            }
        } else if (type === "mouse_pressed" || type === "mouse_released") {
            const btnName = BROWSER_BUTTON_TO_KEY_NAME[event.button];
            if (!btnName) return;
            //track this always regardless of m1 key being in custom layout row or not for now TODO: add conditions for mouse_pad and trail highlight being there
            const isPress = type === "mouse_pressed";
            if (isPress) this.visualizer.activeMouseButtons.add(btnName);
            else this.visualizer.activeMouseButtons.delete(btnName);
            const elements = els.mouseElements.get(btnName);
            if (elements?.length) {
                for (const el of elements) this.visualizer.updateElementState(el, btnName, isPress, this.visualizer.activeMouseButtons);
            }
        } else if (type === "mouse_wheel") {
            if (els.scrollDisplays?.length) this.visualizer.handleScroll(Math.sign(event.deltaY));
        } else if (type === "mouse_moved") {
            if (this.visualizer.mousePadCanvas)
                this.visualizer.handleMouseMove(event.movementX, event.movementY);
        }
    }

    setupBackgroundVideo() {
        const video = document.getElementById("bgvideo");
        const source = document.getElementById("bgsource");
        if (video && source) {
            source.src = `./media/preview_gameplay${Math.floor(Math.random() * 2) + 1}.mp4`;
            video.load();
            video.play();
        }
    }

    setupCheatSheetToggle() {
        for (const details of document.querySelectorAll(".fullscreen-details")) {
            const closeBtn = details.querySelector(".close-btn");
            if (!closeBtn) continue;
            closeBtn.addEventListener("click", (e) => { e.preventDefault(); details.open = false; });
            const update = () => { closeBtn.style.display = details.open ? "block" : "none"; };
            update();
            details.addEventListener("toggle", update);
        }
    }

    async copyLink() {
        const linkInput = document.getElementById("generatedlink");
        const copyBtn = document.getElementById("copybtn");
        try {
            await navigator.clipboard.writeText(linkInput.value);
        } catch {
            linkInput.select();
            document.execCommand("copy");
        }
        flashBtn(copyBtn, "copied", "⎘ copy url");
    }

    async copyShareLink() {
        const shareBtn = document.getElementById("copysharebtn");
        try {
            const settings = this.getCurrentSettings();
            const shareSettings = { ...settings, wsauth: "" };
            const paramsString = this.urlManager.buildURLParams(shareSettings);
            const compressed = this.urlManager.compressSettings(paramsString);
            const base = `${window.location.origin}${window.location.pathname}`;
            const shareUrl = compressed
                ? `${base}?cfg=${compressed}`
                : `${base}?${paramsString}`;
            try {
                await navigator.clipboard.writeText(shareUrl);
            } catch {
                const tmp = document.createElement("textarea");
                tmp.value = shareUrl;
                document.body.appendChild(tmp);
                tmp.select();
                document.execCommand("copy");
                document.body.removeChild(tmp);
            }
            flashBtn(shareBtn, "copied share link!", "share");
        } catch {
            flashBtn(shareBtn, "error", "share");
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
            const viz = this.visualizer;
            if (!viz.previewElements) return;
            if (document.getElementById("forcedisableanalog")?.checked) return;

            const currentScancodes = new Set(activeKeys.map(k => String(k.scancode)));

            for (const { scancode, value } of activeKeys) {
                const keyName = HID_TO_KEY_NAME[scancode];
                if (!keyName) continue;

                if (!viz.forceDisableAnalog) {
                    if (!viz.analogMode) { viz.analogMode = true; viz.applyStyles(this.getCurrentSettings(), true); }
                    viz.setAnalogDepthTarget(keyName, value);
                }

                const wasAbove = (this.analogSensePrevDepths[scancode] ?? 0) >= DIGITAL_THRESHOLD;
                const isAbove = value >= DIGITAL_THRESHOLD;

                if (isAbove !== wasAbove) {
                    const elements = viz.previewElements.keyElements.get(keyName);
                    if (elements) for (const el of elements) viz.updateElementState(el, keyName, isAbove, viz.activeKeys);
                    if (isAbove) this.analogSenseActiveKeys.add(scancode);
                    else this.analogSenseActiveKeys.delete(scancode);
                }

                this.analogSensePrevDepths[scancode] = value;
            }

            for (const scancode of this.analogSenseActiveKeys) {
                if (currentScancodes.has(String(scancode))) continue;
                const keyName = HID_TO_KEY_NAME[scancode];
                if (keyName) {
                    if ((this.analogSensePrevDepths[scancode] ?? 0) >= DIGITAL_THRESHOLD) {
                        const elements = viz.previewElements.keyElements.get(keyName);
                        if (elements) for (const el of elements) viz.updateElementState(el, keyName, false, viz.activeKeys);
                    }
                    viz.setAnalogDepthTarget(keyName, 0);
                }
                delete this.analogSensePrevDepths[scancode];
                this.analogSenseActiveKeys.delete(scancode);
            }
        };

        const setConnected = (name) => {
            statusEl.textContent = `connected: ${name}`;
            statusEl.style.color = "#5cf67d";
            btn.textContent = "disconnect";
            btn.classList.add("connected");
        };

        const disconnect = () => {
            this.analogSenseProvider?.stopListening();
            this.analogSenseProvider = null;
            this.analogSenseActiveKeys.clear();
            this.analogSensePrevDepths = {};

            const viz = this.visualizer;
            if (viz.analogMode) {
                viz.analogMode = false;
                if (viz.analogRafId) { cancelAnimationFrame(viz.analogRafId); viz.analogRafId = null; }
                viz.analogTargetDepths = {};
                viz.analogCurrentDepths = {};

                if (viz.previewElements) {
                    viz.previewElements.keyElements.forEach((elements, keyName) => {
                        for (const el of elements) {
                            if (el.classList.contains("active") || el.classList.contains("analog-key")) {
                                viz.updateElementState(el, keyName, false, viz.activeKeys);
                                el.classList.remove("analog-key");
                                el.style.removeProperty("--analog-depth");
                                el.querySelector(".key-label-primary")?.style.removeProperty("color");
                                const inv = el.querySelector(".key-label-inverted");
                                if (inv) inv.style.clipPath = "inset(100% 0 0 0)";
                            }
                        }
                    });
                }
                viz.applyStyles(this.getCurrentSettings(), true);
            }

            statusEl.textContent = "disconnected";
            statusEl.style.color = "#808080";
            btn.textContent = "connect analog";
            btn.classList.remove("connected");
        };

        const connectDevice = async (provider) => {
            this.analogSenseProvider?.stopListening();
            this.analogSenseProvider = provider;
            provider.startListening(handleAnalogReport);
            setConnected(provider.getProductName());
        };

        analogsense.getDevices().then(devices => { if (devices.length) connectDevice(devices[0]); });

        btn.addEventListener("click", async () => {
            if (this.analogSenseProvider) { disconnect(); return; }
            try {
                const device = await analogsense.requestDevice();
                if (device) await connectDevice(device);
                else { statusEl.textContent = "no compatible keyboard found"; statusEl.style.color = "#f65c5c"; }
            } catch (e) {
                if (e.name !== "SecurityError") { statusEl.textContent = `error: ${e.message}`; statusEl.style.color = "#f65c5c"; }
            }
        });
    }

    setupKeyAddButtons() {
        const popup = document.getElementById("keyAddPopup");
        const keySelect = document.getElementById("popupKeySelect");
        const labelInput = document.getElementById("popupKeyLabel");
        const widthSlider = document.getElementById("popupWidthSlider");
        const widthValue = document.getElementById("popupWidthValue");
        const heightSlider = document.getElementById("popupHeightSlider");
        const heightValue = document.getElementById("popupHeightValue");
        const heightField = document.getElementById("popupHeightField");
        const addBtn = document.getElementById("popupAddBtn");
        const cancelBtn = document.getElementById("popupCancelBtn");
        const scrollerLabels = document.getElementById("popupScrollerLabels");
        const mouseSideLabels = document.getElementById("popupMouseSideLabels");
        const anchorField = document.getElementById("popupAnchorField");
        const anchorSelect = document.getElementById("popupAnchorSelect");

        let currentTargetRow = null, originalValue = "", isUpdating = false;

        const updateKeyString = () => {
            if (isUpdating) return;
            const keyName = keySelect.value;
            let keyString;
            const widthClass = this.getWidthClass(parseInt(widthSlider.value));

            if (keyName === "scroller") {
                const def = document.getElementById("popupScrollerDefault").value || "M3";
                const up = document.getElementById("popupScrollerUp").value || "🡅";
                const down = document.getElementById("popupScrollerDown").value || "🡇";
                keyString = widthClass
                    ? `scroller:"${def}":"${up}":"${down}":${widthClass}`
                    : `scroller:"${def}":"${up}":"${down}"`;
            } else if (keyName === "mouse_side") {
                const m5 = document.getElementById("popupMouseSideM5").value || "M5";
                const m4 = document.getElementById("popupMouseSideM4").value || "M4";
                keyString = widthClass ? `mouse_side:"${m5}":"${m4}":${widthClass}` : `mouse_side:"${m5}":"${m4}"`;
            } else if (keyName === "mouse_pad") {
                const hClass = this.getWidthClass(parseInt(heightSlider.value)) || "u1";
                const anchor = anchorSelect.value;
                keyString = `mouse_pad:${widthClass || "u1"}:${hClass}:${anchor}`;
            } else if (keyName === "gp_ls" || keyName === "gp_rs") {
                const hClass = this.getWidthClass(parseInt(heightSlider.value)) || "u1";
                const anchor = anchorSelect.value;
                keyString = `gp_joystick:${keyName}:${widthClass || "u3"}:${hClass}:${anchor}`;
            } else if (keyName === "br") {
                keyString = "br";
            } else if (keyName === "invisible" || keyName === "dummy") {
                keyString = widthClass ? `$none:"invis":${widthClass}` : keyName;
            } else {
                const label = labelInput.value || keyName.split("_")[1].toUpperCase();
                keyString = widthClass ? `${keyName}:"${label}":${widthClass}` : `${keyName}:"${label}"`;
            }

            const targetInput = document.getElementById(`customLayout${currentTargetRow}`);
            if (targetInput) {
                targetInput.value = originalValue ? `${originalValue}, ${keyString}` : keyString;
                targetInput.dispatchEvent(new Event("input", { bubbles: true }));
            }
        };

        const sliderHandler = (slider, display) => () => {
            display.textContent = `${(parseInt(slider.value) / 100).toFixed(2)}u`;
            updateKeyString();
        };
        widthSlider.addEventListener("input", sliderHandler(widthSlider, widthValue));
        heightSlider.addEventListener("input", sliderHandler(heightSlider, heightValue));

        keySelect.addEventListener("change", () => {
            const key = keySelect.value;
            scrollerLabels.style.display = "none";
            mouseSideLabels.style.display = "none";
            anchorField.style.display = "none";
            heightField.style.display = "none";
            labelInput.parentElement.style.display = "block";

            if (key === "scroller") {
                labelInput.parentElement.style.display = "none";
                scrollerLabels.style.display = "block";
                document.getElementById("popupScrollerDefault").value = "M3";
                document.getElementById("popupScrollerUp").value = "🡅";
                document.getElementById("popupScrollerDown").value = "🡇";
            } else if (key === "mouse_side") {
                labelInput.parentElement.style.display = "none";
                mouseSideLabels.style.display = "block";
                document.getElementById("popupMouseSideM5").value = "M5";
                document.getElementById("popupMouseSideM4").value = "M4";
            } else if (key === "mouse_pad") {
                labelInput.parentElement.style.display = "none";
                heightField.style.display = "block";
                anchorField.style.display = "block";
                widthSlider.value = 500; widthValue.textContent = "5.00u";
                heightSlider.value = 300; heightValue.textContent = "3.00u";
            } else if (key === "gp_ls" || key === "gp_rs") {
                labelInput.parentElement.style.display = "none";
                heightField.style.display = "block";
                anchorField.style.display = "block";
                widthSlider.value = 300; widthValue.textContent = "3.00u";
                heightSlider.value = 300; heightValue.textContent = "3.00u";
            } else if (key === "br") {
                labelInput.parentElement.style.display = "none";
            } else if (key === "invisible" || key === "dummy") {
                labelInput.value = "invisible";
            } else {
                labelInput.value = keySelect.options[keySelect.selectedIndex].text;
            }
            updateKeyString();
        });

        labelInput.addEventListener("input", updateKeyString);
        anchorSelect.addEventListener("change", updateKeyString);
        for (const id of ["popupScrollerDefault", "popupScrollerUp", "popupScrollerDown", "popupMouseSideM5", "popupMouseSideM4"])
            document.getElementById(id).addEventListener("input", updateKeyString);

        const rowMappings = [
            ["addKey1", "Row1"], ["addKey2", "Row2"], ["addKey3", "Row3"],
            ["addKey4", "Row4"], ["addKey5", "Row5"], ["addKeyMouse", "Mouse"],
        ];

        for (const [buttonId, rowId] of rowMappings) {
            const btn = document.getElementById(buttonId);
            if (!btn) continue;
            btn.addEventListener("click", () => {
                isUpdating = false;
                currentTargetRow = rowId;
                originalValue = (document.getElementById(`customLayout${rowId}`)?.value || "").trim();

                const rect = btn.getBoundingClientRect();
                const pw = 340, ph = 400;
                let left = rect.left - pw, top = rect.top;
                if (left < 10) left = rect.right + 10;
                if (left + pw > window.innerWidth - 10) left = Math.max(10, (window.innerWidth - pw) / 2);
                if (top + ph > window.innerHeight - 10) top = Math.max(10, window.innerHeight - ph - 10);
                if (top < 10) top = 10;

                popup.style.cssText = `display:block;left:${left}px;top:${top}px;`;
                keySelect.value = "key_a";
                labelInput.value = "A";
                widthSlider.value = 100; widthValue.textContent = "1.00u";
                heightSlider.value = 100; heightValue.textContent = "1.00u";
                heightField.style.display = "none";
                scrollerLabels.style.display = "none";
                mouseSideLabels.style.display = "none";
                anchorField.style.display = "none";
                anchorSelect.value = "a-tl";
                labelInput.parentElement.style.display = "block";
                updateKeyString();
            });
        }

        const cancelPopup = () => {
            isUpdating = true;
            const inp = document.getElementById(`customLayout${currentTargetRow}`);
            if (inp) { inp.value = originalValue; inp.dispatchEvent(new Event("input", { bubbles: true })); }
            popup.style.display = "none";
        };

        addBtn.addEventListener("click", () => { popup.style.display = "none"; });
        cancelBtn.addEventListener("click", cancelPopup);
        popup.addEventListener("click", (e) => { if (e.target === popup) cancelPopup(); });
    }

    getWidthClass(value) {
        if (value === 100) return "";
        const units = value / 100;
        const intPart = Math.floor(units);
        const decNum = Math.round((units - intPart) * 100);
        if (!decNum) return `u${intPart}`;
        let dec = decNum.toString().padStart(2, "0");
        if (dec.endsWith("0") && !dec.startsWith("0")) dec = dec.slice(0, -1);
        return `u${intPart}-${dec}`;
    }

    async downloadOverlayHTML() {
        const btn = document.getElementById("downloadbtn");
        const original = btn?.textContent || "⬇ download html";
        if (btn) { btn.textContent = "bundling..."; btn.disabled = true; }

        try {
            const settings = this.getCurrentSettings();
            const paramsString = this.urlManager.buildURLParams(settings);
            const compressed = this.urlManager.compressSettings(paramsString);
            const cfgParam = compressed ? `cfg=${compressed}` : paramsString;
            const wsParam = `ws=${settings.wsaddress || "localhost"}:${settings.wsport || "4455"}`;
            const wsauth = settings.wsauth ? `&wsauth=${encodeURIComponent(settings.wsauth)}` : "";

            const base = window.location.origin + window.location.pathname.replace(/\/[^/]*$/, "/");
            const scriptsBase = `${base}scripts/`;

            const fetchText = async (url) => {
                const r = await fetch(url);
                if (!r.ok) throw new Error(`failed to fetch ${url}: ${r.status}`);
                const text = await r.text();
                if (text.trimStart().startsWith("<")) throw new Error(`got HTML instead of JS for ${url} — check path`);
                return text;
            };

            const [cssText, consts, utils, urlManager, layoutParser, overlayVisualiser, webSocketManager, overlay, pakoText] = await Promise.all([
                fetchText(`${base}style.css`),
                fetchText(`${scriptsBase}consts.js`),
                fetchText(`${scriptsBase}utils.js`),
                fetchText(`${scriptsBase}services/urlManager.js`),
                fetchText(`${scriptsBase}services/layoutParser.js`),
                fetchText(`${scriptsBase}services/overlayVisualiser.js`),
                fetchText(`${scriptsBase}services/webSocketManager.js`),
                fetchText(`${scriptsBase}services/overlay.js`),
                fetch("https://cdn.jsdelivr.net/npm/pako/dist/pako.min.js").then(r => r.ok ? r.text() : fetchText(`${scriptsBase}libs/pako.min.js`)).catch(() => fetchText(`${scriptsBase}libs/pako.min.js`)),
            ]);

            const strip = (src) => src
                .replace(/^\/\/.*$/gm, "")
                .replace(/^\s*import\s+.*?from\s+['"][^'"]+['"]\s*;?\s*$/gm, "")
                .replace(/^\s*export\s+(default\s+)?(class|function|const|let|var)\s+/gm, "$2 ");

            const bakedSearch = `?${cfgParam}&${wsParam}${wsauth}`;

            const bundledJS = `
// overlay.girlglock.com generated at ${new Date().toISOString()}
(function() {
'use strict';

${strip(consts)}

${strip(utils)}

${strip(urlManager)}

${strip(layoutParser)}

${strip(overlayVisualiser)}

${strip(webSocketManager)}

${strip(overlay)}

(function boot() {
    const BAKED_PARAMS = '${bakedSearch.replace(/'/g, "\\'")}';
    const _bakedParams = new URLSearchParams(BAKED_PARAMS.slice(1));

    document.addEventListener("DOMContentLoaded", () => {
        const utils = new Utils();
        const urlManager = new UrlManager(utils);
        urlManager.urlParams = _bakedParams;
        urlManager.isOverlayMode = _bakedParams.has("ws");
        const layoutParser = new LayoutParser();
        const visualizer = new OverlayVisualiser(utils, layoutParser);
        new OverlayMode(utils, urlManager, layoutParser, visualizer);
    });
})();

})();
`;

            const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>input overlay</title>
<style>
${cssText}
</style>
</head>
<body>
<div class="overlay-container show" id="overlay">
    <div class="return status" id="return" style="margin-bottom:60px">
        <button class="return-button" style="background:none;border:none;color:aliceblue;cursor:pointer" id="return-button">edit this overlay</button>
    </div>
    <div class="status" id="status">connecting...</div>
    <div class="container" id="inner-overlay-container" style="position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);">
        <div class="top-section">
            <div class="keyboard-section" id="keyboard-target"></div>
        </div>
        <div class="bottom-section" id="mouse-target"></div>
    </div>
</div>
<div id="configurator" style="display:none"></div>
<script>
${pakoText}
</script>
<script>
${bundledJS}
</script>
<script>
document.addEventListener("DOMContentLoaded", () => {
    let timer;
    const returnBox = document.getElementById("return");
    document.addEventListener("mousemove", () => {
        returnBox.classList.add("show");
        clearTimeout(timer);
        timer = setTimeout(() => returnBox.classList.remove("show"), 1500);
    });
    document.getElementById("return-button").addEventListener("click", () => {
        window.open("${window.location.origin}${window.location.pathname}?${cfgParam}", "_blank");
    });
});

window.setDynamicScale = function setDynamicScale() {
    const container = document.getElementById("inner-overlay-container");
    if (!container) return;
    container.style.transformOrigin = "0 0";
    container.style.transform = "none";
    requestAnimationFrame(() => {
        const cr = container.getBoundingClientRect();
        let left = cr.left, top = cr.top, right = cr.right, bottom = cr.bottom;
        container.querySelectorAll(".mousepad-wrap").forEach(wrap => {
            const wr = wrap.getBoundingClientRect();
            if (wr.width === 0 && wr.height === 0) return;
            if (wr.left < left) left = wr.left;
            if (wr.top < top) top = wr.top;
            if (wr.right > right) right = wr.right;
            if (wr.bottom > bottom) bottom = wr.bottom;
        });
        const totalW = right - left, totalH = bottom - top;
        const scale = Math.min(window.innerWidth / totalW, window.innerHeight / totalH) * 0.65;
        const tx = -(left + totalW / 2 - window.innerWidth / 2) * scale;
        const ty = -(top + totalH / 2 - window.innerHeight / 2) * scale;
        container.style.transform = \`translate(\${tx}px, \${ty}px) scale(\${scale})\`;
    });
};
window.addEventListener("resize", window.setDynamicScale);
</script>
</body>
</html>`;

            const blob = new Blob([html], { type: "text/html" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = "overlay.html";
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);

            if (btn) flashBtn(btn, "downloaded!", original);
        } catch (e) {
            console.error("overlay bundle error:", e);
            if (btn) flashBtn(btn, "error", original);
        } finally {
            if (btn) btn.disabled = false;
        }
    }
}