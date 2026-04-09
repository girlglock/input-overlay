//gup
import { RAW_CODE_TO_KEY_NAME, MOUSE_BUTTON_MAP } from "../consts.js";

/**
 * @typedef {Object} KeyboardInputEvent
 * @property {"key_pressed" | "key_released"} event_type
 * @property {number} rawcode
 */

/**
 * @typedef {Object} MouseMoveEvent
 * @property {"mouse_moved" | "mouse_dragged"} event_type
 */

/**
 * @typedef {Object} MouseButtonEvent
 * @property {"mouse_pressed" | "mouse_released"} event_type
 * @property {number} button
 */

/**
 * @typedef {Object} MouseWheelEvent
 * @property {"mouse_wheel"} event_type
 * @property {number} rotation
 */

/**
 * @typedef {Object} AnalogDepthEvent
 * @property {"analog_depth"} event_type
 * @property {number} depth
 * @property {number} [rawcode]
 */

/**
 * @typedef {KeyboardInputEvent | MouseMoveEvent | MouseButtonEvent | MouseWheelEvent | AnalogDepthEvent} InputEvent
 */

export class WebSocketManager {
    constructor(url, statusEl, visualizer, authToken, utils) {
        this.utils = utils;
        this.wsUrl = url;
        this.statusEl = statusEl;
        this.statusCurrentEl = statusEl.querySelector("#status-current");
        this.statusLogEl = statusEl.querySelector("#status-log");
        this.visualizer = visualizer;
        this.authToken = authToken;
        this.ws = null;
        this.connectionAttempts = 0;
        this.authenticated = false;
        this.keyStates = {};
        this.keyDepths = {};
        this.messageHistory = [];
        this.HISTORY_MAX_LENGTH = 100;
    }

    _setStatus(text, state) {
        this.statusCurrentEl.textContent = text;
        this.statusEl.className = `status ${state}`;
    }

    _log(text, level = "") {
        if (!this.statusLogEl) return;
        const MAX_ENTRIES = 8;
        const ts = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
        const entry = document.createElement("div");
        entry.className = `status-log-entry${level ? ` entry-${level}` : ""}`;
        entry.textContent = `${ts}  ${this.utils._maskAddress(text)}`;
        this.statusLogEl.appendChild(entry);
        while (this.statusLogEl.children.length > MAX_ENTRIES) {
            this.statusLogEl.removeChild(this.statusLogEl.firstChild);
        }
        this.statusLogEl.scrollTop = this.statusLogEl.scrollHeight;
    }

    connect() {
        this.connectionAttempts++;
        const attempt = this.connectionAttempts;
        this._setStatus(`connecting... (attempt ${attempt})`, "connecting");
        this._log(`connecting to ${this.wsUrl} (attempt ${attempt})`);
        console.log(`[ws] connect attempt ${attempt} to ${this.wsUrl}`);

        this.ws = new WebSocket(this.wsUrl);
        this.ws.onopen = this._onOpen.bind(this);
        this.ws.onmessage = this._onMessage.bind(this);
        this.ws.onerror = this._onError.bind(this);
        this.ws.onclose = this._onClose.bind(this);
    }

    _onOpen() {
        console.log("[ws] connection opened, sending auth");
        this._log("connection opened, authenticating...");
        this.ws.send(JSON.stringify({ type: "auth", token: this.authToken || "" }));
        this.authenticated = true;
        this.connectionAttempts = 0;
        const msg = this.authToken ? "connected (authenticated)" : "connected";
        this._setStatus(msg, "connected");
        this._log(msg, "ok");
        this.clearStuckKeys();
    }

    _onMessage(e) {
        let data;
        try { data = JSON.parse(e.data); } catch { return; }

        if (data.type === "auth_response") {
            if (data.status === "error" || data.status === "failed") {
                const reason = data.message || "invalid token";
                console.warn(`[ws] authentication failed: ${reason}`);
                this._log(`authentication failed: ${reason}`, "error");
                this.authenticated = false;
                this._setStatus(`authentication failed: ${reason}`, "error");
                this.ws.close();
            } else {
                console.log("[ws] authentication ok");
            }
            return;
        }

        if (this.authenticated) this._handleOverlayInput(data);
    }

