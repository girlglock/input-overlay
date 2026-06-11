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


def enum_evdev_keyboards() -> list[dict]:
    try:
        import evdev  #PLC0415
    except ImportError:
        return []
    results = []
    try:
        for path in evdev.list_devices():
            try:
                dev  = evdev.InputDevice(path)
                caps = dev.capabilities()
                keys = caps.get(evdev.ecodes.EV_KEY, [])
                if evdev.ecodes.KEY_A in keys:
                    results.append({"path": path, "name": dev.name, "phys": getattr(dev, "phys", "")})
                dev.close()
            except Exception:
                pass
    except Exception as e:
        logger.debug("enum_evdev_keyboards error: %s", e)
    return results


def enum_evdev_mice() -> list[dict]:
    try:
        import evdev  #PLC0415
    except ImportError:
        return []
    results = []
    try:
        for path in evdev.list_devices():
            try:
                dev  = evdev.InputDevice(path)
                caps = dev.capabilities()
                keys = caps.get(evdev.ecodes.EV_KEY, [])
                if evdev.ecodes.BTN_LEFT in keys:
                    results.append({"path": path, "name": dev.name, "phys": getattr(dev, "phys", "")})
                dev.close()
            except Exception:
                pass
    except Exception as e:
        logger.debug("enum_evdev_mice error: %s", e)
    return results


class EvdevInputListener(threading.Thread):
    def __init__(
        self,
        on_key_press:         Callable[[int], None],
        on_key_release:       Callable[[int], None],
        on_mouse_click:       Callable[[int, bool], None],
        on_mouse_scroll:      Callable[[int], None],
        keyboard_device_path: str  = "",
        capture_all:          bool = False,
    ) -> None:
        super().__init__(daemon=True, name="EvdevInputListener")
        self._on_key_press          = on_key_press
        self._on_key_release        = on_key_release
        self._on_mouse_click        = on_mouse_click
        self._on_mouse_scroll       = on_mouse_scroll
        self._keyboard_device_path  = keyboard_device_path
        self._capture_all           = capture_all
        self._stop_event            = threading.Event()

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
        devices = self._build_device_set(evdev, self._keyboard_device_path, self._capture_all)

        if not devices:
            logger.warning("evdev: no devices found")
        else:
            logger.info("evdev: %d device(s) opened", len(devices))

        while not self._stop_event.is_set():
            if not devices:
                time.sleep(0.5)
                continue

            fds = {dev.fd: dev for dev in devices.values()}
            try:
                readable, _, _ = select.select(list(fds.keys()), [], [], 0.5)
            except (ValueError, OSError):
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
    def _build_device_set(evdev_mod, keyboard_path: str, capture_all: bool = False) -> dict:
        devices: dict = {}

        if keyboard_path:
            try:
                dev = evdev_mod.InputDevice(keyboard_path)
                devices[keyboard_path] = dev
                logger.info("evdev: opened keyboard %s (%s)", keyboard_path, dev.name)
            except PermissionError:
                logger.warning("evdev: no permission to open %s", keyboard_path)
            except Exception as e:
                logger.warning("evdev: could not open %s: %s", keyboard_path, e)
        elif capture_all:
            #this is only really used by "add key" button in settings
            try:
                for path in evdev_mod.list_devices():
                    try:
                        dev  = evdev_mod.InputDevice(path)
                        caps = dev.capabilities()
                        keys = caps.get(evdev_mod.ecodes.EV_KEY, [])
                        if evdev_mod.ecodes.KEY_A in keys or evdev_mod.ecodes.BTN_LEFT in keys:
                            devices[path] = dev
                            logger.info("evdev: capture_all opened %s (%s)", path, dev.name)
                        else:
                            dev.close()
                    except PermissionError:
                        logger.warning("evdev: no permission to open %s — try: sudo usermod -aG input $USER", path)
                    except Exception as e:
                        logger.debug("evdev: could not open %s: %s", path, e)
            except Exception as e:
                logger.debug("evdev: list_devices error: %s", e)

        return devices

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

            #see if mouse button
            btn_code = _EVDEV_BTN_TO_CODE.get(code)
            if btn_code is not None:
                if value in (0, 1):
                    try:
                        self._on_mouse_click(btn_code, value == 1)
                    except Exception:
                        self._on_mouse_click(btn_code, False)
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
