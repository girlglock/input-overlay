<div align="center">

# input-overlay

**Input overlay for OBS using RawInputBuffer with analog keyboard support**

[![releases](https://img.shields.io/github/v/release/girlglock/input-overlay?style=flat-square&color=c9a0dc&label=release)](https://github.com/girlglock/input-overlay/releases)
[![nightly](https://img.shields.io/badge/nightly-available-6a9fb5?style=flat-square)](https://nightly.link/girlglock/input-overlay/workflows/nightly/main)
[![license](https://img.shields.io/github/license/girlglock/input-overlay?style=flat-square&color=aaa)](./LICENSE)

</div>

> [!NOTE]
> hello, this project is pretty much just something i made for myself and a few friends. im going to try to fix bugs etc as best as i can but keep in mind that im not that skilled of a dev so expect updates to roll out slowly or eventually to stop due to lack of time
>
> you can read up more on how to create your very own HTML overlay that uses the desktop app [here](https://github.com/girlglock/input-overlay/wiki/Creating-your-own-Overlay)

---

## features

- **websocket connection with authentication** with support to stream your inputs to a secondary PC (e.g. a dedicated streaming PC)
- **hall effect keyboard support** via the [AnalogSense SDK ported to rust](https://github.com/AnalogSense/JavaScript-SDK)
- **mouse movement tracking** via the RawInputBuffer windows api to keep track while tabbed into games
- **customizable layouts and labels** (labels support html `img src` tags, not officially though)
- **dual PC support** (e.g. gaming PC to dedicated streaming PC)

<table>
  <tr>
    <td><img src="https://files.catbox.moe/qzqhnc.avif" width="400"/></td>
    <td><img src="https://femboy.beauty/AvwAox.png" width="400"/></td>
    <td><img src="https://femboy.beauty/B2tyyA.png" width="400"/></td>
  </tr>
</table>

---

## single PC setup

1. get the [`input-overlay-ws`](https://github.com/girlglock/input-overlay/releases) server and follow the setup instructions from its release page
2. after its set up, go into its settings
   > standalone app: right-click the tray icon -> settings
   
   > obs plugin: Tools -> Input Overlay WS Settings
3. copy your auth token from the settings
   > you can change it to whatever you like in there too
4. paste the auth token in the auth token field above
5. configure your overlay to your liking, then once done click the `⎘ copy url` button and paste the copied url as an OBS browser source

> tip: you can configure the key whitelist in the server settings to ensure you are only sending keys over your network that are configured in the overlay

<details>
   <summary>nightly builds</summary>
   <br>

   | platform | download |
   |----------|----------|
   | windows | [input-overlay-ws-windows.zip](https://nightly.link/girlglock/input-overlay/workflows/nightly/main/input-overlay-ws-windows.zip) |
   | linux | [input-overlay-ws-linux.zip](https://nightly.link/girlglock/input-overlay/workflows/nightly/main/input-overlay-ws-linux.zip) |

</details>

---

## sending keys to another pc

*(eg. from gaming to streaming pc)*

> this will not work with the obs-plugin version of the input-overlay-ws! you will have to use the standalone app version!

1. open the input-overlay-ws server **on your gaming pc**
2. right-click the tray icon -> settings -> enable the http server
3. change the host of the ws server to the address of your gaming pc
   > run `ipconfig` in cmd and copy the local IPv4 address
   
   > (usually 192.168.0.1 or 192.168.X.X with X being 0-255)
4. click the `open in browser` button inside the http server settings
5. enter the gaming pc's address in both the **input-overlay-ws** and the hosted **configurator**
6. click the `⎘ copy url` button to copy your hosted overlay url from the hosted configurator and add it as a browser source in your OBS running on the streaming pc

> **cant connect from another pc?** Windows Firewall might block inbound connections by default. Run this in PowerShell **as Administrator** on your gaming pc to allow the ws server ports:
> ```powershell
> New-NetFirewallRule -DisplayName "input-overlay WS" -Direction Inbound -Protocol TCP -LocalPort 4455 -Action Allow
> New-NetFirewallRule -DisplayName "input-overlay HTTP" -Direction Inbound -Protocol TCP -LocalPort 4456 -Action Allow
> ```
> adjust the port numbers if you changed them from the defaults

---

## building from source

> [!NOTE]
> released binaries are already built via GitHub Actions, you only need this if you want to build from source yourself

**1. install prerequisites**

- [Rust stable toolchain](https://rustup.rs)

<details>
<summary><b>linux additional dependencies</b></summary>

```bash
sudo apt install libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libudev-dev
```

</details>

**2. build** (run from the `ws-server/` directory)

<details>
<summary><b>both standalone and plugin</b></summary>

```bash
cargo build --release
```

outputs:
- `target/release/input-overlay-ws` (or `.exe` on bimbows) standalone app
- `target/release/input_overlay_ws_server.dll/.so` obs plugin

</details>

<details>
<summary><b>standalone app only</b></summary>

```bash
cargo build --release --package input-overlay-ws
```

output: `target/release/input-overlay-ws` (or with `.exe` on bimbows)

</details>

<details>
<summary><b>obs plugin only</b></summary>

```bash
cargo build --release --package input-overlay-ws-server-plugin
```

output: `target/release/input_overlay_ws_server.dll/.so`

place the output file in your obs-studio plugins folder: `obs-studio/obs-plugins/64bit/`

</details>