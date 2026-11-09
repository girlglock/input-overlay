//guh
import { Utils } from "./utils.js";
import { UrlManager } from "./services/urlManager.js";
import { LayoutParser } from "./services/layoutParser.js";
import { KeyLayoutParser } from "./services/keyLayoutParser.js";
import { OverlayVisualiser } from "./services/overlayVisualiser.js";
import { OverlayMode } from "./services/overlay.js";

document.addEventListener("DOMContentLoaded", () => {
    const utils = new Utils();
    const urlManager = new UrlManager(utils);
    const layoutParser = new LayoutParser();
    const keyLayoutParser = new KeyLayoutParser();
    const visualizer = new OverlayVisualiser(utils, layoutParser, keyLayoutParser);

    new OverlayMode(utils, urlManager, layoutParser, visualizer, keyLayoutParser);
});
