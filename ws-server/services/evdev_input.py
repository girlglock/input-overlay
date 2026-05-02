from __future__ import annotations

import logging
import select
import threading
import time
from typing import Callable

from services.consts import HID_TO_VK, _EVDEV_TO_HID

logger = logging.getLogger(__name__)

_EVDEV_BTN_LEFT   = 0x110  #BTN_LEFT
_EVDEV_BTN_RIGHT  = 0x111  #BTN_RIGHT
_EVDEV_BTN_MIDDLE = 0x112  #BTN_MIDDLE
_EVDEV_BTN_SIDE   = 0x113  #BTN_SIDE   (button 4)
_EVDEV_BTN_EXTRA  = 0x114  #BTN_EXTRA  (button 5)

_EVDEV_BTN_TO_CODE: dict[int, int] = {
    _EVDEV_BTN_LEFT:   1,
    _EVDEV_BTN_RIGHT:  2,
    _EVDEV_BTN_MIDDLE: 3,
    _EVDEV_BTN_SIDE:   4,
    _EVDEV_BTN_EXTRA:  5,
}

_REL_WHEEL = 8   #vertical scroll
RESCAN_INTERVAL = 5.0

class EvdevInputListener(threading.Thread):
    def __init__(
        self,
        on_key_press:    Callable[[int], None],
        on_key_release:  Callable[[int], None],
        on_mouse_click:  Callable[[int, bool], None],
        on_mouse_scroll: Callable[[int], None],
    ) -> None:
        super().__init__(daemon=True, name="EvdevInputListener")
        self._on_key_press    = on_key_press
        self._on_key_release  = on_key_release
        self._on_mouse_click  = on_mouse_click
        self._on_mouse_scroll = on_mouse_scroll
        self._stop_event      = threading.Event()

    def stop(self) -> None:
        self._stop_event.set()
        self.join(timeout=3.0)

    def run(self) -> None:
        try:
            import evdev  #PLC0415
        except ImportError:
            logger.error("evdev is not there")
            return

        logger.info("evdev listener starting")
        devices: dict[str, evdev.InputDevice] = {}
        last_scan = 0.0

        while not self._stop_event.is_set():
            now = time.monotonic()
            if now - last_scan >= RESCAN_INTERVAL:
                self._refresh_devices(evdev, devices)
                if last_scan == 0.0:
                    logger.info("evdev: %d devices opened", len(devices))
                    if not devices:
                        logger.warning("evdev: no devices")
                last_scan = now

            if not devices:
                time.sleep(0.5)
                continue

            fds = {dev.fd: dev for dev in devices.values()}
            try:
                readable, _, _ = select.select(list(fds.keys()), [], [], 0.5)
            except (ValueError, OSError):
                self._refresh_devices(evdev, devices)
                last_scan = time.monotonic()
                continue

            for fd in readable:
                dev = fds.get(fd)
                if dev is None:
                    continue
                try:
                    for event in dev.read():
                        self._dispatch(event)
                except OSError:
                    path = dev.path
                    logger.info("evdev: device removed: %s", path)
                    try:
                        dev.close()
                    except Exception:
                        pass
                    devices.pop(path, None)

        for dev in list(devices.values()):
            try:
                dev.close()
            except Exception:
                pass
        logger.info("evdev listener stopped")

    @staticmethod
    def _refresh_devices(evdev_mod, devices: dict) -> None:
        try:
            current_paths = set(evdev_mod.list_devices())
        except Exception as e:
            logger.debug("evdev: list_devices error: %s", e)
            return

        for path in list(devices):
            if path not in current_paths:
                logger.info("evdev: removing exploded device %s", path)
                try:
                    devices[path].close()
                except Exception:
                    pass
                del devices[path]

        for path in current_paths:
            if path in devices:
                continue
            try:
                dev = evdev_mod.InputDevice(path)
                caps = dev.capabilities()
                ev_key  = evdev_mod.ecodes.EV_KEY
                ev_rel  = evdev_mod.ecodes.EV_REL
                has_keys   = ev_key in caps
                has_scroll = ev_rel in caps

                if not (has_keys or has_scroll):
                    dev.close()
                    continue

                keys = caps.get(ev_key, [])
                is_keyboard = evdev_mod.ecodes.KEY_A in keys
                is_mouse    = evdev_mod.ecodes.BTN_LEFT in keys

                if is_keyboard or is_mouse:
                    devices[path] = dev
                    logger.info("evdev: opened %s (%s): kbd=%s mouse=%s",
                                path, dev.name, is_keyboard, is_mouse)
                else:
                    dev.close()
            except PermissionError:
                logger.warning("evdev: no permissions to open devices, do sudo usermod -aG input $USER or run with sudo", path)
            except Exception as e:
                logger.debug("evdev: couldnt open %s: %s", path, e)

    def _dispatch(self, event) -> None:
        try:
            import evdev  #PLC0415
            EV_KEY = evdev.ecodes.EV_KEY
            EV_REL = evdev.ecodes.EV_REL
        except ImportError:
            return

        if event.type == EV_KEY:
            code    = event.code
            value   = event.value   #1=press  0=release  2=repeat
            pressed = value in (1, 2)

            #see if mousebuttin
            btn_code = _EVDEV_BTN_TO_CODE.get(code)
            if btn_code is not None:
                if value in (0, 1):
                    try:
                        self._on_mouse_click(btn_code, value == 1)
                    except Exception:
                        self._on_mouse_click(btn_code, false)
                        logger.exception("evdev: on_mouse_click error")
                return

            #see if key
            hid = _EVDEV_TO_HID.get(code)
            if hid is None:
                return
            vk = HID_TO_VK.get(hid)
            if vk is None:
                return
            try:
                if pressed:
                    self._on_key_press(vk)
                elif value == 0:
                    self._on_key_release(vk)
            except Exception:
                self._on_key_release(vk)
                logger.exception("evdev: on_key_press/release error")

        elif event.type == EV_REL:
            if event.code == _REL_WHEEL:
                #evdev: +=up -=down
                rotation = -1 if event.value > 0 else 1 if event.value < 0 else 0
                if rotation:
                    try:
                        self._on_mouse_scroll(rotation)
                    except Exception:
                        logger.exception("evdev: on_mouse_scroll error")