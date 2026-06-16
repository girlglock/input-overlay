document.addEventListener('contextmenu', e => e.preventDefault());

document.querySelectorAll('menu[role=tablist] button').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('menu[role=tablist] button').forEach(b => b.setAttribute('aria-selected', 'false'));
    document.querySelectorAll('[role=tabpanel]').forEach(p => { p.hidden = true; });
    btn.setAttribute('aria-selected', 'true');
    document.getElementById(btn.getAttribute('aria-controls')).hidden = false;
  });
});

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getVersion } = window.__TAURI__.app;

const portError = document.getElementById("port-error");
const hostEl = document.getElementById("host");
const portEl = document.getElementById("port");
const authEl = document.getElementById("auth-token");
const showBtn = document.getElementById("show-token");
const copyBtn = document.getElementById("copy-token");
const regenBtn = document.getElementById("regen-token");
const autostartEl = document.getElementById("autostart");
const analogEl = document.getElementById("analog-keyboard");
const httpEnabledEl = document.getElementById("http-enabled");
const httpHostEl = document.getElementById("http-host");
const httpPortEl = document.getElementById("http-port");
const openHttpBtn = document.getElementById("open-http-btn");
const adminWarning = document.getElementById("admin-warning");
const addKeyBtn = document.getElementById("add-key-btn");
const keyList = document.getElementById("key-list");
const mouseMove = document.getElementById("send-mouse-move");
const clientsCount = document.getElementById("clients-count");
const clientsList = document.getElementById("clients-list");
const clientsModal = document.getElementById("clients-modal");
document.getElementById("clients-modal-close").addEventListener("click", () => clientsModal.hidden = true);
document.getElementById("clients-modal-ok").addEventListener("click", () => clientsModal.hidden = true);
clientsCount.addEventListener("click", () => { if (clientsList.children.length) clientsModal.hidden = false; });
const saveBtn = document.getElementById("save-btn");
const cancelBtn = document.getElementById("cancel-btn");
const saveMsg = document.getElementById("save-msg");
const discardModal = document.getElementById("discard-modal");
const modalKeepBtn = document.getElementById("modal-keep-btn");
const modalDiscardBtn = document.getElementById("modal-discard-btn");

const themeToggle = document.getElementById("theme-toggle");
const zuneLink = document.querySelector('link[href*="XP-ZUNE"]');
const systemTheme = () => window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
let themeMode = systemTheme();
function applyThemeMode(mode) {
  themeMode = mode;
  document.documentElement.setAttribute('data-theme', mode);
  zuneLink.media = mode === 'dark' ? 'all' : 'not all';
  themeToggle.textContent = mode === 'dark' ? 'dark' : 'light';
  themeToggle.title = `toggle ${mode === 'dark' ? 'light' : 'dark'} mode`;
}
themeToggle.addEventListener('click', () => {
  const next = themeMode === 'dark' ? 'light' : 'dark';
  applyThemeMode(next);
  invoke('set_theme', { theme: next }).catch(() => {});
});
applyThemeMode(themeMode);

const updateBar = document.getElementById("update-bar");
const updateText = document.getElementById("update-text");
const updateNowBtn = document.getElementById("update-now-btn");
const updateLaterBtn = document.getElementById("update-later-btn");
const updateNotesLink = document.getElementById("update-notes-link");
const updateProgressModal = document.getElementById("update-progress-modal");
const updateProgressStatus = document.getElementById("update-progress-status");
const updateProgressFill = document.getElementById("update-progress-fill");

let whitelist = [];
let isListening = false;
let originalConfig = null;
let dirty = false;
let _saveMsgTimer = null;

function markDirty() {
  if (dirty) return;
  dirty = true;
  saveBtn.classList.add("dirty");
  saveBtn.textContent = "● Save";
  if (!_saveMsgTimer) showDirtyMsg();
}
function markClean() {
  dirty = false;
  saveBtn.classList.remove("dirty");
  saveBtn.textContent = "Save";
  if (!_saveMsgTimer) {
    saveMsg.textContent = "";
    saveMsg.classList.remove("dirty-msg");
  }
}
function showDirtyMsg() {
  saveMsg.textContent = "you have unsaved changes pending";
  saveMsg.classList.add("dirty-msg");
  saveMsg.style.color = "";
}

