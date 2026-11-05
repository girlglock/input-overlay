<div align="center">*falls over and dies*<br><img src='https://cdn.7tv.app/emote/01KVQPM44RTRPF4GBB6666VKS5/4x.avif'></div>

<details>
<summary><b>configurator</b></summary>

- added a file picker when the overlay is hosted locally via http server which lets you pick served files from the `user_mount` dir
- added a "welcome popup" that asks you if you wanna use the web or obs plugin version (read more below)
- added a new option to load and export the overlay config from json file (mainly used in the obs plugin version)

</details>

<details>
<summary><b>overlay</b></summary>

- added iso layout pipe bomb button
- added a new element: input_history can be configured to show any amount of keys in a piano tiles style history in both vertical and horizontal layout. it can also highlight any key overlap.
<img src='https://femboy.beauty/2jaoaT' width=200px>

- added a bool to key and mouse elements: move to top on active, which will show the element at the very top of the order once active

</details>

<details>
<summary><b>new pawsome obs plugin</b></summary>

  - this plugin handles both inputs and rendering. it uses a 2D renderer rather than CEF (browser source) to display the overlay inside your OBS.
  - cons of using it
     - it wont include several features like ws server, cpu affinity, flush hz, http server for dual pc support
     - requires you to restart obs in order to change some settings
     - needs obs to be ran as admin on bimbows
     - no updater
  - pros
    - its convenient to not need to run a separate app beside your obs
    - only runs when obs runs
    - minimizes the performance hit from a traditional browser source
    - minimizes the input to overlay latency of the websocket round trip (the web based version tends to display the inputs a frame or so later)
       - (not your actual input latency ie input -> game, this should be unaffected when using either version of the overlay since we are draining the rawinputbuffer rather than intercepting the inputs)
 - you can either load the overlay config from a exported json file or directly from a link similar to a browser source (pre 1.8.0 links are not supported!)
 - the obs plugin version should have feature parity between the web version, except for:
   - its not being able to use mp4 files as the mouse_pad background. support for gifs has been added to both plugin and web version
   - glow is being rendered quite differently. in order to optimize glow render time, instead of rendering it in a conventional way by using a blur i decided to count all key elements by scale and pre compute a glow texture for each key scale (eg if you overlay has 10 1x1 keys and 5 2x1 keys we compute 2 glow textures for 1x1 and 2x1) and reuse them for the keys rather than rendering expensive blur based glow in real time. this improves render time by a lot
   - some fonts (arial pixel and borevs) needed to be substituted

<img src='https://femboy.beauty/EhQbqy' width=400px>

</details>

> [!IMPORTANT]  
> If you choose to try the plugin version make sure to close the standalone app version. (disable autostart if you wanna do a full switch)

<details>
<summary><b>ws server</b></summary>

**general**
- ditched webview and webkit2gtk dependencies
- added `user_mount` directory where you can add your own files to server whilst using the http server feature. this includes hosting custom http overlays or image/video assets
- fixed an oversight that caused a analog keyboard to never be picked up again by the app after it got disconnected whilst the app was already running
- fixed logs being spammed after said analog keyboard was disconnected
- added an override for the flush interval ws events which you can access from the advanced tab
- ws events now carry a timestamp of when the input has happened
- fixed "enable mouse movement" checkbox not taking effect when unchecked and the whitelist is empty
- added -notray param which when passed wont spawn any tray icon and open the settings window on start. the ws-server will depend on the settings window being open.. when closed the ws-server will shut down. closes #41

**michaelsoft bimbows**
- added os build checks to better distinguish between w10 and w11
- on w11 and supported w10 builds the app will now set its process timer resolution
- added cpu affinity to advanced tab
- fixed right ctrl and right alt vk codes closes #37 
- fixed phantom ctrl when pressing right alt closes #37 

**linus**
- the ws-server is now only released as an appimage, the plain binary zip is gone. its dependencies are small enough now that they dont bloat the appimage anymore after ditching webkit2gtk #34

</details>
