from __future__ import annotations

import logging
import sys
import threading
import time
from typing import Callable

from services.consts import HID_TO_VK, RAZER_TO_HID

logger = logging.getLogger(__name__)

ANALOG_KEYBOARDS: list[tuple] = [
    # (vendor, product, name, usage_page)
    (0x31E3, None,   "Wooting",                    0xFF54),
    (0x31E3, None,   "Wooting",                    0xFF53),  #analog interface v2 firmware (v5.3.0+)
    (0x03EB, 0xFF01, "Wooting One",                0xFF54),
    (0x03EB, 0xFF02, "Wooting Two",                0xFF54),
    (0x1532, 0x0266, "Razer Huntsman V2 Analog",   None),
    (0x1532, 0x0282, "Razer Huntsman Mini Analog",  None),
    (0x1532, 0x02a6, "Razer Huntsman V3 Pro",       None),
    (0x1532, 0x02a7, "Razer Huntsman V3 Pro TKL",   None),
    (0x1532, 0x02b0, "Razer Huntsman V3 Pro Mini",  None),
    (0x19f5, None,   "NuPhy",                      0x0001),
    (0x352D, None,   "DrunkDeer",                   0xFF00),
    (0x3434, None,   "Keychron HE",                 0xFF60),
    (0x362D, None,   "Lemokey HE",                  0xFF60),
    (0x373b, None,   "Madlions HE",                 0xFF60),
    (0x372E, 0x105B, "Redragon K709 HE",            0xFF60),
]


def enum_analog_devices() -> list[dict]:
    try:
        import hid
    except ImportError:
        logger.error("hidapi not installed")
        return []

    devices: list    = []
    seen_vidpid: set = set()
    logger.info("scanning for analog keyboards...")
    try:
        for device_dict in hid.enumerate():
            vid        = device_dict["vendor_id"]
            pid        = device_dict["product_id"]
            usage_page = device_dict.get("usage_page", 0)
            interface  = device_dict.get("interface_number", -1)
            path       = device_dict.get("path", b"").decode("utf-8", errors="ignore")

            for known_vid, known_pid, name, required_usage in ANALOG_KEYBOARDS:
                if vid != known_vid or (known_pid is not None and pid != known_pid):
                    continue

                if required_usage is not None and usage_page != required_usage:
                    if sys.platform != "win32" and usage_page == 0:
                        continue
                    continue

                if required_usage is None:
                    vidpid_key = (vid, pid)
                    if vidpid_key in seen_vidpid:
                        break
                    seen_vidpid.add(vidpid_key)

                canonical_usage = required_usage if required_usage is not None else usage_page

                device_str   = f"{vid:04x}:{pid:04x}:{interface}" if interface >= 0 else f"{vid:04x}:{pid:04x}"
                product_name = device_dict.get("product_string", name)
                logger.info("found: %s (%s) usage_page=0x%04x interface=%d",
                            product_name, device_str, canonical_usage, interface)
                devices.append({
                    "id":         device_str,
                    "name":       f"{product_name} ({device_str}) [usage:0x{canonical_usage:04x}]",
                    "interface":  interface,
                    "usage_page": canonical_usage,
                    "path":       path,
                })
                break
    except Exception:
        logger.exception("error enumerating HID devices")

    logger.info("found %d analog keyboard interface(s)", len(devices))
    return devices