const CODE_TO_KEY = {
  Escape: "key_escape", Digit1: "key_1", Digit2: "key_2", Digit3: "key_3",
  Digit4: "key_4", Digit5: "key_5", Digit6: "key_6", Digit7: "key_7",
  Digit8: "key_8", Digit9: "key_9", Digit0: "key_0", Minus: "key_minus",
  Equal: "key_equals", Backspace: "key_backspace", Tab: "key_tab",
  KeyQ: "key_q", KeyW: "key_w", KeyE: "key_e", KeyR: "key_r", KeyT: "key_t",
  KeyY: "key_y", KeyU: "key_u", KeyI: "key_i", KeyO: "key_o", KeyP: "key_p",
  BracketLeft: "key_openbracket", BracketRight: "key_closebracket",
  Backslash: "key_backslash", CapsLock: "key_capslock",
  KeyA: "key_a", KeyS: "key_s", KeyD: "key_d", KeyF: "key_f", KeyG: "key_g",
  KeyH: "key_h", KeyJ: "key_j", KeyK: "key_k", KeyL: "key_l",
  Semicolon: "key_semicolon", Quote: "key_apostrophe", Enter: "key_enter",
  ShiftLeft: "key_leftshift", KeyZ: "key_z", KeyX: "key_x", KeyC: "key_c",
  KeyV: "key_v", KeyB: "key_b", KeyN: "key_n", KeyM: "key_m",
  Comma: "key_comma", Period: "key_period", Slash: "key_slash",
  ShiftRight: "key_rightshift", ControlLeft: "key_leftctrl",
  MetaLeft: "key_leftwin", AltLeft: "key_leftalt", Space: "key_space",
  AltRight: "key_rightalt", MetaRight: "key_rightwin",
  ContextMenu: "key_menu", ControlRight: "key_rightctrl",
  ArrowLeft: "key_leftarrow", ArrowUp: "key_uparrow",
  ArrowRight: "key_rightarrow", ArrowDown: "key_downarrow",
  PrintScreen: "key_printscreen", ScrollLock: "key_scrolllock",
  Pause: "key_pause", Insert: "key_insert", Delete: "key_delete",
  Home: "key_home", End: "key_end", PageUp: "key_pageup", PageDown: "key_pagedown",
  NumLock: "key_numlock", NumpadDivide: "key_numpad_divide",
  NumpadMultiply: "key_numpad_multiply", NumpadSubtract: "key_numpad_subtract",
  NumpadAdd: "key_numpad_add", NumpadEnter: "key_numpad_enter",
  Numpad0: "key_numpad_0", Numpad1: "key_numpad_1", Numpad2: "key_numpad_2",
  Numpad3: "key_numpad_3", Numpad4: "key_numpad_4", Numpad5: "key_numpad_5",
  Numpad6: "key_numpad_6", Numpad7: "key_numpad_7", Numpad8: "key_numpad_8",
  Numpad9: "key_numpad_9", NumpadDecimal: "key_numpad_decimal",
  Backquote: "key_grave", IntlBackslash: "key_iso_backslash",
  F1: "key_f1", F2: "key_f2", F3: "key_f3", F4: "key_f4",
  F5: "key_f5", F6: "key_f6", F7: "key_f7", F8: "key_f8",
  F9: "key_f9", F10: "key_f10", F11: "key_f11", F12: "key_f12",
};

const MOUSE_BTN_NAMES = {
  0: "mouse_left", 1: "mouse_middle", 2: "mouse_right", 3: "mouse_4", 4: "mouse_5",
};

function applyStatus(s) {
  if (s.running) {
    portError.hidden = true;
  } else {
    const kind = s.bind_error || "oserror";
    portError.textContent = kind === "inuse"
      ? `Port ${s.port} is already in use - try a different port`
      : kind === "denied" ? `Access to ${s.host}:${s.port} was denied`
        : `Could not bind to ${s.host}:${s.port}`;
    portError.hidden = false;
  }
  renderClients(s.clients || []);
}

