## windows

<details>
<summary><b>obs plugin setup</b></summary>
<br>

1. download `input-overlay-obs-plugin-windows.zip` and unzip it
2. merge the `obs-studio` folder into `C:\ProgramData\` *(the plugin ends up in `C:\ProgramData\obs-studio\plugins\input_overlay_obs_plugin\`)*

   if you installed an older version into `C:\Program Files\obs-studio\`, delete `obs-plugins\64bit\input_overlay_obs_plugin.dll` and `obs-plugins\64bit\input-overlay-fonts` from there, otherwise obs shows it as legacy
3. launch obs with admin
4. add a new source via **Add Source -> Input Overlay**, then configure it in its properties

</details>

<details>
<summary><b>standalone ws server setup</b></summary>
<br>

1. download `input-overlay-ws-windows.zip`
2. unzip and run `input-overlay-ws.exe`
3. right-click the tray icon for settings

> [!NOTE]
> you might need to run the ws server as admin when tabbed into apps that run with admin elevations themselves
> the "start with windows" checkbox will automatically start the ws server with admin rights on the next boot

</details>