def enum_analog_devices_all() -> list[dict]:
    try:
        import hid
    except ImportError:
        logger.error("hidapi not installed")
        return []

    devices: list    = []
    seen_paths: set  = set()
    logger.info("scanning for analog keyboards on all usage_pages...")
    try:
        for device_dict in hid.enumerate():
            vid        = device_dict["vendor_id"]
            pid        = device_dict["product_id"]
            usage_page = device_dict.get("usage_page", 0)
            interface  = device_dict.get("interface_number", -1)
            path       = device_dict.get("path", b"").decode("utf-8", errors="ignore")

            for known_vid, known_pid, name, _ in ANALOG_KEYBOARDS:
                if vid != known_vid or (known_pid is not None and pid != known_pid):
                    continue

                if path in seen_paths:
                    break
                seen_paths.add(path)

                device_str   = f"{vid:04x}:{pid:04x}:{interface}" if interface >= 0 else f"{vid:04x}:{pid:04x}"
                product_name = device_dict.get("product_string", name)
                logger.info("found: %s (%s) usage_page=0x%04x interface=%d",
                            product_name, device_str, usage_page, interface)
                devices.append({
                    "id":         device_str,
                    "name":       f"{product_name} ({device_str}) [usage:0x{usage_page:04x}]",
                    "interface":  interface,
                    "usage_page": usage_page,
                    "path":       path,
                })
                break
    except Exception:
        logger.exception("error enumerating HID devices (all usage pages)")

    logger.info("found %d analog keyboard interface(s) (all usage pages)", len(devices))
    return devices

