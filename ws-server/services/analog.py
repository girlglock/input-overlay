from __future__ import annotations

import logging
import threading
from typing import Callable

from services.consts import HID_TO_VK

logger = logging.getLogger(__name__)


def enum_analog_devices() -> list[dict]:
    try:
        from analogsense import AnalogSense
    except ImportError:
        logger.error("analogsensepy is missing")
        return []

    devices: list[dict] = []
    try:
        sense = AnalogSense()
        for provider in sense.get_devices():
            name = getattr(provider.dev, "product_string", "Unknown Device")
            vid  = getattr(provider.dev, "vendor_id",  0)
            pid  = getattr(provider.dev, "product_id", 0)
            device_id = f"{vid:04x}:{pid:04x}"
            display   = f"{name} ({device_id}) [{type(provider).__name__}]"
            logger.info("found: %s", display)
            devices.append({"id": device_id, "name": display})
            provider.forget()
    except Exception:
        logger.exception("error enumerating analog devices")

    logger.info("found %d analog keyboard interface(s)", len(devices))
    return devices

class AnalogHandler:
    def __init__(
        self,
        queue_message: Callable[[dict], None],
        is_allowed:    Callable[[int], bool],
    ) -> None:
        self._queue_message = queue_message
        self._is_allowed    = is_allowed
        self._provider      = None
        self._running       = False
        self._lock          = threading.Lock()

    def start(self, device_id: str) -> None:
        with self._lock:
            if self._running:
                logger.debug("analog handler already running")
                return
            try:
                from analogsense import AnalogSense
            except ImportError:
                logger.error("analogsensepy is missing")
                return

            if not device_id or ":" not in device_id:
                logger.warning("no analog device configured")
                return

            parts = device_id.split(":")
            try:
                vid = int(parts[0], 16)
                pid = int(parts[1], 16)
            except ValueError:
                logger.error("invalid device_id format: %s", device_id)
                return

            provider = AnalogSense().open_device(vid, pid)
            if provider is None:
                logger.error("analog device %04x:%04x not found", vid, pid)
                return

            self._provider = provider
            self._running  = True
            provider.start_listening(self._on_analog_report)
            logger.info(
                "analog support started - %s (%04x:%04x) via %s",
                getattr(provider.dev, "product_string", "?"),
                vid, pid,
                type(provider).__name__,
            )

    def stop(self) -> None:
        with self._lock:
            provider, self._provider = self._provider, None
            self._running = False
        if provider:
            try:
                provider.forget()
            except Exception:
                logger.exception("error stopping analog provider")
        logger.info("analog support stopped")

    @property
    def is_running(self) -> bool:
        with self._lock:
            if not self._running or self._provider is None:
                return False
            t = getattr(self._provider, "_thread", None)
            return bool(t and t.is_alive())

    def _on_analog_report(self, active_keys: list[dict]) -> None:
        for entry in active_keys:
            scancode = entry.get("scancode", 0)
            value    = entry.get("value", 0.0)
            if not scancode:
                continue

            rawcode = HID_TO_VK.get(scancode, 0)
            if rawcode == 0:
                continue
            if not self._is_allowed(rawcode):
                continue

            depth = round(float(value), 4)
            self._queue_message({
                "event_type": "analog_depth",
                "rawcode":    rawcode,
                "depth":      depth,
            })