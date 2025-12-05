//guh
import {WebSocketManager} from "./webSocketManager.js";

export class OverlayMode {
    constructor(utils, urlManager, layoutParser, visualizer) {
        this.urlManager = urlManager;
        this.visualizer = visualizer;

        document.getElementById("configurator").style.display = "none";
        document.getElementById("overlay").classList.add("show");
        const statusEl = document.getElementById("status");

        const settings = this.urlManager.getOverlaySettings();

        requestAnimationFrame(() => {
            const innerContainer = document.getElementById("inner-overlay-container");

            if (innerContainer) {
                this.visualizer.applyTransformations(innerContainer, settings);
            }

            this.visualizer.applyStyles(settings);
            this.visualizer.rebuildInterface(settings);

            const wsConfig = (this.urlManager.urlParams.get("ws") || "").split(":");
            const wsAddress = wsConfig[0] || "localhost";
            const wsPort = wsConfig[1] || "16899";
            const wsUrl = `ws://${wsAddress}:${wsPort}/`;

            this.websocketManager = new WebSocketManager(wsUrl, statusEl, this.visualizer);
            this.websocketManager.connect();

            window.addEventListener("focus", () => {
                if (this.websocketManager) {
                    this.websocketManager.clearStuckKeys();
                }
            });

            this.visualizer.adjustScrollDisplays();
            this.visualizer.adjustKeyFontSizes();
        });
    }
}