class AnalogHandler:
    def __init__(
        self,
        queue_message: Callable[[dict], None],
        is_allowed: Callable[[int], bool],
    ) -> None:
        self._queue_message  = queue_message
        self._is_allowed     = is_allowed
        self._running        = False
        self._thread: threading.Thread | None = None
        self._buffer: dict   = {}   # bytech keyed by rawcode

    def start(self, device_id: str) -> None:
        if self._running:
            logger.debug("analog handler already running")
            return
        try:
            import hid
        except ImportError:
            logger.error("hidapi not there")
            return

        self._running = True
        self._thread  = threading.Thread(
            target=self._worker,
            args=(device_id,),
            daemon=True,
            name="AnalogWorker",
        )
        self._thread.start()
        logger.info("analog support started")

    def stop(self) -> None:
        self._running = False
        if self._thread:
            self._thread.join(timeout=1.0)
            self._thread = None
        logger.info("analog support stopped")

    @property
    def is_running(self) -> bool:
        return self._running and bool(self._thread and self._thread.is_alive())

    def _worker(self, device_id: str) -> None:
        import hid
        device = None
        try:
            if not device_id or ":" not in device_id:
                logger.warning("no analog device configured")
                return

            parts = device_id.split(":")
            vid   = int(parts[0], 16)
            pid   = int(parts[1], 16)
            device = hid.device()

            if len(parts) > 2:
                interface_num = int(parts[2])
                logger.info("looking for analog interface %d", interface_num)
                target_path = next(
                    (d["path"] for d in hid.enumerate(vid, pid)
                     if d.get("interface_number", -1) == interface_num),
                    None,
                )
                if target_path:
                    logger.info("found analog interface %d at path: %s", interface_num, target_path)
                    device.open_path(target_path)
                else:
                    logger.warning("analog interface %d not found, trying default open", interface_num)
                    device.open(vid, pid)
            else:
                device.open(vid, pid)

            device.set_nonblocking(False)

            # resolve actual usage_page for the opened interface
            _connected_usage = 0
            for d in hid.enumerate(vid, pid):
                if len(parts) > 2:
                    if d.get("interface_number", -1) == int(parts[2]):
                        _connected_usage = d.get("usage_page", 0)
                        break
                else:
                    _connected_usage = d.get("usage_page", 0)
                    break

            logger.info(
                "connected to analog device: vid=0x%04x pid=0x%04x usage_page=0x%04x interface=%s manufacturer=%s product=%s",
                vid, pid, _connected_usage,
                parts[2] if len(parts) > 2 else "default",
                device.get_manufacturer_string(),
                device.get_product_string(),
            )

            if vid == 0x31E3 or (vid == 0x03EB and pid in [0xFF01, 0xFF02]):
                #see if its anawuhg interface v2 or v1
                usage_page = _connected_usage
                if usage_page == 0xFF53:
                    self._loop_wooting_v2(device)
                else:
                    self._loop_wooting(device)
            elif vid == 0x1532:
                logger.info("detected Razer keyboard - PID %04x", pid)
                if pid in [0x0266, 0x0282]:
                    self._loop_razer_v2(device)
                elif pid in [0x02a6, 0x02a7, 0x02b0]:
                    self._loop_razer_v3(device)
                else:
                    logger.warning("unsupported Razer PID %04x", pid)
            elif vid == 0x19f5:
                self._loop_nuphy(device)
            elif vid == 0x352D:
                self._loop_drunkdeer(device)
            elif vid == 0x373b:
                self._loop_madlions(device, pid)
            elif vid == 0x372E:
                self._loop_bytech(device)
            else:
                logger.warning("analog protocol not implemented for VID %04x", vid)

        except Exception:
            logger.exception("analog support error")
        finally:
            if device:
                try:
                    device.close()
                    logger.info("analog device closed")
                except Exception:
                    pass
            self._running = False

    def _loop_wooting(self, device) -> None:
        logger.info("detected Wooting keyboard (analog interface v1)")
        consecutive_empty = 0
        first_data_logged = False
        while self._running:
            try:
                data = device.read(32, timeout_ms=100)
                if not first_data_logged and data and any(b != 0 for b in data):
                    logger.info("FIRST analog data received: %s", " ".join(f"{b:02x}" for b in data[:16]))
                    first_data_logged = True
                if data:
                    if any(b != 0 for b in data):
                        consecutive_empty = 0
                        self._process_wooting(data)
                    else:
                        consecutive_empty += 1
                        if consecutive_empty == 50:
                            logger.warning("receiving only zeros - press a key to test")
                        elif consecutive_empty % 100 == 0:
                            logger.warning("still receiving zeros (%d reads)", consecutive_empty)
                else:
                    consecutive_empty += 1
            except Exception as e:
                logger.error("error reading Wooting: %s", e)
                break

    def _loop_wooting_v2(self, device) -> None:
        logger.info("detected Wooting keyboard (analog interface v2)")
        while self._running:
            try:
                data = device.read(64, timeout_ms=100)
                if data and any(b != 0 for b in data):
                    self._process_wooting_v2(data)
            except Exception as e:
                logger.error("error reading Wooting analog interface v2: %s", e)
                break

    def _loop_razer_v2(self, device) -> None:
        logger.info("using Huntsman V2/Mini protocol")
        while self._running:
            try:
                data = device.read(64, timeout_ms=100)
                if data and any(b != 0 for b in data):
                    self._process_razer_huntsman(data)
            except Exception as e:
                logger.error("error reading Razer V2: %s", e)
                break

    def _loop_razer_v3(self, device) -> None:
        logger.info("using Huntsman V3 protocol")
        while self._running:
            try:
                data = device.read(64, timeout_ms=100)
                if data and any(b != 0 for b in data):
                    self._process_razer_huntsman(data)   # same wire format as V2
            except Exception as e:
                logger.error("error reading Razer V3: %s", e)
                break

    def _loop_nuphy(self, device) -> None:
        logger.info("detected NuPhy keyboard")
        buf: dict = {}
        while self._running:
            try:
                data = device.read(64, timeout_ms=100)
                if data and len(data) >= 8 and data[0] == 0xA0:
                    self._process_nuphy(data, buf)
            except Exception as e:
                logger.error("error reading NuPhy: %s", e)
                break

    def _loop_drunkdeer(self, device) -> None:
        logger.info("detected DrunkDeer keyboard")
        active_keys_buf: list = []
        last_poll = 0.0
        while self._running:
            try:
                now = time.monotonic()
                if now - last_poll >= 0.008:
                    poll_buf     = [0x00] * 63
                    poll_buf[:7] = [0xb6, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00]
                    device.write([0x04] + poll_buf)
                    last_poll = now
                data = device.read(64, timeout_ms=10)
                if data and len(data) > 4:
                    self._process_drunkdeer(data, active_keys_buf)
            except Exception as e:
                logger.error("error reading DrunkDeer: %s", e)
                break

    def _loop_madlions(self, device, pid: int) -> None:
        logger.info("detected Madlions keyboard")
        buf: dict    = {}
        offset       = [0]
        layout_size  = 5 * 14 if pid in [0x1055, 0x1056, 0x105D] else 5 * 15
        init_buf     = [0x00] * 32
        init_buf[:8] = [0x02, 0x96, 0x1C, 0x00, 0x00, 0x00, 0x00, 0x04]
        device.write(init_buf)
        while self._running:
            try:
                data = device.read(64, timeout_ms=100)
                if data and len(data) >= 27:
                    self._process_madlions(data, buf, offset, layout_size, device, init_buf, pid)
            except Exception as e:
                logger.error("error reading Madlions: %s", e)
                break

    def _loop_bytech(self, device) -> None:
        logger.info("detected Bytech/Redragon keyboard")
        self._buffer.clear()
        poll_payload  = _build_bytech_payload(0x97, 0x00)
        device.write([0x09] + list(poll_payload))
        logger.info("bytech initial poll sent")
        POLL_INTERVAL = 0.008
        last_poll     = time.monotonic()
        while self._running:
            try:
                now = time.monotonic()
                if now - last_poll >= POLL_INTERVAL:
                    device.write([0x09] + list(poll_payload))
                    last_poll = now
                data = device.read(64, timeout_ms=4)
                if not data:
                    continue
                if len(data) >= 3 and data[1] == 0x97 and data[2] == 0x01:
                    self._process_bytech(data, self._buffer)
            except Exception as e:
                logger.error("error reading Bytech: %s", e)
                break

    def _process_wooting(self, data: list) -> None:
        #[scancode_hi, scancode_lo, value, ...] triplets :gasp:
        try:
            active_keys = []
            i = 0
            while i < len(data) - 2:
                scancode = (data[i] << 8) | data[i + 1]
                if scancode == 0:
                    break
                i += 2
                if i >= len(data):
                    break
                depth   = data[i] / 255.0
                i += 1
                rawcode = HID_TO_VK.get(scancode, 0)
                if rawcode == 0 and (scancode >> 8) == 0 and (scancode & 0xFF) > 0:
                    rawcode = HID_TO_VK.get(scancode & 0xFF, 0)
                if rawcode > 0 and depth > 0.01 and self._is_allowed(rawcode):
                    active_keys.append({"rawcode": rawcode, "depth": round(depth, 2)})
            for key in active_keys:
                self._queue_message({"event_type": "analog_depth", "rawcode": key["rawcode"], "depth": key["depth"]})
        except Exception as e:
            logger.error("error processing Wooting data: %s", e)

    def _process_wooting_v2(self, data: list) -> None:
        #each entry is 4 bytes (pos, keycode, namespace + analog lo, analog depth
        #namespace 0 are regular hid keys, non zero are media keys
        #analog value is 10bit 0 to1023
        try:
            active_keys = []
            i = 0
            while i + 4 <= len(data):
                keycode = data[i + 1]
                if keycode == 0:
                    break
                packed        = data[i + 2]
                value_hi      = data[i + 3]
                key_namespace = (packed >> 2) & 0xF
                value_lo      = (packed >> 6) & 0x3
                scancode      = (key_namespace << 8) | keycode
                value         = (value_hi << 2) | value_lo
                depth         = value / 1023.0
                i += 4
                rawcode = HID_TO_VK.get(scancode, 0)
                if rawcode == 0 and (scancode >> 8) == 0 and (scancode & 0xFF) > 0:
                    rawcode = HID_TO_VK.get(scancode & 0xFF, 0)
                if rawcode > 0 and depth > 0.01 and self._is_allowed(rawcode):
                    active_keys.append({"rawcode": rawcode, "depth": round(depth, 4)})
            for key in active_keys:
                self._queue_message({"event_type": "analog_depth", "rawcode": key["rawcode"], "depth": key["depth"]})
        except Exception as e:
            logger.error("error processing Wooting v2 data: %s", e)

    def _process_razer_huntsman(self, data: list) -> None:
        try:
            active_keys = []
            i = 0
            while i < len(data) - 1:
                razer_sc = data[i]
                if razer_sc == 0:
                    break
                i += 1
                if i >= len(data):
                    break
                value  = data[i]; i += 1
                hid_sc = RAZER_TO_HID.get(razer_sc, 0)
                if hid_sc:
                    rawcode = HID_TO_VK.get(hid_sc, 0)
                    depth   = value / 255.0
                    if rawcode > 0 and depth > 0.01 and self._is_allowed(rawcode):
                        active_keys.append({"rawcode": rawcode, "depth": round(depth, 2)})
            for key in active_keys:
                self._queue_message({"event_type": "analog_depth", "rawcode": key["rawcode"], "depth": key["depth"]})
        except Exception as e:
            logger.error("error processing Razer data: %s", e)

    def _process_nuphy(self, data: list, buffer: dict) -> None:
        try:
            if data[0] != 0xA0 or len(data) < 8:
                return
            key_count = data[2]
            for i in range(key_count):
                base    = 3 + i * 2
                if base + 1 >= len(data):
                    break
                hid_sc  = data[base]
                value   = data[base + 1]
                rawcode = HID_TO_VK.get(hid_sc, 0)
                if rawcode == 0:
                    continue
                if value == 0:
                    buffer.pop(rawcode, None)
                elif self._is_allowed(rawcode):
                    buffer[rawcode] = round(value / 255.0, 2)
            for rawcode, depth in list(buffer.items()):
                self._queue_message({"event_type": "analog_depth", "rawcode": rawcode, "depth": depth})
        except Exception as e:
            logger.error("error processing NuPhy data: %s", e)

    _DRUNKDEER_INDEX_TO_HID: dict[int, int] = {
        # row 0
        (0*21)+0:  0x29, (0*21)+1:  0x3A, (0*21)+2:  0x3B, (0*21)+3:  0x3C, (0*21)+4:  0x3D,
        (0*21)+5:  0x3E, (0*21)+6:  0x3F, (0*21)+7:  0x40, (0*21)+8:  0x41, (0*21)+9:  0x42,
        (0*21)+10: 0x43, (0*21)+11: 0x44, (0*21)+12: 0x45, (0*21)+14: 0x46, (0*21)+15: 0x47, (0*21)+16: 0x48,
        # row 1
        (1*21)+0:  0x35, (1*21)+1:  0x1E, (1*21)+2:  0x1F, (1*21)+3:  0x20, (1*21)+4:  0x21,
        (1*21)+5:  0x22, (1*21)+6:  0x23, (1*21)+7:  0x24, (1*21)+8:  0x25, (1*21)+9:  0x26,
        (1*21)+10: 0x27, (1*21)+11: 0x2D, (1*21)+12: 0x2E, (1*21)+13: 0x2A,
        (1*21)+14: 0x49, (1*21)+15: 0x4A, (1*21)+16: 0x4B,
        # row 2
        (2*21)+0:  0x2B, (2*21)+1:  0x14, (2*21)+2:  0x1A, (2*21)+3:  0x08, (2*21)+4:  0x15,
        (2*21)+5:  0x17, (2*21)+6:  0x1C, (2*21)+7:  0x18, (2*21)+8:  0x0C, (2*21)+9:  0x12,
        (2*21)+10: 0x13, (2*21)+11: 0x2F, (2*21)+12: 0x30, (2*21)+13: 0x31,
        (2*21)+14: 0x4C, (2*21)+15: 0x4D, (2*21)+16: 0x4E,
        # row 3
        (3*21)+0:  0x39, (3*21)+1:  0x04, (3*21)+2:  0x16, (3*21)+3:  0x07, (3*21)+4:  0x09,
        (3*21)+5:  0x0A, (3*21)+6:  0x0B, (3*21)+7:  0x0D, (3*21)+8:  0x0E, (3*21)+9:  0x0F,
        (3*21)+10: 0x33, (3*21)+11: 0x34, (3*21)+13: 0x28, (3*21)+15: 0x4E,
        # row 4
        (4*21)+0:  0xE1, (4*21)+2:  0x1D, (4*21)+3:  0x1B, (4*21)+4:  0x06, (4*21)+5:  0x19,
        (4*21)+6:  0x05, (4*21)+7:  0x11, (4*21)+8:  0x10, (4*21)+9:  0x36, (4*21)+10: 0x37,
        (4*21)+11: 0x38, (4*21)+13: 0xE5, (4*21)+14: 0x52, (4*21)+15: 0x4D,
        # row 5
        (5*21)+0:  0xE0, (5*21)+1:  0xE3, (5*21)+2:  0xE2, (5*21)+6:  0x2C,
        (5*21)+10: 0xE6, (5*21)+11: 0x409, (5*21)+12: 0x65,
        (5*21)+14: 0x50, (5*21)+15: 0x51, (5*21)+16: 0x4F,
    }

    def _process_drunkdeer(self, data: list, active_keys_buf: list) -> None:
        try:
            n      = data[3]
            stride = 64 - 5
            if n == 0:
                active_keys_buf.clear()
            for i in range(4, len(data)):
                value = data[i]
                if value:
                    idx    = n * stride + (i - 4)
                    hid_sc = self._DRUNKDEER_INDEX_TO_HID.get(idx, 0)
                    if hid_sc:
                        rawcode = HID_TO_VK.get(hid_sc, 0)
                        if rawcode and self._is_allowed(rawcode):
                            active_keys_buf.append({
                                "rawcode": rawcode,
                                "depth":   round(min(value / 40.0, 1.0), 2),
                            })
            if n == 2:
                for key in active_keys_buf:
                    self._queue_message({"event_type": "analog_depth", "rawcode": key["rawcode"], "depth": key["depth"]})
        except Exception as e:
            logger.error("error processing DrunkDeer data: %s", e)

    _MADLIONS_LAYOUT_60: list[int] = [
        0x29, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2D, 0x2E, 0x2A,
        0x2B, 0x14, 0x1A, 0x08, 0x15, 0x17, 0x1C, 0x18, 0x0C, 0x12, 0x13, 0x2F, 0x30, 0x31,
        0x39, 0x04, 0x16, 0x07, 0x09, 0x0A, 0x0B, 0x0D, 0x0E, 0x0F, 0x33, 0x34, 0x00, 0x28,
        0xE1, 0x00, 0x1D, 0x1B, 0x06, 0x19, 0x05, 0x11, 0x10, 0x36, 0x37, 0x38, 0x00, 0xE5,
        0xE0, 0xE3, 0xE2, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00, 0xE7, 0xE6, 0x65, 0xE4, 0x409,
    ]
    _MADLIONS_LAYOUT_68: list[int] = [
        0x29, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2D, 0x2E, 0x2A, 0x49,
        0x2B, 0x14, 0x1A, 0x08, 0x15, 0x17, 0x1C, 0x18, 0x0C, 0x12, 0x13, 0x2F, 0x30, 0x31, 0x4C,
        0x39, 0x04, 0x16, 0x07, 0x09, 0x0A, 0x0B, 0x0D, 0x0E, 0x0F, 0x33, 0x34, 0x00, 0x28, 0x4B,
        0xE1, 0x00, 0x1D, 0x1B, 0x06, 0x19, 0x05, 0x11, 0x10, 0x36, 0x37, 0x38, 0xE5, 0x52, 0x4E,
        0xE0, 0xE3, 0xE2, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00, 0xE6, 0x409, 0xE4, 0x50, 0x51, 0x4F,
    ]

    def _process_madlions(self, data: list, buffer: dict, offset: list,
                           layout_size: int, device, init_buf: list, pid: int) -> None:
        try:
            layout = self._MADLIONS_LAYOUT_60 if pid in [0x1055, 0x1056, 0x105D] else self._MADLIONS_LAYOUT_68
            for i in range(4):
                li = offset[0] + i
                if li < len(layout):
                    hid_sc  = layout[li]
                    travel  = (data[7 + i*5 + 3] << 8) | data[7 + i*5 + 4]
                    rawcode = HID_TO_VK.get(hid_sc, 0)
                    if rawcode:
                        if travel == 0:
                            buffer.pop(rawcode, None)
                        elif self._is_allowed(rawcode):
                            buffer[rawcode] = round(min(travel / 350.0, 1.0), 2)
            for rawcode, depth in list(buffer.items()):
                self._queue_message({"event_type": "analog_depth", "rawcode": rawcode, "depth": depth})
            offset[0] = (offset[0] + 4) % layout_size
            init_buf[6] = offset[0]
            try:
                device.write(init_buf)
            except Exception:
                pass
        except Exception as e:
            logger.error("error processing Madlions data: %s", e)

    _BYTECH_TO_HID: dict[int, int] = {
        1:  0x29,  2:  0x3A,  3:  0x3B,  4:  0x3C,  5:  0x3D,  6:  0x3E,  7:  0x3F,
        8:  0x40,  9:  0x41, 10:  0x42, 11:  0x43, 12:  0x44, 13:  0x45,
        14: 0x35, 15:  0x1E, 16:  0x1F, 17:  0x20, 18:  0x21, 19:  0x22,
        20: 0x23, 21:  0x24, 22:  0x25, 23:  0x26, 24:  0x27, 25:  0x2D,
        26: 0x2E, 27:  0x2A, 28:  0x2B, 29:  0x14, 30:  0x1A, 31:  0x08,
        32: 0x15, 33:  0x17, 34:  0x1C, 35:  0x18, 36:  0x0C, 37:  0x12,
        38: 0x13, 39:  0x2F, 40:  0x30, 41:  0x31, 42:  0x39, 43:  0x04,
        44: 0x16, 45:  0x07, 46:  0x09, 47:  0x0A, 48:  0x0B, 49:  0x0D,
        50: 0x0E, 51:  0x0F, 52:  0x33, 53:  0x34, 54:  0x28, 55:  0xE1,
        56: 0x1D, 57:  0x1B, 58:  0x06, 59:  0x19, 60:  0x05, 61:  0x11,
        62: 0x10, 63:  0x36, 64:  0x37, 65:  0x38, 66:  0xE5, 67:  0xE0,
        68: 0xE3, 69:  0xE2, 70:  0x2C, 71:  0xE6, 72: 0x409, 73:  0xE4,
        74: 0x52, 75:  0x51, 76:  0x50, 77:  0x4F,
        99: 0x4C, 100: 0x4A, 102: 0x4B, 103: 0x4E,
    }

    def _process_bytech(self, data: list, buffer: dict) -> None:
        try:
            count      = data[6]
            new_buffer = {}
            for i in range(0, count, 4):
                if 7 + i + 4 > len(data):
                    break
                pos      = data[8 + i]
                distance = (data[9 + i] << 8) | data[10 + i]
                hid_sc   = self._BYTECH_TO_HID.get(pos, 0)
                if not hid_sc:
                    continue
                rawcode = HID_TO_VK.get(hid_sc, 0)
                if rawcode == 0 or not self._is_allowed(rawcode):
                    continue
                new_buffer[rawcode] = round(min(distance / 355.0, 1.0), 2) if distance > 10 else 0.0
            for rawcode in buffer:
                if rawcode not in new_buffer:
                    self._queue_message({"event_type": "analog_depth", "rawcode": rawcode, "depth": 0.0})
            for rawcode, depth in new_buffer.items():
                if buffer.get(rawcode) != depth:
                    self._queue_message({"event_type": "analog_depth", "rawcode": rawcode, "depth": depth})
            buffer.clear()
            buffer.update({k: v for k, v in new_buffer.items() if v > 0.0})
        except Exception as e:
            logger.error("error processing Bytech data: %s", e)

def _build_bytech_payload(cmd: int, sub: int) -> bytes:
    buf     = bytearray(63)
    buf[0]  = cmd
    buf[1]  = sub
    total   = 9 + sum(buf[:-1])
    buf[-1] = (255 - (total % 256)) & 0xFF
    return bytes(buf)