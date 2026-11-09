//guh
import { WebSocketManager } from "./webSocketManager.js";
import { GamepadManager } from "./gamepadManager.js";
import { DEFAULT_KEY_LAYOUT_TUPLES } from "./defaultLayout.js";

export class OverlayMode {
    constructor(utils, urlManager, layoutParser, visualizer, keyLayoutParser = null) {
        this.urlManager = urlManager;
        this.visualizer = visualizer;

        document.getElementById("overlay").classList.add("show");
        document.body.classList.add("overlay-mode");
        const statusEl = document.getElementById("status");

        const settings = this.urlManager.getOverlaySettings();

        // the standalone app has no hosted configurator, so use the built-in layout when none is given
        if (!settings.keyLayout) {
            settings.keyLayout = JSON.stringify(DEFAULT_KEY_LAYOUT_TUPLES);
        }

        requestAnimationFrame(() => {
            this.visualizer.applyStyles(settings);
            this.visualizer.rebuildInterface(settings);

            let wsOnlyGamepad;
            if (settings.keyLayout && keyLayoutParser) {
                let tuples = keyLayoutParser.decompressTuples(settings.keyLayout);
                if (!tuples) { try { tuples = JSON.parse(settings.keyLayout); } catch { tuples = null; } }
                const defs = tuples ? keyLayoutParser.parseAll(tuples) : [];
                wsOnlyGamepad = !keyLayoutParser.needsWebSocket(defs);
            }

            if (!wsOnlyGamepad) {
                const wsConfig = (this.urlManager.urlParams.get("ws") || "").split(":");
                const wsAddress = wsConfig[0] || "localhost";
                const wsPort = wsConfig[1] || "4455";
                const wsUrl = `ws://${wsAddress}:${wsPort}/`;
                const wsAuth = settings.wsauth || "";

                this.websocketManager = new WebSocketManager(wsUrl, statusEl, this.visualizer, wsAuth, utils);
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