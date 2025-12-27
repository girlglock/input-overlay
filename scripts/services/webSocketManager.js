//gup
import { RAW_CODE_TO_KEY_NAME, MOUSE_BUTTON_MAP } from "../consts.js";

export class WebSocketManager {
    constructor(url, statusEl, visualizer, authToken) {
        this.wsUrl = url;
        this.statusEl = statusEl;
        this.visualizer = visualizer;
        this.authToken = authToken;
        this.elements = visualizer.previewElements;
        this.ws = null;
        this.connectionAttempts = 0;
        this.authenticated = false;

        this.messageHistory = [];
        this.keyStates = {};
        this.HISTORY_MAX_LENGTH = 100;
    }

    connect() {
        this.connectionAttempts++;
        this.statusEl.textContent = `connecting to ${this.wsUrl} (attempt ${this.connectionAttempts})...`;
        this.statusEl.className = "status connecting";

        this.ws = new WebSocket(this.wsUrl);

        this.ws.onopen = this.onOpen.bind(this);
        this.ws.onmessage = this.onMessage.bind(this);
        this.ws.onerror = this.onError.bind(this);
        this.ws.onclose = this.onClose.bind(this);
    }

    onOpen() {
        this.ws.send(JSON.stringify({
            type: 'auth',
            token: this.authToken || ''
        }));

        this.authenticated = true;
        this.connectionAttempts = 0;
        this.statusEl.textContent = this.authToken ? "connected (authenticated)" : "connected";
        this.statusEl.className = "status connected";
        this.clearStuckKeys();
    }

    onMessage(e) {
        try {
            const data = JSON.parse(e.data);

            if (data.type === 'auth_response') {
                if (data.status === 'error' || data.status === 'failed') {
                    this.authenticated = false;
                    const reason = data.message || "invalid token";
                    this.statusEl.textContent = `authentication failed: ${reason}`;
                    this.statusEl.className = "status error";
                    this.ws.close();
                }
                return;
            }

            if (this.authenticated) {
                this.handleOverlayInput(e.data);
            }
        } catch (err) {
            if (this.authenticated) {
                this.handleOverlayInput(e.data);
            }
        }
    }

    onError() {
        this.statusEl.textContent = "connection failed";
        this.statusEl.className = "status error";
    }

    onClose(event) {
        this.authenticated = false;
        if (event.code === 1008) {
            this.statusEl.textContent = "connection closed: authentication required";
            this.statusEl.className = "status error";
            return;
        }

        this.statusEl.textContent = "disconnected. reconnecting...";
        this.statusEl.className = "status connecting";
        this.clearStuckKeys();
        setTimeout(() => this.connect(), 2000);
    }

    getMappedKeyId(event) {
        if (event.event_type.startsWith("key_")) {
            return {
                id: `k_${event.rawcode}`,
                name: RAW_CODE_TO_KEY_NAME[event.rawcode],
                type: "key"
            };
        } else if (event.event_type.startsWith("mouse_") && event.button) {
            return {
                id: `m_${event.button}`,
                name: MOUSE_BUTTON_MAP[event.button],
                type: "mouse"
            };
        }
        return null;
    }

    recalculateKeyStates() {
        const tempStates = {};
        const isKeyActive = {};
        this.elements = this.visualizer.previewElements;

        for (const event of this.messageHistory) {
            const keyInfo = this.getMappedKeyId(event);
            if (!keyInfo || !keyInfo.name || !this.elements) continue;

            const elementMap = keyInfo.type === "key" ? this.elements.keyElements : this.elements.mouseElements;
            const elements = elementMap.get(keyInfo.name);
            if (!elements || elements.length === 0) continue;

            if (event.event_type.endsWith("_pressed")) {
                isKeyActive[keyInfo.id] = true;
            } else if (event.event_type.endsWith("_released")) {
                isKeyActive[keyInfo.id] = false;
            }
        }

        const keysToCheck = new Set([...Object.keys(isKeyActive), ...Object.keys(this.keyStates)]);

        for (const keyId of keysToCheck) {
            const isActive = isKeyActive[keyId] !== undefined ? isKeyActive[keyId] : (this.keyStates[keyId] === true);
            const wasActive = this.keyStates[keyId] === true;

            if (isActive !== wasActive && this.elements) {
                const type = keyId.startsWith("k_") ? "key" : "mouse";
                const idValue = parseInt(keyId.substring(2));
                const keyName = type === "key" ? RAW_CODE_TO_KEY_NAME[idValue] : MOUSE_BUTTON_MAP[idValue];

                const elements = type === "key" ? this.elements.keyElements.get(keyName) : this.elements.mouseElements.get(keyName);
                const activeSet = type === "key" ? this.visualizer.activeKeys : this.visualizer.activeMouseButtons;

                if (elements && elements.length > 0) {
                    elements.forEach(el => {
                        this.visualizer.updateElementState(el, keyName, isActive, activeSet);
                    });
                }
            }
            tempStates[keyId] = isActive;
        }

        this.keyStates = Object.fromEntries(
            Object.entries(tempStates).filter(([keyId, isActive]) => isActive || Object.hasOwn(isKeyActive, keyId))
        );
    }

    handleOverlayInput(data) {
        try {
            const event = JSON.parse(data);
            if (event.event_type === "mouse_moved" || event.event_type === "mouse_dragged") return;

            if (event.event_type === "mouse_wheel") {
                const dir = event.rotation;
                if (this.visualizer.previewElements.scrollDisplay) {
                    this.visualizer.handleScroll(dir);
                }
                return;
            }

            if (event.event_type === "key_pressed" && this.visualizer.previewElements) {
                const keyName = RAW_CODE_TO_KEY_NAME[event.rawcode];
                if (keyName && this.visualizer.scrollerAliases && this.visualizer.scrollerAliases.has(keyName)) {
                    const scrollDir = this.visualizer.scrollerAliases.get(keyName);
                    if (this.visualizer.previewElements.scrollDisplay) {
                        this.visualizer.handleScroll(scrollDir);
                    }
                }
            }

            this.messageHistory.push(event);
            if (this.messageHistory.length > this.HISTORY_MAX_LENGTH) {
                this.messageHistory.shift();
            }

            if (["key_released", "mouse_released", "key_pressed", "mouse_pressed"].includes(event.event_type)) {
                this.recalculateKeyStates();
            }

        } catch (err) { }
    }

    clearStuckKeys() {
        if (!this.visualizer.previewElements) return;

        const clearElements = (map) => {
            map.forEach(elements => {
                elements.forEach(el => {
                    el.classList.remove("active");
                    this.visualizer.activeElements.delete(el);
                });
            });
        };

        clearElements(this.visualizer.previewElements.keyElements);
        clearElements(this.visualizer.previewElements.mouseElements);

        this.visualizer.activeKeys.clear();
        this.visualizer.activeMouseButtons.clear();

        if (this.visualizer.previewElements.scrollDisplays?.length > 0) {
            this.visualizer.previewElements.scrollDisplays.forEach((display, index) => {
                display.classList.remove("active");
                this.visualizer.activeElements.delete(display);
                this.visualizer.previewElements.scrollArrows[index].textContent = display.dataset.defaultLabel || "-";
                this.visualizer.previewElements.scrollCounts[index].textContent = "";
            });
        }
        this.visualizer.currentScrollCount = 0;
        this.messageHistory = [];
        this.keyStates = {};
    }
}