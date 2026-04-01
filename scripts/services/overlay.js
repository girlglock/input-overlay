//guh
import {WebSocketManager} from "./webSocketManager.js";
import {GamepadManager} from "./gamepadManager.js";

export class OverlayMode {
    constructor(utils, urlManager, layoutParser, visualizer) {
        this.urlManager = urlManager;
        this.visualizer = visualizer;

        document.getElementById("configurator").style.display = "none";
        document.getElementById("overlay").classList.add("show");
        const statusEl = document.getElementById("status");

        const settings = this.urlManager.getOverlaySettings();

        requestAnimationFrame(() => {
            this.visualizer.applyStyles(settings);
            this.visualizer.rebuildInterface(settings);

            const wsOnlyGamepad = !layoutParser.needsWebSocket(settings);

            if (!wsOnlyGamepad) {
                const wsConfig = (this.urlManager.urlParams.get("ws") || "").split(":");
                const wsAddress = wsConfig[0] || "localhost";
                const wsPort = wsConfig[1] || "16899";
                const wsUrl = `ws://${wsAddress}:${wsPort}/`;
                const wsAuth = settings.wsauth || "";

                this.websocketManager = new WebSocketManager(wsUrl, statusEl, this.visualizer, wsAuth);
                this.websocketManager.connect();
            } else {
                statusEl.style.display = "none";
            }

            this.gamepadManager = new GamepadManager(this.visualizer);

            window.addEventListener("focus", () => {
                if (this.websocketManager) {
                    this.websocketManager.clearStuckKeys();
                }
                if (this.gamepadManager) {
                    this.gamepadManager.clearAll();
                }
            });

            this.visualizer.adjustScrollDisplays();
            this.visualizer.adjustKeyFontSizes(parseFloat(visualizer.outlineScaleUnpressed) || 0);
        });
    }
}