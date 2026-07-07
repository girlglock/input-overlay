//guh
import { Utils } from "./utils.js";
import { UrlManager } from "./services/urlManager.js";
import { LayoutParser } from "./services/layoutParser.js";
import { KeyLayoutParser } from "./services/keyLayoutParser.js";
import { OverlayVisualiser } from "./services/overlayVisualiser.js";
import { OverlayMode } from "./services/overlay.js";
import { ConfiguratorMode } from "./services/configurator.js";

document.addEventListener("DOMContentLoaded", () => {
    const utils = new Utils();
    const urlManager = new UrlManager(utils);
    const layoutParser = new LayoutParser();
    const keyLayoutParser = new KeyLayoutParser();
    const visualizer = new OverlayVisualiser(utils, layoutParser, keyLayoutParser);

    if (urlManager.isOverlayMode) {
        new OverlayMode(utils, urlManager, layoutParser, visualizer, keyLayoutParser);
    } else {
        new ConfiguratorMode(utils, urlManager, layoutParser, visualizer, keyLayoutParser);
    }
});