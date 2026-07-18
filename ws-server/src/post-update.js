document.addEventListener('contextmenu', e => e.preventDefault());

const { invoke } = window.__TAURI__.core;

const zuneLink = document.querySelector('link[href*="XP-ZUNE"]');
const systemTheme = () => window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
function applyThemeMode(mode) {
  document.documentElement.setAttribute('data-theme', mode);
  zuneLink.media = mode === 'dark' ? 'all' : 'not all';
}
applyThemeMode(systemTheme());

document.getElementById("close-btn").addEventListener("click", () => invoke("close_window"));
document.getElementById("ok-btn").addEventListener("click", () => invoke("close_window"));

async function init() {
  try {
    const [version, cfg] = await Promise.all([
      invoke("get_post_update_version"),
      invoke("get_config").catch(() => null),
    ]);
    applyThemeMode(cfg?.theme ?? systemTheme());
    document.getElementById("update-done-text").textContent =
      version ? `Successfully updated to v${version}!` : "Update complete!";
  } catch (e) {
    console.error("post-update init:", e);
    invoke("close_window");
  }
}

window.addEventListener("DOMContentLoaded", init);
