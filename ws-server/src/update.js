document.addEventListener('contextmenu', e => e.preventDefault());

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let updateInfo = null;

const versionText = document.getElementById("update-version-text");
const notesEl = document.getElementById("update-notes");
const updateNowBtn = document.getElementById("update-now-btn");
const updateSkipBtn = document.getElementById("update-skip-btn");
const updateLaterBtn = document.getElementById("update-later-btn");
const notesLink = document.getElementById("update-notes-link");
const progressModal = document.getElementById("update-progress-modal");
const progressStatus = document.getElementById("update-progress-status");
const progressFill = document.getElementById("update-progress-fill");

document.getElementById("close-btn").addEventListener("click", () => invoke("close_window"));

notesLink.addEventListener("click", (e) => {
  e.preventDefault();
  if (updateInfo) invoke("open_url", { url: updateInfo.release_url });
});

updateLaterBtn.addEventListener("click", () => invoke("close_window"));
updateSkipBtn.addEventListener("click", async () => {
  if (updateInfo) {
    try { await invoke("dismiss_update", { version: updateInfo.version }); } catch (_) { }
  }
  invoke("close_window");
});

updateNowBtn.addEventListener("click", async () => {
  if (!updateInfo) return;
  progressModal.hidden = false;
  progressStatus.textContent = "starting...";
  progressFill.value = 0;

  const unlisten = await listen("update-progress", (event) => {
    const { percent, status } = event.payload;
    progressFill.value = percent;
    progressStatus.textContent = status;
  });

  try {
    await invoke("apply_update", { downloadUrl: updateInfo.download_url });
  } catch (e) {
    unlisten();
    progressModal.hidden = true;
    progressStatus.textContent = "";
    versionText.textContent = `update failed: ${e}`;
    versionText.style.color = "#ff7070";
  }
});

async function init() {
  try {
    const [info, canUpdate] = await Promise.all([
      invoke("check_update"),
      invoke("can_auto_update").catch(() => false),
    ]);
    if (!info) { invoke("close_window"); return; }
    updateInfo = info;
    versionText.textContent = `v${info.version} is now available`;
    const md = info.body?.trim() || "(no release notes)";
    notesEl.innerHTML = marked.parse(md);
    if (!canUpdate) updateNowBtn.hidden = true;
  } catch (e) {
    console.error("update popup init:", e);
    invoke("close_window");
  }
}

window.addEventListener("DOMContentLoaded", init);