function renderClients(clients) {
  clientsList.innerHTML = "";
  if (clients.length === 0) {
    clientsCount.textContent = "no clients connected";
    clientsCount.style.cursor = "";
    return;
  }
  clientsCount.textContent = `${clients.length} client${clients.length === 1 ? "" : "s"} connected`;
  clientsCount.style.cursor = "pointer";
  clients.forEach((addr) => {
    const li = document.createElement("li");
    li.textContent = addr;
    clientsList.appendChild(li);
  });
}

function applyConfig(cfg) {
  originalConfig = structuredClone(cfg);
  hostEl.value = cfg.host ?? "localhost";
  portEl.value = cfg.port ?? 4455;
  authEl.value = cfg.auth_token ?? "";
  mouseMove.checked = cfg.send_mouse_move ?? true;
  analogEl.value = cfg.analog_keyboard ?? "";
  httpEnabledEl.checked = cfg.http_enabled ?? false;
  httpHostEl.value = cfg.host ?? "localhost";
  httpPortEl.value = cfg.http_port ?? 4456;
  applyHttpEnabled(httpEnabledEl.checked);
  whitelist = [...(cfg.key_whitelist ?? [])];
  renderKeyList();
  const kbdEl = document.getElementById("kbd-device");
  const mouseDevEl = document.getElementById("mouse-device");
  if (kbdEl) kbdEl.value = cfg.linux_evdev_keyboard_device ?? "";
  if (mouseDevEl) mouseDevEl.value = cfg.linux_raw_mouse_device ?? "";
  markClean();
}

function readConfig() {
  const kbdEl = document.getElementById("kbd-device");
  const mouseDevEl = document.getElementById("mouse-device");
  return {
    host: hostEl.value.trim() || "localhost",
    port: parseInt(portEl.value, 10) || 4455,
    auth_token: authEl.value.trim(),
    send_mouse_move: mouseMove.checked,
    key_whitelist: [...whitelist],
    analog_keyboard: analogEl.value,
    http_enabled: httpEnabledEl.checked,
    http_port: parseInt(httpPortEl.value, 10) || 4456,
    raw_mouse_min_delta: originalConfig?.raw_mouse_min_delta ?? 0,
    cpu_affinity: originalConfig?.cpu_affinity ?? [0, 1],
    linux_evdev_keyboard_device: kbdEl ? kbdEl.value : (originalConfig?.linux_evdev_keyboard_device ?? ""),
    linux_raw_mouse_device: mouseDevEl ? mouseDevEl.value : (originalConfig?.linux_raw_mouse_device ?? ""),
    dismissed_update_versions: originalConfig?.dismissed_update_versions ?? [],
    theme: themeMode,
  };
}

function renderKeyList() {
  keyList.innerHTML = "";
  if (whitelist.length === 0) {
    const el = document.createElement("span");
    el.style.cssText = "font-size:11px;color:#555;padding:2px 4px;";
    el.textContent = "all keys allowed";
    keyList.appendChild(el);
    return;
  }
  for (const key of whitelist) {
    const btn = document.createElement("button");
    btn.className = "key-tag";
    btn.textContent = key.toUpperCase();
    btn.title = "Click to remove";
    btn.addEventListener("click", () => {
      whitelist = whitelist.filter(k => k !== key);
      renderKeyList();
      markDirty();
    });
    keyList.appendChild(btn);
  }
}

function addToWhitelist(name) {
  if (name && !whitelist.includes(name)) {
    whitelist.push(name);
    renderKeyList();
    markDirty();
  }
}

let _kl = null, _ml = null, _wl = null;

function startListening() {
  isListening = true;
  addKeyBtn.textContent = "LISTENING...";
  addKeyBtn.classList.add("listening");

  _kl = (e) => {
    e.preventDefault(); e.stopPropagation();
    const name = CODE_TO_KEY[e.code];
    if (name) addToWhitelist(name);
    stopListening();
  };
  _ml = (e) => {
    if (e.target === addKeyBtn) return;
    e.preventDefault(); e.stopPropagation();
    const name = MOUSE_BTN_NAMES[e.button];
    if (name) addToWhitelist(name);
    stopListening();
  };
  _wl = (e) => {
    e.preventDefault();
    addToWhitelist("mouse_wheel");
    stopListening();
  };

  window.addEventListener("keydown", _kl, { capture: true });
  window.addEventListener("mousedown", _ml, { capture: true });
  window.addEventListener("wheel", _wl, { capture: true, passive: false });
}

