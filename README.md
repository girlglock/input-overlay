# Input-Overlay

A input overlay for OBS and other broadcast software.

> [!NOTE]  
> hello, this project is pretty much just something i made for myself and a few friends,.. im going to try to fix bug etc as best as i can but keep in mind that im not that skilled of a dev so expect updates to roll out slowly or eventually to stop due to lack of time

## Features

- **WebSocket connection with authentication** with support to stream your inputs to a secondary PC (for example a dedicated streaming PC)
- **Hall effect keyboard support** via the [AnalogSense SDK](https://github.com/AnalogSense/JavaScript-SDK) ported to py
  <details>
  <summary>Keyboards/Devices</summary>

  - Everything by Wooting
  - Everything by NuPhy
  - Everything by DrunkDeer
  - Razer Huntsman V2 Analog<sup>R</sup>
  - Razer Huntsman Mini Analog<sup>R</sup>
  - Razer Huntsman V3 Pro<sup>R</sup>
  - Razer Huntsman V3 Pro Mini<sup>R</sup>
  - Razer Huntsman V3 Pro Tenkeyless<sup>R</sup>
  - Keychron Q1 HE<sup>P, F</sup>
  - Keychron Q3 HE<sup>P, F</sup>
  - Keychron Q5 HE<sup>P, F</sup>
  - Keychron K2 HE<sup>P, F</sup>
  - Lemokey P1 HE<sup>P, F</sup>
  - Madlions MAD60HE<sup>P</sup>
  - Madlions MAD68HE<sup>P</sup>
  - Madlions MAD68R<sup>P</sup>
  - Redragon K709HE<sup>P</sup>

  <sup>R</sup> Razer Synapse needs to be installed and running for analogue inputs to be received from this keyboard.  
  <sup>P</sup> The official firmware only supports polling, which can lead to lag and missed inputs.  
  <sup>F</sup> [Custom firmware with full analog report functionality is available](https://analogsense.org/firmware/).

  **Tested devices:**
  - Wooting 60HE
  - Redragon K709HE

  All other devices are theoretical and have not been tested. If you have a one of these devices and it works please open a pr to update the readme (or if its brokey open an issue or pr with the fix)
  </details>
- **Customizable layouts and lables** (lables support html img src tags although not officialy)

<table>
  <tr>
    <td><img src="https://files.catbox.moe/qzqhnc.avif" width="400"/></td>
    <td><img src="https://femboy.beauty/MKZmJt.png" width="400"/></td>
  </tr>
</table>

## Single PC Setup
<ol>
    <li>get the <a href="https://github.com/girlglock/input-overlay/releases"
            target="_blank">input-overlay-ws</a> server</li>
    <li>run it and right click the tray icon to open the settings</li>
    <li>copy your auth token</li>
    <blockquote class="note note--purple">(you can change it to whatever you like)
    </blockquote>
    <li>paste the token in the auth token field above</li>
    <li>configure your overlay to your liking, then click the <code>⎘ copy url</code> button
        and paste the copied url as an OBS
        browser source</li>
</ol>
<blockquote class="note note--purple">tip: you can configure the key whitelist in the
    server settings to ensure you
    are only sending keys over your network that are configured in the overlay
</blockquote>
 
## Sending Keys to Another PC
<blockquote class="note note--purple">(eg. from gaming to streaming pc)</blockquote>
<ol>
    <li>open the input-overlay-ws server settings and enable the http ser(run
            the ws server on your gaming pc)</small></li>
    <li>change the address in to the address of your gaming pc
        <br><small>run <code>ipconfig</code> in cmd and copy the local IPv4
            address (usually 192.168.0.1 or 192.168.X.X with X being 0-255)</small>
    </li>
    <li>click the <code>open in browser</code> button inside the http serli>
    <li>enter the gaming pc's address in both the <strong>input-overlay-wsthe
        hosted
        <strong>configurator</strong>
    </li>
    <li>click the <code>⎘ copy url</code> button to copy your hosted overlay url from the
        hosted configurator and add it as a browser source in your OBS running on the
        streaming pc</li>
</ol>

## Building the input-overlay-ws server

> [!NOTE]
> the released binaries are built via GitHub Actions already...
> you only need this if you want to build from source yourself

**1. install dependencies**

```bash
python -m pip install --upgrade pip
pip install pyinstaller -r ws-server/requirements.txt
```

**2. build** (run from the `ws-server/` dir)

Windows:
```bash
python -m PyInstaller --onedir --windowed --noupx \
  --add-data "assets;assets" \
  --add-data "services;services" \
  --add-data "../index.html;web" \
  --add-data "../style.css;web" \
  --add-data "../scripts;web/scripts" \
  --icon="assets/icon.ico" \
  --name="input-overlay-ws" \
  --hidden-import=services.analog \
  --hidden-import=services.consts \
  --hidden-import=services.logger \
  --hidden-import=services.utils \
  --hidden-import=services.dialogs \
  --hidden-import=services.settings \
  --hidden-import=services.tray \
  --hidden-import=services.rawinput \
  --hidden-import=winotify \
  --hidden-import=certifi \
  --hidden-import=markdown \
  --manifest admin.manifest \
  input-overlay-ws.py
```

Linux:
```bash
python -m PyInstaller --onedir --windowed --noupx \
  --add-data "assets:assets" \
  --add-data "services:services" \
  --add-data "../index.html:web" \
  --add-data "../style.css:web" \
  --add-data "../scripts:web/scripts" \
  --icon="assets/icon.ico" \
  --name="input-overlay-ws" \
  --hidden-import=services.analog \
  --hidden-import=services.consts \
  --hidden-import=services.logger \
  --hidden-import=services.utils \
  --hidden-import=services.dialogs \
  --hidden-import=services.settings \
  --hidden-import=services.tray \
  --hidden-import=certifi \
  --hidden-import=markdown \
  input-overlay-ws.py
```

the output will be inside `dist/input-overlay-ws/`