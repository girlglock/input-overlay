<div align="center">

# input-overlay

**a keyboard/input overlay for OBS and other broadcast software**

[![releases](https://img.shields.io/github/v/release/girlglock/input-overlay?style=flat-square&color=c9a0dc&label=release)](https://github.com/girlglock/input-overlay/releases)
[![nightly](https://img.shields.io/badge/nightly-available-6a9fb5?style=flat-square)](https://nightly.link/girlglock/input-overlay/workflows/nightly/main/)
[![license](https://img.shields.io/github/license/girlglock/input-overlay?style=flat-square&color=aaa)](./LICENSE)

</div>

> [!NOTE]
> hello, this project is pretty much just something i made for myself and a few friends. im going to try to fix bugs etc as best as i can but keep in mind that im not that skilled of a dev so expect updates to roll out slowly or eventually to stop due to lack of time

---

## features

- **websocket connection with authentication** with support to stream your inputs to a secondary PC (e.g. a dedicated streaming PC)
- **hall effect keyboard support** via the [AnalogSense SDK py port](https://github.com/girlglock/AnalogSense-Python-SDK)
- **mouse movement tracking** via the RawInputBuffer windows api to keep track while tabbed into games
- **customizable layouts and labels** (labels support html `img src` tags, not officially though)
- **dual PC support** (e.g. gaming PC to dedicated streaming PC)

<table>
  <tr>
    <td><img src="https://files.catbox.moe/qzqhnc.avif" width="400"/></td>
    <td><img src="https://femboy.beauty/Ce9G3v" width="400"/></td>
  </tr>
</table>

---

## single PC setup

1. download the [`input-overlay-ws`](https://github.com/girlglock/input-overlay/releases) server
2. run it and right-click the tray icon to open settings
3. copy your auth token *(you can change it to whatever you like)*
4. paste the token in the auth token field in the configurator
5. configure your overlay to your liking, click **`⎘ copy url`**, and paste the copied url as an OBS browser source

<details>
   <summary>nightly builds</summary>
   <br>

   | platform | download |
   |----------|----------|
   | windows | [input-overlay-ws-windows.zip](https://nightly.link/girlglock/input-overlay/workflows/nightly/main/input-overlay-ws-windows.zip) |
   | linux | [input-overlay-ws-linux.zip](https://nightly.link/girlglock/input-overlay/workflows/nightly/main/input-overlay-ws-linux.zip) |

</details>

> [!TIP]
> you can configure the key whitelist in the server settings to ensure you're only sending keys over your network that are configured in the overlay

---

## sending keys to another PC

*(e.g. from gaming PC to streaming PC)*

1. run the `input-overlay-ws` server on your **gaming PC** and enable the HTTP server in its settings
2. find your gaming PC's local IP, run `ipconfig` in cmd and copy the IPv4 address (usually `192.168.X.X`)
3. click **`open in browser`** inside the HTTP server settings
4. enter the gaming PC's address in both the **input-overlay-ws** field and the hosted **configurator**
5. click **`⎘ copy url`** to copy the hosted overlay url and add it as a browser source in OBS on the streaming PC

---

## building from source

> [!NOTE]
> released binaries are already built via GitHub Actions, you only need this if you want to build from source yourself

**1. install dependencies**

```bash
python -m pip install --upgrade pip
pip install pyinstaller -r ws-server/requirements.txt
```

**2. build** (run from the `ws-server/` directory)

<details>
<summary><b>windows</b></summary>

```bash
python -m PyInstaller --onefile --windowed --add-data "assets;assets" --add-data "services;services" --add-data "../index.html;web" --add-data "../style.css;web" --add-data "../scripts;web/scripts" --icon="assets/icon.ico" --name="input-overlay-ws" --hidden-import=services.analog --hidden-import=services.consts --hidden-import=services.logger --hidden-import=services.utils --hidden-import=services.dialogs --hidden-import=services.settings --hidden-import=services.tray --hidden-import=services.rawinput --hidden-import=winotify --hidden-import=certifi --hidden-import=markdown --manifest admin.manifest input-overlay-ws.py
```

</details>

<details>
<summary><b>linux</b></summary>

```bash
python -m PyInstaller --onefile --windowed --add-data "assets:assets" --add-data "services:services" --add-data "../index.html:web" --add-data "../style.css:web" --add-data "../scripts:web/scripts" --icon="assets/icon.ico" --name="input-overlay-ws" --hidden-import=services.analog --hidden-import=services.consts --hidden-import=services.logger --hidden-import=services.utils --hidden-import=services.dialogs --hidden-import=services.settings --hidden-import=services.tray --hidden-import=certifi --hidden-import=markdown input-overlay-ws.py
```

</details>

output will be inside `dist/input-overlay-ws/`