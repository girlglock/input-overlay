# input-overlay (standalone)

A desktop input overlay for Windows. Install it, run it, and the overlay shows up on screen. No OBS, no browser source, and no website.

Based on [girlglock/input-overlay](https://github.com/girlglock/input-overlay).

## install

1. Download `Input Overlay_<version>_x64-setup.exe` from the [installer workflow](.github/workflows/installer.yml) artifacts, or from the release page for a tagged version.
2. Run the installer. It installs for the current user only, so no admin prompt is needed.
3. The overlay starts right away as a transparent, always-on-top window over your primary monitor. It ignores mouse clicks, so it never blocks the desktop or your games.
4. The tray icon opens the settings window (server, auth token, analog keyboard, key whitelist). Tray → Exit quits.

## what it does

- Reads keyboard and mouse input through the Windows RawInput API, so it keeps working while tabbed into games.
- Hall-effect (analog) keyboard support, see the settings window.
- Runs a local websocket server on `localhost:4455` with an auth token. The overlay inside the app connects to it automatically.
- Ships with a built-in default layout (a keyboard with a mouse pad).

## build the installer

The installer is built on a Windows runner. Run the `windows installer` workflow manually, or push a `v*` tag. The `.exe` setup file is attached to the workflow run and, for tags, to the release.

To build locally on Windows:

```powershell
cargo install tauri-cli --version "^2" --locked
cd ws-server\src-tauri
cargo tauri build --bundles nsis
```

## changes from upstream

- no OBS browser source and no hosted configurator or website
- no local HTTP server
- no built-in updater (it would fetch upstream builds)
- the overlay runs inside the app as a native window, and the layout is built in

## license

See [LICENSE](./LICENSE).
