//me when she when we

export const GP_BUTTON_MAP = {
    0:  "gp_a",
    1:  "gp_b",
    2:  "gp_x",
    3:  "gp_y",
    4:  "gp_lb",
    5:  "gp_rb",
    6:  "gp_lt",
    7:  "gp_rt",
    8:  "gp_select",
    9:  "gp_start",
    10: "gp_ls",
    11: "gp_rs",
    12: "gp_up",
    13: "gp_down",
    14: "gp_left",
    15: "gp_right",
    16: "gp_guide",
};

export const GP_AXIS_MAP = {
    0: { stick: "gp_ls", axis: "x" },
    1: { stick: "gp_ls", axis: "y" },
    2: { stick: "gp_rs", axis: "x" },
    3: { stick: "gp_rs", axis: "y" },
};

const ANALOG_BUTTONS = new Set([6, 7]); //triggers

const DEADZONE = 0.08;

export class GamepadManager {
    constructor(visualizer) {
        this.visualizer = visualizer;
        this._rafId = null;
        this._prevButtonState = {};
        this._prevAxisState   = {};
        this._stickValues     = {};

        this._loop = this._loop.bind(this);

        window.addEventListener("gamepadconnected",    e => this._onConnected(e));
        window.addEventListener("gamepaddisconnected", e => this._onDisconnected(e));
        this._startLoop();
        setTimeout(window.setDynamicScale, 100);
    }

    _onConnected(e) {
        console.log(`[gamepadManager] connected: "${e.gamepad.id}" index: ${e.gamepad.index}`);
        this._startLoop();
    }

    _onDisconnected(e) {
        console.log(`[gamepadManager] disconnected: index ${e.gamepad.index}`);
        const prev = this._prevButtonState[e.gamepad.index] || [];
        prev.forEach((wasPressed, btnIdx) => {
            if (wasPressed) this._setButton(GP_BUTTON_MAP[btnIdx], false, 0);
        });
        delete this._prevButtonState[e.gamepad.index];
        delete this._prevAxisState[e.gamepad.index];
    }

    _startLoop() {
        if (this._rafId) return;
        this._rafId = requestAnimationFrame(this._loop);
    }

    stop() {
        if (this._rafId) { cancelAnimationFrame(this._rafId); this._rafId = null; }
    }

    _loop() {
        this._rafId = null;
        const pads = navigator.getGamepads ? navigator.getGamepads() : [];
        let anyConnected = false;

        for (const pad of pads) {
            if (!pad) continue;
            anyConnected = true;
            this._processPad(pad);
        }

        this._rafId = requestAnimationFrame(this._loop);
    }

    _processPad(pad) {
        const idx = pad.index;
        if (!this._prevButtonState[idx]) this._prevButtonState[idx] = [];
        if (!this._prevAxisState[idx])   this._prevAxisState[idx]   = [];

        pad.buttons.forEach((btn, i) => {
            const name = GP_BUTTON_MAP[i];
            if (!name) return;

            const isPressed = btn.pressed;
            const wasPressed = !!this._prevButtonState[idx][i];

            if (isPressed !== wasPressed) {
                this._setButton(name, isPressed, btn.value);
            } else if (isPressed && ANALOG_BUTTONS.has(i)) {
                this._setAnalog(name, btn.value);
            }

            this._prevButtonState[idx][i] = isPressed;

            if (ANALOG_BUTTONS.has(i)) {
                const prevVal = this._prevAxisState[idx][`b${i}`] ?? 0;
                if (Math.abs(btn.value - prevVal) > 0.005) {
                    this._setAnalog(name, btn.value);
                    this._prevAxisState[idx][`b${i}`] = btn.value;
                }
            }
        });

        const axisCount = Math.min(pad.axes.length, 4);
        for (let a = 0; a < axisCount; a++) {
            const raw = pad.axes[a];
            const val = Math.abs(raw) < DEADZONE ? 0 : raw;
            const prev = this._prevAxisState[idx][a] ?? 0;
            if (Math.abs(val - prev) > 0.002) {
                const info = GP_AXIS_MAP[a];
                if (info) this._setAxis(info.stick, info.axis, val);
                this._prevAxisState[idx][a] = val;
            }
        }
    }

    _setButton(name, isPressed, value) {
        const viz = this.visualizer;
        if (!viz.previewElements) return;

        const elements = viz.previewElements.gamepadElements?.get(name);
        if (!elements?.length) return;

        const isTrigger = name === "gp_lt" || name === "gp_rt";

        for (const el of elements) {
            viz.updateElementState(el, name, isPressed, viz.activeGamepadButtons);
            if (!isTrigger) {
                const animDur = viz.animDuration || "0.15s";
                const t = `all ${animDur} cubic-bezier(0.4,0,0.2,1)`;
                el.style.setProperty("transition", t, "important");
                el.style.setProperty("transform", isPressed ? `scale(${viz.pressScaleValue || 1.05})` : "scale(1)", "important");
            }
        }

        if (isTrigger) {
            viz.setAnalogDepthTarget(name, isPressed ? value : 0, "gamepad");
        }
    }

    _setAnalog(name, depth) {
        const viz = this.visualizer;
        if (!viz.previewElements) return;
        viz.setAnalogDepthTarget(name, depth, "gamepad");
    }

    _setAxis(stickId, axis, value) {
        const viz = this.visualizer;
        if (!viz.previewElements) return;

        if (!this._stickValues[stickId]) this._stickValues[stickId] = { x: 0, y: 0 };
        this._stickValues[stickId][axis] = value;

        const { x, y } = this._stickValues[stickId];
        viz.handleJoystickMove(stickId, x, y);
    }

    clearAll() {
        const viz = this.visualizer;
        if (!viz.previewElements?.gamepadElements) return;
        viz.previewElements.gamepadElements.forEach((elements, name) => {
            for (const el of elements) {
                el.classList.remove("active", "analog-key");
                viz.activeElements?.delete(el);
                el.style.transform = "";
            }
        });
        viz.activeGamepadButtons?.clear();
        this._prevButtonState = {};
        this._prevAxisState   = {};
        this._stickValues     = {};
    }
}