    _onError(event) {
        console.error("[ws] websocket error", event);
    }

    _onClose(event) {
        this.authenticated = false;
        console.log(`[ws] closed - code: ${event.code}, reason: "${event.reason || "none"}", clean: ${event.wasClean}`);

        const CLOSE_REASONS = {
            1000: ["closed normally", "error", false, 0],
            1001: ["server going away", "error", true, 5000],
            1006: ["server unreachable?", "error", true, 5000],
            1008: ["authentication required", "error", true, 5000],
            1011: ["server error", "error", true, 5000],
        };

        const [msg, state, reconnect, delay] = CLOSE_REASONS[event.code] ?? [`disconnected (code ${event.code})`, "connecting", true, 5000];

        if (event.code === 1008) {
            this._setStatus(`connection closed: ${msg}`, state);
            this._log(`closed: ${msg}`, "error");
            console.warn("[ws] not reconnecting: auth required");
            return;
        }

        if (!reconnect) {
            this._setStatus(msg, state);
            this._log(msg, "warn");
            return;
        }

        this._log(`${msg} - reconnecting in ${delay / 1000}s`, "warn");
        this.clearStuckKeys();

        if (this._countdownInterval) clearInterval(this._countdownInterval);
        let remaining = delay;
        const tick = () => {
            const secs = Math.ceil(remaining / 1000);
            this._setStatus(`${msg} - retrying in ${secs}s`, "connecting");
            remaining -= 1000;
        };
        tick();
        this._countdownInterval = setInterval(() => {
            if (remaining <= 0) {
                clearInterval(this._countdownInterval);
                this._countdownInterval = null;
            } else {
                tick();
            }
        }, 1000);

        setTimeout(() => this.connect(), delay);
    }

    _getMappedKeyInfo(event) {
        if (event.rawcode !== undefined && (event.event_type.startsWith("key_") || event.event_type === "analog_depth")) {
            const name = RAW_CODE_TO_KEY_NAME[event.rawcode];
            return name ? { id: `k_${event.rawcode}`, name, type: "key" } : null;
        }
        if (event.button && event.event_type.startsWith("mouse_")) {
            const name = MOUSE_BUTTON_MAP[event.button];
            return name ? { id: `m_${event.button}`, name, type: "mouse" } : null;
        }
        return null;
    }

    _recalculateKeyStates() {
        const viz = this.visualizer;
        const els = viz.previewElements;
        if (!els) return;

        const desiredActive = {};
        for (const event of this.messageHistory) {
            const info = this._getMappedKeyInfo(event);
            if (!info) continue;
            const map = info.type === "key" ? els.keyElements : els.mouseElements;
            if (!map.has(info.name)) continue;
            desiredActive[info.id] = event.event_type.endsWith("_pressed");
        }

        const allIds = new Set([...Object.keys(desiredActive), ...Object.keys(this.keyStates)]);
        for (const id of allIds) {
            const desired = !!desiredActive[id];
            const current = !!this.keyStates[id];
            if (desired === current) continue;

            const isKey = id.startsWith("k_");
            const rawId = parseInt(id.substring(2));
            const name = isKey ? RAW_CODE_TO_KEY_NAME[rawId] : MOUSE_BUTTON_MAP[rawId];
            if (!name) continue;

            const elements = isKey ? els.keyElements.get(name) : els.mouseElements.get(name);
            const activeSet = isKey ? viz.activeKeys : viz.activeMouseButtons;
            if (!elements?.length) continue;

            for (const el of elements) {
                viz.updateElementState(el, name, desired, activeSet);
                if (!isKey) {
                    const animDur = viz.animDuration || "0.15s";
                    const t = `all ${animDur} cubic-bezier(0.4,0,0.2,1)`;
                    el.style.setProperty("transition", t, "important");
                    el.style.setProperty("transform", desired ? `scale(${viz.pressScaleValue || 1.05})` : "scale(1)", "important");
                }
            }
        }

        this.keyStates = {};
        for (const [id, active] of Object.entries(desiredActive)) {
            if (active) this.keyStates[id] = true;
        }
    }

