## linux

<details>
<summary><b>needed perms and rules</b></summary>
<br>

these steps let input-overlay read your keyboard/mouse devices. do this before using either setup method below.

1. add yourself to the `input` group

   ```bash
   sudo usermod -aG input $USER
   ```

   if the one above fails you might be on a immutable distro (bazzite, bluefin, openSUSE Aeon, etc etc). try this then:

   ```bash
   echo "input:x:$(getent group input | cut -d: -f3):$USER" | sudo tee -a /etc/group
   ```

2. set up analog support *(optional)*:

   ```bash
   printf '%s\n' \
     'KERNEL=="hidraw*", SUBSYSTEM=="hidraw", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="31e3", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="03eb", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="1532", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="19f5", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="352d", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="3434", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="362d", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="373b", MODE="0660", GROUP="input"' \
     'SUBSYSTEM=="usb", ATTRS{idVendor}=="372e", MODE="0660", GROUP="input"' \
     | sudo tee /etc/udev/rules.d/99-analog.rules
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```

3. reboot

</details>

<details>
<summary><b>obs plugin setup</b></summary>
<br>

> [!IMPORTANT]
> the obs plugin version does **not** include all features, will **not** receive automatic update checks, and is only intended for **single pc setups** (obs and the game running on the same peesee)

1. download `input-overlay-obs-plugin-linux.zip` and unzip it
2. merge the directory into your obs-studio install, eg `~/.config/obs-studio/` *(or whereever your obs folder is :p)*
3. launch obs, add a new source via `Add Source -> Input Overlay`, then configure it in its properties

</details>

<details>
<summary><b>standalone ws server setup (experimental)</b></summary>
<br>

download `input-overlay-ws-linux.zip` and unzip it, then follow the steps below.

1. install runtime dependencies:

   ```bash
   # debian / ubuntu
   sudo apt-get install libwebkit2gtk-4.1-0 libgtk-3-0 libayatana-appindicator3-1 librsvg2-2

   # fedora
   sudo dnf install webkit2gtk4.1 gtk3 libayatana-appindicator librsvg2

   # arch
   sudo pacman -S webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg
   ```

   an appimage build is also available which should run on most distros without installing dependencies at the cost of being hughmungous in comparison

2. run `./input-overlay-ws`
3. right-click the tray icon and open settings -> devices to pick your keyboard and mouse evdev devices

   some mice and keyboards might show up duplicate (nkro, wireless dongle, keyboard mouse button emulation for macros, mouse keyboard macros etc etc), you will just kinda have to try and error them)

</details>
