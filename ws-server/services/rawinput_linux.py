from __future__ import annotations

import logging
import os
import select
import threading
import time
from typing import Callable

logger = logging.getLogger(__name__)

from services.consts import RAW_MOUSE_FLUSH_HZ

FLUSH_HZ = RAW_MOUSE_FLUSH_HZ


def enum_raw_mouse_devices() -> list[dict]:
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
                EV_REL = evdev.ecodes.EV_REL
                REL_X  = evdev.ecodes.REL_X
                REL_Y  = evdev.ecodes.REL_Y
                rel_axes = caps.get(EV_REL, [])
                if REL_X in rel_axes and REL_Y in rel_axes:
                    results.append({
                        "path": path,
                        "name": dev.name,
                        "phys": getattr(dev, "phys", ""),
                    })
                dev.close()
            except Exception:
                pass
    except Exception as e:
        logger.debug("enum_raw_mouse_devices error: %s", e)

    return results


class RawMouseLinuxThread(threading.Thread):
    def __init__(
        self,
        callback:    Callable[[int, int], None],
        device_path: str  = "",
        min_delta:   int  = 0,
        daemon:      bool = True,
    ) -> None:
        super().__init__(daemon=daemon, name="RawMouseLinuxThread")
        self._callback    = callback
        self._device_path = device_path
        self._min_delta   = min_delta
        self._stop_evt    = threading.Event()
        self._lock        = threading.Lock()
        self._accum_dx    = 0
        self._accum_dy    = 0

    def stop(self) -> None:
        self._stop_evt.set()
        self.join(timeout=3.0)

    def run(self) -> None:
        if not self._device_path:
            logger.warning("raw mouse linux: no device path set.. skipping")
            return

        try:
            import evdev  #PLC0415
        except ImportError:
            logger.error("evdev is not there")
            return

        try:
            dev = evdev.InputDevice(self._device_path)
        except PermissionError:
            logger.error("raw mouse linux: no perms to open %s\ndo sudo usermod -aG input $USER",self._device_path)
            return
        except Exception as e:
            logger.error("raw mouse linux: could not open %s: %s", self._device_path, e)
            return

        logger.info("raw mouse linux: opened %s (%s)", self._device_path, dev.name)

        try:
            os.nice(5)
            logger.debug("raw mouse linux: niceness set to %d", os.nice(0))
        except OSError:
            logger.debug("raw mouse linux: could not set niceness (not permitted)")

        flush_thread = threading.Thread(
            target=self._flush_loop,
            daemon=True,
            name="RawMouseLinuxFlush",
        )
        flush_thread.start()

        EV_REL = evdev.ecodes.EV_REL
        REL_X  = evdev.ecodes.REL_X
        REL_Y  = evdev.ecodes.REL_Y

        try:
            while not self._stop_evt.is_set():
                try:
                    readable, _, _ = select.select([dev.fd], [], [], 0.5)
                except (ValueError, OSError) as e:
                    logger.error("raw mouse linux: select error on %s: %s", self._device_path, e)
                    break

                if not readable:
                    continue

                try:
                    for event in dev.read():
                        if event.type != EV_REL:
                            continue
                        dx = dy = 0
                        if event.code == REL_X:
                            dx = event.value
                        elif event.code == REL_Y:
                            dy = event.value
                        else:
                            continue
                        if self._min_delta and abs(dx) + abs(dy) < self._min_delta:
                            continue
                        with self._lock:
                            self._accum_dx += dx
                            self._accum_dy += dy
                except OSError as e:
                    logger.error("raw mouse (linux): device %s disconnected - %s", self._device_path, e)
                    break
        finally:
            try:
                dev.close()
            except Exception:
                pass

        logger.info("raw mouse (linux): thread stopped")

    def _flush_loop(self) -> None:
        interval = 1.0 / FLUSH_HZ
        while not self._stop_evt.is_set():
            time.sleep(interval)
            with self._lock:
                dx, dy = self._accum_dx, self._accum_dy
                self._accum_dx = 0
                self._accum_dy = 0
            if dx == 0 and dy == 0:
                continue
            try:
                self._callback(dx, dy)
            except Exception:
                logger.exception("raw mouse (linux): exception in flush callback")