function stopListening() {
  isListening = false;
  addKeyBtn.textContent = "ADD KEY";
  addKeyBtn.classList.remove("listening");
  if (_kl) { window.removeEventListener("keydown", _kl, { capture: true }); _kl = null; }
  if (_ml) { window.removeEventListener("mousedown", _ml, { capture: true }); _ml = null; }
  if (_wl) { window.removeEventListener("wheel", _wl, { capture: true }); _wl = null; }
}

addKeyBtn.addEventListener("click", () => {
  isListening ? stopListening() : startListening();
});

showBtn.addEventListener("click", () => {
  authEl.type = authEl.type === "password" ? "text" : "password";
});
copyBtn.addEventListener("click", () => {
  navigator.clipboard.writeText(authEl.value).then(() => showMsg("token copied"));
});
regenBtn.addEventListener("click", () => {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
  const arr = new Uint8Array(32);
  crypto.getRandomValues(arr);
  authEl.value = Array.from(arr).map(b => chars[b % chars.length]).join("");
  markDirty();
});

autostartEl.addEventListener("change", async () => {
  await invoke("set_autostart", { enabled: autostartEl.checked });
});

function applyHttpEnabled(enabled) {
  httpHostEl.disabled = !enabled;
  httpPortEl.disabled = !enabled;
  openHttpBtn.disabled = !enabled;
}

hostEl.addEventListener("input", () => { httpHostEl.value = hostEl.value; markDirty(); });
portEl.addEventListener("input", () => markDirty());
authEl.addEventListener("input", () => markDirty());
mouseMove.addEventListener("change", () => markDirty());
analogEl.addEventListener("change", () => markDirty());
httpPortEl.addEventListener("input", () => markDirty());

httpEnabledEl.addEventListener("change", async () => {
  const enabled = httpEnabledEl.checked;
  applyHttpEnabled(enabled);
  const host = hostEl.value.trim() || "localhost";
  const port = parseInt(httpPortEl.value, 10) || 4456;
  try {
    await invoke("toggle_http", { enabled, host, port });
  } catch (e) {
    showMsg(String(e), true);
  }
});

openHttpBtn.addEventListener("click", () => {
  const host = httpHostEl.value || "localhost";
  const port = parseInt(httpPortEl.value, 10) || 4456;
  invoke("open_url", { url: `http://${host}:${port}` });
});

document.getElementById("min-btn").addEventListener("click", () => {
  invoke("minimize_window");
});
document.getElementById("close-title-btn").addEventListener("click", () => {
  if (dirty) {
    discardModal.hidden = false;
  } else {
    invoke("close_window");
  }
});

const LINKS = {
  "link-github": "https://github.com/girlglock/input-overlay",
  "link-twitter": "https://twitter.com/girlglock_",
  "link-website": "https://girlglock.com",
};
for (const [id, url] of Object.entries(LINKS)) {
  document.getElementById(id)?.addEventListener("click", (e) => {
    e.preventDefault();
    invoke("open_url", { url });
  });
}

function showMsg(text, isError = false) {
  saveMsg.classList.remove("dirty-msg");
  saveMsg.textContent = text;
  saveMsg.style.color = isError ? "#bf5e5e" : "#003c74";
  if (_saveMsgTimer) clearTimeout(_saveMsgTimer);
  _saveMsgTimer = setTimeout(() => {
    _saveMsgTimer = null;
    if (dirty) showDirtyMsg();
    else { saveMsg.textContent = ""; saveMsg.style.color = ""; }
  }, 3000);
}

saveBtn.addEventListener("click", async () => {
  try {
    await invoke("save_config", { newCfg: readConfig() });
    markClean();
    await invoke("close_window");
  } catch (e) {
    showMsg(String(e), true);
  }
});

cancelBtn.addEventListener("click", () => {
  if (dirty) {
    discardModal.hidden = false;
  } else {
    invoke("close_window");
  }
});

modalKeepBtn.addEventListener("click", () => {
  discardModal.hidden = true;
});

modalDiscardBtn.addEventListener("click", async () => {
  discardModal.hidden = true;
  if (originalConfig) applyConfig(originalConfig);
  await invoke("close_window");
});

let _updateInfo = null;