    _handleOverlayInput(event) {
        const { event_type } = event;

        if (event_type === "mouse_moved" || event_type === "mouse_dragged") {
            if (this.visualizer.mousePadCanvas)
                this.visualizer.handleMouseMove(event.dx ?? 0, event.dy ?? 0);
            return;
        }

        if (event_type === "analog_depth") {
            if (!this.visualizer.forceDisableAnalog) this._handleAnalogDepth(event);
            return;
        }

        if (event_type === "mouse_wheel") {
            if (this.visualizer.previewElements?.scrollDisplay)
                this.visualizer.handleScroll(event.rotation ?? 1);
            return;
        }

        if (event_type === "key_pressed" || event_type === "key_released" ||
            event_type === "mouse_pressed" || event_type === "mouse_released") {
            //TODO: add conditions for mouse_pad and trail highlight being there
            if (event_type === "mouse_pressed" || event_type === "mouse_released") {
                const info = this._getMappedKeyInfo(event);
                if (info?.type === "mouse") {
                    if (event_type === "mouse_pressed") this.visualizer.activeMouseButtons.add(info.name);
                    else this.visualizer.activeMouseButtons.delete(info.name);
                }
            }
            this.messageHistory.push(event);
            if (this.messageHistory.length > this.HISTORY_MAX_LENGTH) this.messageHistory.shift();
            this._recalculateKeyStates();
        }
    }

    _handleAnalogDepth(event) {
        if (!this.visualizer.previewElements) return;
        if (!this.visualizer.analogMode) this.visualizer.analogMode = true;

        const info = this._getMappedKeyInfo(event);
        if (!info?.name?.startsWith("key_")) return;

        this.keyDepths[info.id] = event.depth || 0;
        this.visualizer.setAnalogDepthTarget(info.name, event.depth || 0);
    }

    clearStuckKeys() {
        const viz = this.visualizer;
        if (!viz.previewElements) return;

        const clearMap = (map) => {
            map.forEach(elements => {
                for (const el of elements) {
                    el.classList.remove("active", "analog-key");
                    viz.activeElements.delete(el);
                    el.style.transform = "";
                    el.querySelector(".key-label-primary")?.style.removeProperty("color");
                    const inv = el.querySelector(".key-label-inverted");
                    if (inv) inv.style.clipPath = "inset(100% 0 0 0)";
                }
            });
        };

        clearMap(viz.previewElements.keyElements);
        clearMap(viz.previewElements.mouseElements);
        if (viz.previewElements.gamepadElements) clearMap(viz.previewElements.gamepadElements);

        viz.activeKeys.clear();
        viz.activeMouseButtons.clear();
        viz.activeGamepadButtons?.clear();

        const { scrollDisplays, scrollArrows, scrollCounts } = viz.previewElements;
        if (scrollDisplays?.length) {
            scrollDisplays.forEach((display, i) => {
                display.classList.remove("active");
                viz.activeElements.delete(display);
                scrollArrows[i].textContent = display.dataset.defaultLabel || "-";
                scrollCounts[i].textContent = "";
            });
        }

        viz.currentScrollCount = 0;
        this.messageHistory = [];
        this.keyStates = {};
        this.keyDepths = {};

        if (viz.analogRafId) {
            cancelAnimationFrame(viz.analogRafId);
            viz.analogRafId = null;
        }
        viz.analogTargetDepths = {};
        viz.analogCurrentDepths = {};

        viz.mousePadTrail = [];
        viz.mousePadCursorX = null;
        viz.mousePadCursorY = null;
        viz.MOUSEPAD_PAN_X = 0;
        viz.MOUSEPAD_PAN_Y = 0;
        viz._mousePadTotalDistancePx = 0;
        viz._mousePadLastFrameTime = 0;
        if (viz.mousePadRafId) {
            cancelAnimationFrame(viz.mousePadRafId);
            viz.mousePadRafId = null;
        }
        if (viz.mousePadCtx && viz.mousePadCanvas) {
            viz.mousePadCtx.clearRect(0, 0, viz.mousePadCanvas.width, viz.mousePadCanvas.height);
        }
    }
}