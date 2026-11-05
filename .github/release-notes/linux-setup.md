## linux

<details>
<summary><b>needed perms and rules</b></summary>
<br>

these steps let input-overlay read your keyboard and mouse. do this before using either setup method below.

instead of adding yourself to the `input` group (which gives access to **every** input device: gamepads, touchscreens, power buttons, etc), you make a dedicated `input-overlay` group and give it access to only your keyboards and mice.

1. create the group and add yourself to it

   ```bash
   sudo groupadd --system input-overlay
   sudo usermod -aG input-overlay $USER
   ```

2. give the group read-only access to keyboards and mice

   ```bash
   printf '%s\n' \
     'SUBSYSTEM=="input", KERNEL=="event*", ENV{ID_INPUT_KEYBOARD}=="1", GROUP="input-overlay", MODE="0640"' \
     'SUBSYSTEM=="input", KERNEL=="event*", ENV{ID_INPUT_MOUSE}=="1", GROUP="input-overlay", MODE="0640"' \
     | sudo tee /etc/udev/rules.d/99-input-overlay.rules
   ```

3. set up analog support *(optional)*

   only matches the analog keyboard brands input-overlay supports. these need read **and** write (`0660`), because input-overlay sends requests to the keyboard to get the analog values.

   ```bash
   printf '%s\n' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="31e3", GROUP="input-overlay", MODE="0660"' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="03eb", GROUP="input-overlay", MODE="0660"' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1532", GROUP="input-overlay", MODE="0660"' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="19f5", GROUP="input-overlay", MODE="0660"' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="352d", GROUP="input-overlay", MODE="0660"' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3434", GROUP="input-overlay", MODE="0660"' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="362d", GROUP="input-overlay", MODE="0660"' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="373b", GROUP="input-overlay", MODE="0660"' \
     'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="372e", GROUP="input-overlay", MODE="0660"' \
     | sudo tee /etc/udev/rules.d/99-input-overlay-analog.rules
   ```

4. reboot

5. check if worked: `ls -l /dev/input/event*` should show `input-overlay` as the group on your keyboard and mouse's

<details>
<summary>coming from an older version?</summary>

older setup steps added you to the `input` group and used a broader analog rule. you can undo those:

```bash
sudo gpasswd -d $USER input
sudo rm /etc/udev/rules.d/99-analog.rules
```

</details>

<details>
<summary>how to only allow specific devices?</summary>

to limit it to specific devices, match on their ids instead. find the ids with `udevadm info /dev/input/by-id/your_device_id` (look for `ID_VENDOR_ID` and `ID_MODEL_ID`) then use rules like this in `/etc/udev/rules.d/99-input-overlay.rules`:

```
SUBSYSTEM=="input", KERNEL=="event*", ATTRS{idVendor}=="xxxx", ATTRS{idProduct}=="xxxx", GROUP="input-overlay", MODE="0640"
```

`udevadm verify` is usefull to verify the rules file, and to debug a rule you can use `udevadm test /dev/input/eventX`... `evtest` helps to find out which event device actually reacts etc

</details>

</details>

<details>
<summary><b>obs plugin setup</b></summary>
<br>

> [!IMPORTANT]
> the obs plugin version does **not** include all features, will **not** receive automatic update checks, and is only intended for **single pc setups** (obs and the game running on the same peesee)

1. download `input-overlay-obs-plugin-linux.zip` and unzip it
2. merge the `obs-studio` folder into `~/.local/share/` *(the plugin ends up in `~/.local/share/obs-studio/plugins/input_overlay_obs_plugin/`)*

   for the flatpak version of obs merge it into `~/.var/app/com.obsproject.Studio/data/` instead

   if you installed an older version into `~/.config/obs-studio/plugins/`, delete it from there, otherwise obs shows it as legacy
3. launch obs, add a new source via `Add Source -> Input Overlay`, then configure it in its properties

</details>

<details>
<summary><b>standalone ws server setup (experimental)</b></summary>
<br>

download `input-overlay-ws-linux.zip` and unzip it, then follow the steps below.

1. the dependencies are X11/XWayland, OpenGL and fontconfig. the tray icon uses StatusNotifierItem, so with GNOME you will need the AppIndicator extension to see it.. on DEs without any tray support, run `./input-overlay-ws --notray`

   an appimage build is also there

2. run `./input-overlay-ws`
3. right-click the tray icon and open settings -> devices to pick your keyboard and mouse evdev devices

   some mice and keyboards might show up duplicate (nkro, wireless dongle, keyboard mouse button emulation for macros, mouse keyboard macros etc etc), you will just kinda have to try and error them)

</details>