function applyUpdateInfo(info) {
  if (!info || _updateInfo?.version === info.version) return;
  _updateInfo = info;
  updateText.textContent = `update available: v${info.version}`;
  updateNotesLink.onclick = (e) => { e.preventDefault(); invoke("open_url", { url: info.release_url }); };
  updateBar.hidden = false;

  const vl = document.getElementById("version-label");
  const cur = vl.dataset.version ?? "";
  vl.innerHTML = `Input-Overlay WebSocket Server | Version: ${cur}<br>`
    + `<a class="accent-link" id="update-version-link" href="#">→ v${info.version} available</a>`;
  document.getElementById("update-version-link")?.addEventListener("click", (e) => {
    e.preventDefault(); invoke("open_url", { url: info.release_url });
  });
}


updateNowBtn.addEventListener("click", async () => {
  if (!_updateInfo) return;
  updateProgressModal.hidden = false;
  updateProgressStatus.textContent = "starting...";
  updateProgressFill.value = 0;

  const unlisten = await listen("update-progress", (event) => {
    const { percent, status } = event.payload;
    updateProgressFill.value = percent;
    updateProgressStatus.textContent = status;
  });

  try {
    await invoke("apply_update", { downloadUrl: _updateInfo.download_url, version: _updateInfo.version });
  } catch (e) {
    unlisten();
    updateProgressModal.hidden = true;
    showMsg(`update failed: ${e}`, true);
  }
});

updateLaterBtn.addEventListener("click", () => {
  updateBar.hidden = true;
  _updateInfo = null;
});

async function initLinuxDevices(cfg) {
  try {
    const [keyboards, mice, perms] = await Promise.all([
      invoke("enum_keyboards"),
      invoke("enum_mice"),
      invoke("check_linux_perms"),
    ]);

    document.getElementById("autostart-label").textContent = "Start on login";
    document.getElementById("linux-devices").hidden = false;

    const kbdEl = document.getElementById("kbd-device");
    for (const [path, name] of keyboards) {
      const opt = document.createElement("option");
      opt.value = path;
      opt.textContent = name;
      kbdEl.appendChild(opt);
    }
    kbdEl.value = cfg.linux_evdev_keyboard_device ?? "";
    kbdEl.addEventListener("change", () => markDirty());

    const mouseDevEl = document.getElementById("mouse-device");
    for (const [path, name] of mice) {
      const opt = document.createElement("option");
      opt.value = path;
      opt.textContent = name;
      mouseDevEl.appendChild(opt);
    }
    mouseDevEl.value = cfg.linux_raw_mouse_device ?? "";
    mouseDevEl.addEventListener("change", () => markDirty());

    if (perms.length > 0) {
      const warn = document.getElementById("linux-perm-warning");
      warn.innerHTML = perms.map(msg =>
        `<span class="warn-line"><span class="warn-icon">&#9888;</span>${msg}</span>`
      ).join("");
      warn.hidden = false;
    }
  } catch (_) {
    // not on Linux or commands unavailable
  }
}

async function init() {
  try {
    const [cfg, status, autostart, admin, version] = await Promise.all([
      invoke("get_config"),
      invoke("get_status"),
      invoke("get_autostart"),
      invoke("is_admin"),
      getVersion(),
    ]);
    const vl = document.getElementById("version-label");
    vl.dataset.version = version;
    vl.innerHTML = `Input-Overlay WebSocket Server | Version: ${version}<br>(latest)`;
    document.getElementById("status-version").textContent = `v${version}`;
    applyThemeMode(cfg.theme ?? systemTheme());
    applyConfig(cfg);
    applyStatus(status);
    autostartEl.checked = autostart;
    if (!admin) {
      adminWarning.hidden = false;
      autostartEl.disabled = true;
      autostartEl.parentElement.title = "Admin rights required to create/remove the Scheduled Task";
    }
    await initLinuxDevices(cfg);
  } catch (e) {
    console.error("init error:", e);
  }

  listen("status-update", (e) => applyStatus(e.payload));
  listen("update-available", (e) => applyUpdateInfo(e.payload));
  invoke("check_update").then((info) => { if (info) applyUpdateInfo(info); }).catch(() => {});
}

window.addEventListener("DOMContentLoaded", init);
