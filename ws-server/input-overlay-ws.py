import asyncio
import json
import logging
import os
import secrets
import threading
import sys
import subprocess
import time
from typing import Set
from pathlib import Path
from queue import Queue

import websockets
from pynput import keyboard, mouse
import pystray
from PIL import Image, ImageDraw

from services.consts import (MOUSE_BUTTON_MAP, HID_TO_VK, RAZER_TO_HID,
                             RAW_CODE_TO_KEY_NAME, MOUSE_BUTTON_NAMES, MOUSE_SCROLL_NAMES,
                             get_rawcode)

if sys.platform == 'win32':
    try:
        from services.rawinput import RawMouseThread
        RAW_MOUSE_AVAILABLE = True
    except Exception as _e:
        RAW_MOUSE_AVAILABLE = False
        logging.getLogger(__name__).warning(f"raw mouse unavailable: {_e}")
else:
    RAW_MOUSE_AVAILABLE = False

import re
import atexit
import signal
import traceback as _traceback

_SECRET_PATTERNS = [ #for logging
    (re.compile(r'(?i)(sec-websocket-key\s*:\s*)(\S{4})\S+'),     r'\g<1>\g<2>****'),
    (re.compile(r'(?i)(sec-websocket-accept\s*:\s*)(\S{4})\S+'),  r'\g<1>\g<2>****'),
    (re.compile(r'(?i)(authorization\s*:\s*)(\S{4})\S+'),         r'\g<1>\g<2>****'),
    (re.compile(r'(?i)(auth[_\-]?token\s*[=:]\s*)(\S{4})\S+'),   r'\g<1>\g<2>****'),
    (re.compile(r'(?i)(wsauth=)(\S{4})\S+'),                      r'\g<1>\g<2>****'),
    (re.compile(r'"token"\s*:\s*"([^"]{4})[^"]*"'),               r'"token": "\1****"'),
    (re.compile(r"'token'\s*:\s*'([^']{4})[^']*'"),               r"'token': '\1****'"),
]

def _redact(text: str) -> str:
    for pattern, repl in _SECRET_PATTERNS:
        text = pattern.sub(repl, text)
    return text


class _RedactingHandler(logging.Handler):
    def __init__(self, inner: logging.Handler):
        super().__init__()
        self.inner = inner

    def setFormatter(self, fmt):
        self.inner.setFormatter(fmt)

    def emit(self, record: logging.LogRecord):
        try:
            msg = self.inner.format(record)
            msg = _redact(msg)
            if not msg.strip():
                return
            stream = getattr(self.inner, 'stream', None)
            if stream is not None:
                stream.write(msg + self.inner.terminator)
                stream.flush()
            else:
                self.inner.emit(record)
        except Exception:
            self.handleError(record)

    def flush(self):
        self.inner.flush()

    def close(self):
        self.inner.close()
        super().close()


import datetime as _dt
_logs_dir = (Path(sys.executable).parent if getattr(sys, 'frozen', False) else Path(__file__).parent) / "logs"
_logs_dir.mkdir(exist_ok=True)
_LOG_FILE = _logs_dir / f"{_dt.datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}_{os.getpid()}.log"
_fmt = logging.Formatter('%(asctime)s - %(levelname)s - %(message)s')

_seen_events: set = set()

_file_handler = _RedactingHandler(logging.FileHandler(_LOG_FILE, mode='w', encoding='utf-8', delay=False))
_file_handler.inner.setFormatter(_fmt)

_console_handler = _RedactingHandler(logging.StreamHandler(sys.stdout))
_console_handler.inner.setFormatter(_fmt)

_root = logging.getLogger()
_root.setLevel(logging.DEBUG)
_root.addHandler(_file_handler)
_root.addHandler(_console_handler)

logging.getLogger('websockets').setLevel(logging.WARNING)
logging.getLogger('PIL').setLevel(logging.WARNING)

def _flush_log():
    try:
        _file_handler.flush()
        _file_handler.close()
    except Exception:
        pass

atexit.register(_flush_log)

def _crash_handler(exc_type, exc_value, exc_tb):
    logging.getLogger(__name__).critical(
        "unhandled exception:\n%s",
        "".join(_traceback.format_exception(exc_type, exc_value, exc_tb))
    )
    _flush_log()
    sys.__excepthook__(exc_type, exc_value, exc_tb)

sys.excepthook = _crash_handler

def _signal_handler(sig, frame):
    logging.getLogger(__name__).info("received signal %s, shutting down", sig)
    _flush_log()
    sys.exit(0)

for _sig in (signal.SIGTERM, signal.SIGINT):
    try:
        signal.signal(_sig, _signal_handler)
    except (OSError, ValueError):
        pass

logger = logging.getLogger(__name__)

if sys.platform == 'win32':
    try:
        from winotify import Notification, audio
        TOAST_AVAILABLE = True
    except ImportError:
        TOAST_AVAILABLE = False
        logger.warning("winotify not installed")
else:
    TOAST_AVAILABLE = False


class InputOverlayServer:
    def __init__(self, host: str = "localhost", port: int = 16899, auth_token: str = None):
        self.host = host
        self.port = port
        self.auth_token = auth_token
        self.clients: Set[websockets.WebSocketServerProtocol] = set()
        self.authenticated_clients: Set[websockets.WebSocketServerProtocol] = set()
        self.keyboard_listener = None
        self.mouse_listener = None
        self.running = False
        self.loop = None
        self.message_queue = Queue()
        self.queue_processor_task = None
        self.analog_enabled = False
        self.analog_device = None
        self.analog_thread = None
        self._analog_running = False
        self.key_whitelist = []
        self.balloon_notifications = True
        self.config_path = "config.json"
        self.config_last_modified = 0
        self.analog_buffer = {}
        self.raw_mouse_enabled = False
        self.raw_mouse_min_delta = 0
        self._raw_mouse_thread: "RawMouseThread | None" = None

    def show_toast_notification(self, title: str, message: str, duration: str = "short"):
        if not TOAST_AVAILABLE or not self.balloon_notifications:
            return
        try:
            icon_path = get_resource_path("assets/icon.ico")
            toast = Notification(
                app_id="Input Overlay",
                title=title,
                msg=message,
                icon=str(icon_path) if icon_path.exists() else ""
            )
            toast.set_audio(audio.Reminder, loop=False)
            toast.show()
        except Exception as e:
            logger.error(f"failed to show toast notification: {e}")

    def load_config(self, config_path: str = "config.json") -> dict:
        try:
            config_file = Path(config_path)
            if config_file.exists():
                with open(config_file, 'r') as f:
                    config = json.load(f)
                    logger.info(f"loaded config from {config_path}")
                    self.config_last_modified = config_file.stat().st_mtime
                    return config
            else:
                random_token = secrets.token_urlsafe(32)
                default_config = {
                    "host": "localhost",
                    "port": 16899,
                    "auth_token": random_token,
                    "analog_enabled": False,
                    "analog_device": None,
                    "key_whitelist": [],
                    "balloon_notifications": True,
                    "dismissed_versions": [],
                    "raw_mouse_enabled": True,
                    "raw_mouse_min_delta": 0,
                    "cpu_affinity": [0, 1]
                }
                with open(config_file, 'w') as f:
                    json.dump(default_config, f, indent=4)
                logger.info(f"generated auth token: {random_token}")
                logger.info(f"add to overlay url: &wsauth={random_token}")
                self.config_last_modified = config_file.stat().st_mtime
                return default_config
        except Exception as e:
            logger.error(f"error loading config: {e}")
            return {}

    def reload_config_if_changed(self):
        try:
            config_file = Path(self.config_path)
            if config_file.exists():
                current_mtime = config_file.stat().st_mtime
                if current_mtime > self.config_last_modified:
                    logger.info("config file changed, reloading...")
                    config = self.load_config(self.config_path)

                    old_analog_enabled = self.analog_enabled
                    old_analog_device = self.analog_device
                    old_raw_mouse = self.raw_mouse_enabled

                    self.auth_token = config.get('auth_token', self.auth_token)
                    self.analog_enabled = config.get('analog_enabled', False)
                    self.analog_device = config.get('analog_device', None)
                    self.key_whitelist = config.get('key_whitelist', [])
                    self.balloon_notifications = config.get('balloon_notifications', True)
                    self.raw_mouse_enabled = config.get('raw_mouse_enabled', False)
                    self.raw_mouse_min_delta = config.get('raw_mouse_min_delta', 0)

                    logger.info(f"settings updated - analog: {self.analog_enabled}, device: {self.analog_device}")
                    logger.info(f"whitelist: {len(self.key_whitelist)} keys")

                    if old_analog_enabled != self.analog_enabled or old_analog_device != self.analog_device:
                        if old_analog_enabled:
                            self.stop_analog_support()
                        if self.analog_enabled:
                            self.start_analog_support()

                    if old_raw_mouse != self.raw_mouse_enabled:
                        if old_raw_mouse:
                            self.stop_raw_mouse()
                        if self.raw_mouse_enabled:
                            self.start_raw_mouse()

                    return True
        except Exception as e:
            logger.error(f"error reloading config: {e}")
        return False

    def save_config(self, config_path: str = "config.json"):
        try:
            config = {
                "host": self.host,
                "port": self.port,
                "auth_token": self.auth_token,
                "analog_enabled": self.analog_enabled,
                "analog_device": self.analog_device,
                "key_whitelist": self.key_whitelist
            }
            with open(config_path, 'w') as f:
                json.dump(config, f, indent=4)
            logger.info(f"saved config to {config_path}")
            self.config_last_modified = Path(config_path).stat().st_mtime
        except Exception as e:
            logger.error(f"error saving config: {e}")

    def is_allowed(self, code: int, is_mouse: bool = False,
                   is_scroll: bool = False, is_mouse_move: bool = False) -> bool:
        if not self.key_whitelist:
            return True

        if is_mouse_move and self.raw_mouse_enabled:
            return True

        name = None
        if is_scroll:
            name = MOUSE_SCROLL_NAMES.get(code)
            if "mouse_wheel" in self.key_whitelist:
                return True
        elif is_mouse:
            name = MOUSE_BUTTON_NAMES.get(code)
        else:
            name = RAW_CODE_TO_KEY_NAME.get(code)

        if name and name in self.key_whitelist:
            return True

        return False

    async def broadcast(self, message: dict):
        if not self.authenticated_clients:
            return
        
        message_json = json.dumps(message)
        disconnected = set()
        for client in list(self.authenticated_clients):
            try:
                await client.send(message_json)
            except websockets.exceptions.ConnectionClosed:
                disconnected.add(client)
            except Exception as e:
                logger.warning(f"error sending to client: {e}")
                disconnected.add(client)
        for client in disconnected:
            self.authenticated_clients.discard(client)
            self.clients.discard(client)

    async def process_message_queue(self):
        logger.debug("message queue processor started")
        FLUSH_INTERVAL = 1.0 / 125.0
        last_flush = asyncio.get_event_loop().time()
        pending_dx = 0
        pending_dy = 0

        while self.running:
            try:
                now = asyncio.get_event_loop().time()

                while not self.message_queue.empty():
                    try:
                        msg = self.message_queue.get_nowait()
                    except Exception:
                        break
                    if msg.get("event_type") == "mouse_moved":
                        pending_dx += msg.get("dx", 0)
                        pending_dy += msg.get("dy", 0)
                    else:
                        await self.broadcast(msg)

                if (pending_dx != 0 or pending_dy != 0) and (now - last_flush) >= FLUSH_INTERVAL:
                    await self.broadcast({"event_type": "mouse_moved", "dx": pending_dx, "dy": pending_dy})
                    pending_dx = 0
                    pending_dy = 0
                    last_flush = now

                await asyncio.sleep(0.001)
            except Exception as e:
                logger.error(f"error processing message queue: {e}")
                await asyncio.sleep(0.01)
        logger.debug("message queue processor stopped")

    def queue_message(self, message: dict):
        try:
            event_type = message.get('event_type')
            if event_type and event_type not in _seen_events:
                _seen_events.add(event_type)
                logger.info("%s detected", event_type.replace('_', ' '))
            self.message_queue.put(message)
        except Exception as e:
            logger.error(f"error queuing message: {e}")

    def on_key_press(self, key):
        rawcode = get_rawcode(key)
        if rawcode and self.is_allowed(rawcode):
            self.queue_message({"event_type": "key_pressed", "rawcode": rawcode})

    def on_key_release(self, key):
        rawcode = get_rawcode(key)
        if rawcode and self.is_allowed(rawcode):
            self.queue_message({"event_type": "key_released", "rawcode": rawcode})

    def on_mouse_click(self, x, y, button, pressed):
        button_code = MOUSE_BUTTON_MAP.get(button, 0)
        if button_code and self.is_allowed(button_code, is_mouse=True):
            message = {
                "event_type": "mouse_pressed" if pressed else "mouse_released",
                "button": button_code
            }
            self.queue_message(message)

    def on_mouse_scroll(self, x, y, dx, dy):
        rotation = -1 if dy > 0 else 1 if dy < 0 else 0
        if rotation != 0 and self.is_allowed(rotation, is_scroll=True):
            message = {"event_type": "mouse_wheel", "rotation": rotation}
            self.queue_message(message)

    def _on_raw_mouse_move(self, dx: int, dy: int):
        if not self.is_allowed(0, is_mouse_move=True):
            return
        self.queue_message({"event_type": "mouse_moved", "dx": dx, "dy": dy})

    def start_raw_mouse(self):
        if not RAW_MOUSE_AVAILABLE:
            logger.warning("rawinput buffer not available on this os")
            return
        if self._raw_mouse_thread and self._raw_mouse_thread.is_alive():
            logger.debug("rawinput buffer already running")
            return
        self._raw_mouse_thread = RawMouseThread(
            callback=self._on_raw_mouse_move,
            min_delta=self.raw_mouse_min_delta,
        )
        self._raw_mouse_thread.start()
        logger.info("rawinput buffer started")

    def stop_raw_mouse(self):
        if self._raw_mouse_thread:
            self._raw_mouse_thread.stop()
            self._raw_mouse_thread = None
        logger.info("rawinput buffer stopped")

    def start_analog_support(self):
        if not self.analog_enabled:
            return
        
        try:
            import hid
            
            self._analog_running = True

            def analog_worker():
                device = None
                try:
                    if self.analog_device and ':' in self.analog_device:
                        parts = self.analog_device.split(':')
                        vid_str, pid_str = parts[0], parts[1]
                        vid = int(vid_str, 16)
                        pid = int(pid_str, 16)
                        
                        device = hid.device()

                        if len(parts) > 2:
                            interface_num = int(parts[2])
                            logger.info(f"looking for analog interface {interface_num}")

                            all_devs = hid.enumerate(vid, pid)
                            target_path = None
                            for d in all_devs:
                                if d.get('interface_number', -1) == interface_num:
                                    target_path = d['path']
                                    logger.info(f"found analog interface {interface_num} at path: {target_path}")
                                    break
                            
                            if target_path:
                                device.open_path(target_path)
                            else:
                                logger.warning(f"anal interface {interface_num} not found, trying default open")
                                device.open(vid, pid)
                        else:
                            device.open(vid, pid)
                        
                        device.set_nonblocking(False)
                        
                        logger.info(f"opened analog device: {vid:04x}:{pid:04x}")
                        logger.info(f"manufacturer: {device.get_manufacturer_string()}")
                        logger.info(f"product: {device.get_product_string()}")
                        
                        if vid == 0x31E3 or (vid == 0x03EB and pid in [0xFF01, 0xFF02]):
                            logger.info("detected Wooting keyboard")
                            consecutive_empty = 0
                            first_data_logged = False
                            
                            while self._analog_running:
                                try:
                                    data = device.read(32, timeout_ms=100)

                                    if not first_data_logged and data and any(b != 0 for b in data):
                                        logger.info(f"FIRST analog DATA RECEIVED: {' '.join(f'{b:02x}' for b in data[:16])}")
                                        first_data_logged = True
                                    
                                    if data and len(data) > 0:
                                        if any(b != 0 for b in data):
                                            consecutive_empty = 0
                                            self.process_wooting_data(data)
                                        else:
                                            consecutive_empty += 1
                                            if consecutive_empty == 50:
                                                logger.warning("receiving only zeros - press a key to test")
                                            elif consecutive_empty % 100 == 0:
                                                logger.warning(f"still receiving zeros ({consecutive_empty} reads)")
                                    else:
                                        consecutive_empty += 1
                                except Exception as e:
                                    logger.error(f"error reading: {e}")
                                    break
                        
                        elif vid == 0x1532:
                            logger.info(f"detected Razer keyboard - PID {pid:04x}")
                            if pid in [0x0266, 0x0282]:
                                logger.info("using Huntsman V2/Mini protocol")
                                while self._analog_running:
                                    try:
                                        data = device.read(64, timeout_ms=100)
                                        if data and any(b != 0 for b in data):
                                            self.process_razer_huntsman_data(data)
                                    except Exception as e:
                                        logger.error(f"error: {e}")
                                        break
                            elif pid in [0x02a6, 0x02a7, 0x02b0]:
                                logger.info("using Huntsman V3 protocol")
                                while self._analog_running:
                                    try:
                                        data = device.read(64, timeout_ms=100)
                                        if data and any(b != 0 for b in data):
                                            self.process_razer_huntsman_v3_data(data)
                                    except Exception as e:
                                        logger.error(f"error: {e}")
                                        break
                        elif vid == 0x19f5:
                            logger.info("detected NuPhy keyboard")
                            nuphy_buffer = {}
                            while self._analog_running:
                                try:
                                    data = device.read(64, timeout_ms=100)
                                    if data and len(data) >= 8 and data[0] == 0xA0:
                                        self.process_nuphy_data(data, nuphy_buffer)
                                except Exception as e:
                                    logger.error(f"error reading NuPhy: {e}")
                                    break

                        elif vid == 0x352D:
                            logger.info("detected DrunkDeer keyboard")
                            import time as _time
                            active_keys_buf = []
                            last_poll = 0
                            while self._analog_running:
                                try:
                                    now = _time.monotonic()
                                    if now - last_poll >= 0.008:
                                        poll_buf = [0x00] * 63
                                        poll_buf[0:7] = [0xb6, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00]
                                        device.write([0x04] + poll_buf)
                                        last_poll = now
                                    data = device.read(64, timeout_ms=10)
                                    if data and len(data) > 4:
                                        self.process_drunkdeer_data(data, active_keys_buf)
                                except Exception as e:
                                    logger.error(f"error reading DrunkDeer: {e}")
                                    break

                        elif vid == 0x373b:
                            logger.info("detected Madlions keyboard")
                            madlions_buffer = {}
                            madlions_offset = [0]
                            if pid in [0x1055, 0x1056, 0x105D]:
                                madlions_layout_size = 5 * 14           
                            else:
                                madlions_layout_size = 5 * 15                    
                            init_buf = [0x00] * 32
                            init_buf[0:8] = [0x02, 0x96, 0x1C, 0x00, 0x00, 0x00, 0x00, 0x04]
                            device.write(init_buf)
                            while self._analog_running:
                                try:
                                    data = device.read(64, timeout_ms=100)
                                    if data and len(data) >= 27:
                                        self.process_madlions_data(data, madlions_buffer, madlions_offset,
                                                                   madlions_layout_size, device, init_buf, pid)
                                except Exception as e:
                                    logger.error(f"error reading Madlions: {e}")
                                    break

                        elif vid == 0x372E:
                            logger.info("detected Bytech/Redragon keyboard")
                            self.analog_buffer.clear()
                            poll_payload = self._build_bytech_payload(0x97, 0x00)
                            device.write([0x09] + list(poll_payload))
                            logger.info("bytech initial poll sent")
                            import time as _time
                            POLL_INTERVAL = 0.008       
                            last_poll = _time.monotonic()
                            while self._analog_running:
                                try:
                                    now = _time.monotonic()
                                    if now - last_poll >= POLL_INTERVAL:
                                        device.write([0x09] + list(poll_payload))
                                        last_poll = now

                                    data = device.read(64, timeout_ms=4)
                                    if not data:
                                        continue

                                    if len(data) >= 3 and data[1] == 0x97 and data[2] == 0x01:
                                        self.process_bytech_data(data, self.analog_buffer)
                                    elif len(data) >= 2 and data[1] == 0x97:
                                        pass
                                except Exception as e:
                                    logger.error(f"error reading Bytech: {e}")
                                    break

                        else:
                            logger.warning(f"protocol not implemented for VID {vid:04x}")
                    else:
                        logger.warning("no analog device configured")
                except Exception as e:
                    logger.error(f"analog support error: {e}")
                    import traceback
                    traceback.print_exc()
                finally:
                    if device:
                        try:
                            device.close()
                            logger.info("analog device closed")
                        except:
                            pass
            
            self.analog_thread = threading.Thread(target=analog_worker, daemon=True)
            self.analog_thread.start()
            logger.info("analog support started")
        except ImportError:
            logger.error("hidapi not installed. install with: pip install hidapi")
        except Exception as e:
            logger.error(f"failed to start analog support: {e}")
            import traceback
            traceback.print_exc()

    def stop_analog_support(self):
        self._analog_running = False
        if self.analog_thread:
            self.analog_thread.join(timeout=1.0)
            self.analog_thread = None
        logger.info("analog support stopped")

    def process_wooting_data(self, data: list):
        try:
            active_keys = []
            i = 0
            
            while i < len(data) - 2:
                scancode_high = data[i]
                scancode_low = data[i + 1]
                scancode = (scancode_high << 8) | scancode_low
                
                if scancode == 0:
                    break
                    
                i += 2
                if i >= len(data):
                    break
                    
                value = data[i]
                i += 1
                depth = value / 255.0
                
                rawcode = HID_TO_VK.get(scancode, 0)
                if rawcode == 0 and scancode_low > 0:
                    rawcode = HID_TO_VK.get(scancode_low, 0)
                
                if rawcode > 0 and depth > 0.01:
                    if self.is_allowed(rawcode):
                        active_keys.append({'scancode': scancode, 'rawcode': rawcode, 'depth': round(depth, 2)})
            
            if active_keys:
                for key in active_keys:
                    self.queue_message({
                        "event_type": "analog_depth",
                        "rawcode": key['rawcode'],
                        "depth": key['depth']
                    })
        except Exception as e:
            logger.error(f"error processing Wooting data: {e}")

    def process_razer_huntsman_data(self, data: list):
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
                    
                value = data[i]
                i += 1
                
                hid_sc = RAZER_TO_HID.get(razer_sc, 0)
                if hid_sc > 0:
                    rawcode = HID_TO_VK.get(hid_sc, 0)
                    depth = value / 255.0
                    if rawcode > 0 and depth > 0.01:
                        if self.is_allowed(rawcode):
                            active_keys.append({'razer_sc': razer_sc, 'hid_sc': hid_sc, 'rawcode': rawcode, 'depth': round(depth, 2)})
            
            if active_keys:
                for key in active_keys:
                    self.queue_message({
                        "event_type": "analog_depth",
                        "rawcode": key['rawcode'],
                        "depth": key['depth']
                    })
        except Exception as e:
            logger.error(f"error processing Razer data: {e}")

    def process_razer_huntsman_v3_data(self, data: list):
        try:
            active_keys = []
            i = 0
            
            while i < len(data) - 2:
                razer_sc = data[i]
                if razer_sc == 0:
                    break
                i += 1
                if i >= len(data):
                    break
                    
                value = data[i]
                i += 2
                
                hid_sc = RAZER_TO_HID.get(razer_sc, 0)
                if hid_sc > 0:
                    rawcode = HID_TO_VK.get(hid_sc, 0)
                    depth = value / 255.0
                    if rawcode > 0 and depth > 0.01:
                        if self.is_allowed(rawcode):
                            active_keys.append({'razer_sc': razer_sc, 'hid_sc': hid_sc, 'rawcode': rawcode, 'depth': round(depth, 2)})
            
            if active_keys:
                for key in active_keys:
                    self.queue_message({
                        "event_type": "analog_depth",
                        "rawcode": key['rawcode'],
                        "depth": key['depth']
                    })
        except Exception as e:
            logger.error(f"error processing Razer V3 data: {e}")

    NUPHY_TO_HID = {
        0x100: 0xE0, 0x200: 0xE1, 0x400: 0xE2, 0x800: 0xE3,
        0x1000: 0xE4, 0x2000: 0xE5, 0x4000: 0xE6, 0x8000: 0xE7,
        0xff05: 0x409,
    }

    def _nuphy_to_hid(self, scancode: int) -> int:
        return self.NUPHY_TO_HID.get(scancode, scancode)

    def process_nuphy_data(self, data: list, buffer: dict):
        try:
            nuphy_sc = (data[2] << 8) | data[3]
            hid_sc = self._nuphy_to_hid(nuphy_sc)
            if hid_sc == 0:
                return
            rawcode = HID_TO_VK.get(hid_sc, 0)
            if rawcode == 0:
                return
            value = data[7]
            if value == 0:
                buffer.pop(rawcode, None)
            else:
                depth = min(value / 200.0, 1.0)
                if self.is_allowed(rawcode):
                    buffer[rawcode] = round(depth, 2)
                    self.queue_message({"event_type": "analog_depth", "rawcode": rawcode, "depth": round(depth, 2)})
        except Exception as e:
            logger.error(f"error processing NuPhy data: {e}")

    DRUNKDEER_INDEX_TO_HID = {
        (0*21)+0: 0x29,  (0*21)+2: 0x3A,  (0*21)+3: 0x3B,  (0*21)+4: 0x3C,
        (0*21)+5: 0x3D,  (0*21)+6: 0x3E,  (0*21)+7: 0x3F,  (0*21)+8: 0x40,
        (0*21)+9: 0x41,  (0*21)+10: 0x42, (0*21)+11: 0x43, (0*21)+12: 0x44,
        (0*21)+13: 0x45, (0*21)+14: 0x4C,
        (1*21)+0: 0x35,  (1*21)+1: 0x1E,  (1*21)+2: 0x1F,  (1*21)+3: 0x20,
        (1*21)+4: 0x21,  (1*21)+5: 0x22,  (1*21)+6: 0x23,  (1*21)+7: 0x24,
        (1*21)+8: 0x25,  (1*21)+9: 0x26,  (1*21)+10: 0x27, (1*21)+11: 0x2D,
        (1*21)+12: 0x2E, (1*21)+13: 0x2A, (1*21)+15: 0x4A,
        (2*21)+0: 0x2B,  (2*21)+1: 0x14,  (2*21)+2: 0x1A,  (2*21)+3: 0x08,
        (2*21)+4: 0x15,  (2*21)+5: 0x17,  (2*21)+6: 0x1C,  (2*21)+7: 0x18,
        (2*21)+8: 0x0C,  (2*21)+9: 0x12,  (2*21)+10: 0x13, (2*21)+11: 0x2F,
        (2*21)+12: 0x30, (2*21)+13: 0x31, (2*21)+15: 0x4B,
        (3*21)+0: 0x39,  (3*21)+1: 0x04,  (3*21)+2: 0x16,  (3*21)+3: 0x07,
        (3*21)+4: 0x09,  (3*21)+5: 0x0A,  (3*21)+6: 0x0B,  (3*21)+7: 0x0D,
        (3*21)+8: 0x0E,  (3*21)+9: 0x0F,  (3*21)+10: 0x33, (3*21)+11: 0x34,
        (3*21)+13: 0x28, (3*21)+15: 0x4E,
        (4*21)+0: 0xE1,  (4*21)+2: 0x1D,  (4*21)+3: 0x1B,  (4*21)+4: 0x06,
        (4*21)+5: 0x19,  (4*21)+6: 0x05,  (4*21)+7: 0x11,  (4*21)+8: 0x10,
        (4*21)+9: 0x36,  (4*21)+10: 0x37, (4*21)+11: 0x38, (4*21)+13: 0xE5,
        (4*21)+14: 0x52, (4*21)+15: 0x4D,
        (5*21)+0: 0xE0,  (5*21)+1: 0xE3,  (5*21)+2: 0xE2,  (5*21)+6: 0x2C,
        (5*21)+10: 0xE6, (5*21)+11: 0x409,(5*21)+12: 0x65, (5*21)+14: 0x50,
        (5*21)+15: 0x51, (5*21)+16: 0x4F,
    }

    def process_drunkdeer_data(self, data: list, active_keys_buf: list):
        try:
            n = data[3]
            if n == 0:
                active_keys_buf.clear()
            stride = 64 - 5
            for i in range(4, len(data)):
                value = data[i]
                idx = n * stride + (i - 4)
                if value != 0:
                    hid_sc = self.DRUNKDEER_INDEX_TO_HID.get(idx, 0)
                    if hid_sc != 0:
                        rawcode = HID_TO_VK.get(hid_sc, 0)
                        if rawcode != 0:
                            depth = min(value / 40.0, 1.0)
                            if self.is_allowed(rawcode):
                                active_keys_buf.append({"rawcode": rawcode, "depth": round(depth, 2)})
            if n == 2:
                for key in active_keys_buf:
                    self.queue_message({"event_type": "analog_depth", "rawcode": key["rawcode"], "depth": key["depth"]})
        except Exception as e:
            logger.error(f"error processing DrunkDeer data: {e}")

    MADLIONS_LAYOUT_60 = [
        0x29, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2D, 0x2E, 0x2A,
        0x2B, 0x14, 0x1A, 0x08, 0x15, 0x17, 0x1C, 0x18, 0x0C, 0x12, 0x13, 0x2F, 0x30, 0x31,
        0x39, 0x04, 0x16, 0x07, 0x09, 0x0A, 0x0B, 0x0D, 0x0E, 0x0F, 0x33, 0x34, 0x00, 0x28,
        0xE1, 0x00, 0x1D, 0x1B, 0x06, 0x19, 0x05, 0x11, 0x10, 0x36, 0x37, 0x38, 0x00, 0xE5,
        0xE0, 0xE3, 0xE2, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00, 0xE7, 0xE6, 0x65, 0xE4, 0x409,
    ]
    MADLIONS_LAYOUT_68 = [
        0x29, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2D, 0x2E, 0x2A, 0x49,
        0x2B, 0x14, 0x1A, 0x08, 0x15, 0x17, 0x1C, 0x18, 0x0C, 0x12, 0x13, 0x2F, 0x30, 0x31, 0x4C,
        0x39, 0x04, 0x16, 0x07, 0x09, 0x0A, 0x0B, 0x0D, 0x0E, 0x0F, 0x33, 0x34, 0x00, 0x28, 0x4B,
        0xE1, 0x00, 0x1D, 0x1B, 0x06, 0x19, 0x05, 0x11, 0x10, 0x36, 0x37, 0x38, 0xE5, 0x52, 0x4E,
        0xE0, 0xE3, 0xE2, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00, 0xE6, 0x409, 0xE4, 0x50, 0x51, 0x4F,
    ]

    def process_madlions_data(self, data: list, buffer: dict, offset: list,
                               layout_size: int, device, init_buf: list, pid: int):
        try:
            layout = self.MADLIONS_LAYOUT_60 if pid in [0x1055, 0x1056, 0x105D] else self.MADLIONS_LAYOUT_68
            for i in range(4):
                li = offset[0] + i
                if li < len(layout):
                    hid_sc = layout[li]
                    travel = (data[7 + i*5 + 3] << 8) | data[7 + i*5 + 4]
                    rawcode = HID_TO_VK.get(hid_sc, 0)
                    if rawcode != 0:
                        if travel == 0:
                            buffer.pop(rawcode, None)
                        else:
                            depth = min(travel / 350.0, 1.0)
                            if self.is_allowed(rawcode):
                                buffer[rawcode] = round(depth, 2)

            for rawcode, depth in list(buffer.items()):
                self.queue_message({"event_type": "analog_depth", "rawcode": rawcode, "depth": depth})

            offset[0] += 4
            if offset[0] >= layout_size:
                offset[0] = 0
            init_buf[6] = offset[0]
            try:
                device.write(init_buf)
            except Exception:
                pass
        except Exception as e:
            logger.error(f"error processing Madlions data: {e}")

    BYTECH_TO_HID = {
        1: 0x29, 2: 0x3A, 3: 0x3B, 4: 0x3C, 5: 0x3D, 6: 0x3E, 7: 0x3F,
        8: 0x40, 9: 0x41, 10: 0x42, 11: 0x43, 12: 0x44, 13: 0x45,
        14: 0x35, 15: 0x1E, 16: 0x1F, 17: 0x20, 18: 0x21, 19: 0x22,
        20: 0x23, 21: 0x24, 22: 0x25, 23: 0x26, 24: 0x27, 25: 0x2D,
        26: 0x2E, 27: 0x2A, 28: 0x2B, 29: 0x14, 30: 0x1A, 31: 0x08,
        32: 0x15, 33: 0x17, 34: 0x1C, 35: 0x18, 36: 0x0C, 37: 0x12,
        38: 0x13, 39: 0x2F, 40: 0x30, 41: 0x31, 42: 0x39, 43: 0x04,
        44: 0x16, 45: 0x07, 46: 0x09, 47: 0x0A, 48: 0x0B, 49: 0x0D,
        50: 0x0E, 51: 0x0F, 52: 0x33, 53: 0x34, 54: 0x28, 55: 0xE1,
        56: 0x1D, 57: 0x1B, 58: 0x06, 59: 0x19, 60: 0x05, 61: 0x11,
        62: 0x10, 63: 0x36, 64: 0x37, 65: 0x38, 66: 0xE5, 67: 0xE0,
        68: 0xE3, 69: 0xE2, 70: 0x2C, 71: 0xE6, 72: 0x409, 73: 0xE4,
        74: 0x52, 75: 0x51, 76: 0x50, 77: 0x4F,
        99: 0x4C, 100: 0x4A, 102: 0x4B, 103: 0x4E,
    }

    @staticmethod
    def _build_bytech_payload(cmd: int, sub: int) -> bytes:
        buf = bytearray(63)
        buf[0] = cmd
        buf[1] = sub
        total = 9
        for b in buf[:-1]:
            total += b
        buf[-1] = (255 - (total % 256)) & 0xff
        return bytes(buf)

    def process_bytech_data(self, data: list, buffer: dict):
        try:
            count = data[6]
            new_buffer = {}
            for i in range(0, count, 4):
                if 7 + i + 4 > len(data):
                    break
                pos      = data[8 + i]
                distance = (data[9 + i] << 8) | data[10 + i]
                hid_sc = self.BYTECH_TO_HID.get(pos, 0)
                if hid_sc == 0:
                    continue
                rawcode = HID_TO_VK.get(hid_sc, 0)
                if rawcode == 0 or not self.is_allowed(rawcode):
                    continue
                depth = round(min(distance / 355.0, 1.0), 2) if distance > 10 else 0.0
                new_buffer[rawcode] = depth

            for rawcode in buffer:
                if rawcode not in new_buffer:
                    self.queue_message({"event_type": "analog_depth", "rawcode": rawcode, "depth": 0.0})
            for rawcode, depth in new_buffer.items():
                if buffer.get(rawcode) != depth:
                    self.queue_message({"event_type": "analog_depth", "rawcode": rawcode, "depth": depth})
            buffer.clear()
            buffer.update({k: v for k, v in new_buffer.items() if v > 0.0})
        except Exception as e:
            logger.error(f"error processing Bytech data: {e}")

    def get_analog_devices(self) -> list:
        try:
            import hid
            devices = []
            
            analog_keyboards = [
                (0x31E3, None, "Wooting", 0xFF54),
                (0x03EB, 0xFF01, "Wooting One", 0xFF54),
                (0x03EB, 0xFF02, "Wooting Two", 0xFF54),
                (0x1532, 0x0266, "Razer Huntsman V2 Analog", None),
                (0x1532, 0x0282, "Razer Huntsman Mini Analog", None),
                (0x1532, 0x02a6, "Razer Huntsman V3 Pro", None),
                (0x1532, 0x02a7, "Razer Huntsman V3 Pro TKL", None),
                (0x1532, 0x02b0, "Razer Huntsman V3 Pro Mini", None),
                (0x19f5, None, "NuPhy", 0x0001),
                (0x352D, None, "DrunkDeer", 0xFF00),
                (0x3434, None, "Keychron HE", 0xFF60),
                (0x362D, None, "Lemokey HE", 0xFF60),
                (0x373b, None, "Madlions HE", 0xFF60),
                (0x372E, 0x105B, "Redragon K709 HE", 0xFF60),
            ]
            
            logger.info("scanning for analog keyboards...")
            all_devices = hid.enumerate()
            seen_vidpid = set()

            for device_dict in all_devices:
                vid = device_dict['vendor_id']
                pid = device_dict['product_id']
                usage_page = device_dict.get('usage_page', 0)
                interface = device_dict.get('interface_number', -1)
                path = device_dict.get('path', b'').decode('utf-8', errors='ignore')
                
                for known_vid, known_pid, name, required_usage in analog_keyboards:
                    if vid == known_vid and (known_pid is None or pid == known_pid):
                        if required_usage is not None and usage_page != required_usage:
                            if sys.platform == 'win32' or usage_page != 0:
                                continue

                        if required_usage is None:
                            vidpid_key = (vid, pid)
                            if vidpid_key in seen_vidpid:
                                break
                            seen_vidpid.add(vidpid_key)
                        
                        device_str = f"{vid:04x}:{pid:04x}:{interface}" if interface >= 0 else f"{vid:04x}:{pid:04x}"
                        product_name = device_dict.get('product_string', name)
                        
                        logger.info(f"found: {product_name} ({device_str}) usage_page=0x{usage_page:04x} interface={interface}")
                        
                        devices.append({
                            'id': device_str,
                            'name': f"{product_name} ({device_str}) [usage:0x{usage_page:04x}]",
                            'interface': interface,
                            'usage_page': usage_page,
                            'path': path
                        })
                        break
            
            logger.info(f"found {len(devices)} analog keyboard interface(s)")
            return devices
        except ImportError:
            logger.error("hidapi not installed")
            return []
        except Exception as e:
            logger.error(f"error enumerating devices: {e}")
            import traceback
            traceback.print_exc()
            return []

    async def handle_client(self, websocket):
        self.clients.add(websocket)

        try:
            remote_address = websocket.remote_address
            client_ip = remote_address[0] if remote_address else "unknown"
            client_port = remote_address[1] if remote_address else "unknown"
        except Exception:
            client_ip = "unknown"
            client_port = "unknown"

        try:
            origin = websocket.request.headers.get('Origin', 'N/A')
            user_agent = websocket.request.headers.get('User-Agent', 'N/A')
        except Exception:
            origin = 'N/A'
            user_agent = 'N/A'

        logger.info(f"new connection from {client_ip}:{client_port} - origin: {origin}")

        try:
            async for message in websocket:
                try:
                    data = json.loads(message)
                    msg_type = data.get('type')
                    logger.debug(f"message from {client_ip}:{client_port} - type={msg_type!r}")

                    if msg_type == 'auth':
                        token = data.get('token', '')
                        if not token:
                            logger.warning(f"auth rejected from {client_ip}:{client_port} - no token provided")
                            await websocket.send(json.dumps({'type': 'auth_response', 'status': 'failed'}))
                            await websocket.close()
                        elif not self.auth_token or token == self.auth_token:
                            self.authenticated_clients.add(websocket)
                            await websocket.send(json.dumps({'type': 'auth_response', 'status': 'success'}))
                            client_count = len(self.authenticated_clients)
                            logger.info(f"client authenticated from {client_ip}:{client_port} (total authed: {client_count})")
                            self.show_toast_notification(
                                "a new client connected",
                                f"origin: {origin}\nuseragent: {user_agent}\nactive connections: {client_count}"
                            )
                        else:
                            logger.warning(f"auth rejected from {client_ip}:{client_port} - token mismatch")
                            await websocket.send(json.dumps({'type': 'auth_response', 'status': 'failed'}))
                            self.show_toast_notification(
                                "authentication failed",
                                f"ip: {client_ip}:{client_port}\nuseragent: {user_agent}\norigin: {origin}\nreason: bad token"
                            )
                            await websocket.close()
                    else:
                        logger.debug(f"unhandled message type {msg_type!r} from {client_ip}:{client_port}")

                except json.JSONDecodeError as e:
                    logger.warning(f"invalid json from {client_ip}:{client_port}: {e}")

        except websockets.exceptions.ConnectionClosedOK as e:
            logger.info(f"connection closed cleanly: {client_ip}:{client_port} code={e.code} reason={e.reason!r}")
        except websockets.exceptions.ConnectionClosedError as e:
            logger.warning(f"connection closed with error: {client_ip}:{client_port} code={e.code} reason={e.reason!r}")
        except websockets.exceptions.ConnectionClosed as e:
            logger.info(f"connection closed: {client_ip}:{client_port} code={e.code} reason={e.reason!r}")
        except Exception as e:
            logger.error(f"unexpected error handling {client_ip}:{client_port}: {e}", exc_info=True)
        finally:
            was_authed = websocket in self.authenticated_clients
            self.clients.discard(websocket)
            self.authenticated_clients.discard(websocket)
            if was_authed:
                remaining = len(self.authenticated_clients)
                logger.info(f"authenticated client disconnected: {client_ip}:{client_port} (remaining: {remaining})")
                self.show_toast_notification(
                    "client disconnected",
                    f"ip: {client_ip}:{client_port}\nremaining connections: {remaining}"
                )
            else:
                logger.debug(f"unauthenticated client removed: {client_ip}:{client_port}")

    def start_input_listeners(self):
        self.keyboard_listener = keyboard.Listener(on_press=self.on_key_press, on_release=self.on_key_release)
        self.keyboard_listener.start()
        self.mouse_listener = mouse.Listener(on_click=self.on_mouse_click, on_scroll=self.on_mouse_scroll)
        self.mouse_listener.start()

    def stop_input_listeners(self):
        if self.keyboard_listener:
            self.keyboard_listener.stop()
        if self.mouse_listener:
            self.mouse_listener.stop()

    async def start(self):
        self.loop = asyncio.get_event_loop()
        self.running = True
        
        self.queue_processor_task = asyncio.create_task(self.process_message_queue())
        self.start_input_listeners()
        self.start_analog_support()
        if self.raw_mouse_enabled:
            self.start_raw_mouse()
        
        async with websockets.serve(self.handle_client, self.host, self.port):
            logger.info(f"server started on ws://{self.host}:{self.port}")
            if self.auth_token:
                logger.info("auth token: %s****", self.auth_token[:4] if len(self.auth_token) >= 4 else self.auth_token)
            else:
                logger.warning("auth disabled")
            
            if self.analog_enabled:
                logger.info(f"analog support: enabled")
                if self.analog_device:
                    logger.info(f"analog device: {self.analog_device}")
                else:
                    logger.warning("no analog device selected")
            else:
                logger.info(f"analog support: disabled")
            
            while self.running:
                await asyncio.sleep(5)
                self.reload_config_if_changed()
            
            if self.queue_processor_task:
                self.queue_processor_task.cancel()
                try:
                    await self.queue_processor_task
                except asyncio.CancelledError:
                    pass
            
            self.stop_input_listeners()
            self.stop_analog_support()
            self.stop_raw_mouse()

    def stop(self):
        self.running = False


def get_resource_path(relative_path):
    try:
        base_path = sys._MEIPASS
    except Exception:
        base_path = Path(__file__).parent
    return Path(base_path) / relative_path


def create_tray_icon():
    icon_path = get_resource_path("assets/icon.ico")
    if icon_path.exists():
        return Image.open(icon_path)
    else:
        img = Image.new('RGB', (64, 64), color='blue')
        d = ImageDraw.Draw(img)
        d.rectangle([16, 16, 48, 48], fill='white')
        return img


def run_server(server):
    try:
        asyncio.run(server.start())
    except OSError as e:
        # errno 10048 is win, errno 98 is linux
        if e.errno in (10048, 98):
            logger.error(f"port {server.port} already in use")
        else:
            logger.error(f"server error: {e}")
    except Exception as e:
        logger.error(f"server error: {e}")


def run_settings_editor_subprocess(config_path="config.json"):
    try:
        if getattr(sys, 'frozen', False):
            exe_path = sys.executable
            proc = subprocess.Popen([exe_path, "--settings", config_path])
            logger.info("launched settings editor (exe)")
        else:
            script_path = Path(__file__).parent / "services" / "settings.py"
            proc = subprocess.Popen([sys.executable, str(script_path), config_path])
            logger.info("launched settings editor (script)")
        return proc
    except Exception as e:
        logger.error(f"failed to launch settings editor: {e}")
        import traceback
        traceback.print_exc()
        return None


def main():
    if '-debug' in sys.argv:
        if getattr(sys, 'frozen', False) and sys.platform == 'win32':
            import ctypes
            ctypes.windll.kernel32.AllocConsole()
            sys.stdout = open('CONOUT$', 'w')
            sys.stderr = open('CONOUT$', 'w')
            root = logging.getLogger()
            for handler in root.handlers[:]:
                root.removeHandler(handler)
            handler = logging.StreamHandler(sys.stdout)
            handler.setFormatter(logging.Formatter('%(asctime)s - %(levelname)s - %(message)s'))
            handler.addFilter(_RedactingFilter())
            root.addHandler(handler)
        elif not getattr(sys, 'frozen', False):
            pass
        logging.getLogger().setLevel(logging.DEBUG)

    if len(sys.argv) >= 2 and sys.argv[1] == "--settings":
        config_path = sys.argv[2] if len(sys.argv) >= 3 else "config.json"
        from services.settings import run_settings_editor
        run_settings_editor(config_path)
        return

    if len(sys.argv) >= 2 and sys.argv[1] == "--update-popup":
        latest = sys.argv[2] if len(sys.argv) >= 3 else ""
        config_path = sys.argv[3] if len(sys.argv) >= 4 else "config.json"
        import os
        release_body = os.environ.get("IOV_UPDATE_BODY", "")
        from services.settings import _run_update_popup_process
        _run_update_popup_process(latest, config_path, release_body)
        return

    server = InputOverlayServer()
    config = server.load_config()
    server.host = config.get('host', 'localhost')
    server.port = config.get('port', 16899)
    server.auth_token = config.get('auth_token', '')
    server.analog_enabled = config.get('analog_enabled', False)
    server.analog_device = config.get('analog_device', None)
    server.key_whitelist = config.get('key_whitelist', [])
    server.balloon_notifications = config.get('balloon_notifications', True)
    server.raw_mouse_enabled = config.get('raw_mouse_enabled', False)
    server.raw_mouse_min_delta = config.get('raw_mouse_min_delta', 0)

    if sys.platform == 'win32':
        cpu_affinity = config.get('cpu_affinity', [0, 1])
        if isinstance(cpu_affinity, list) and cpu_affinity:
            mask = 0
            for core in cpu_affinity:
                mask |= (1 << core)
            import ctypes
            ctypes.windll.kernel32.SetProcessAffinityMask(
                ctypes.windll.kernel32.GetCurrentProcess(), mask
            )

    server_thread = threading.Thread(target=run_server, args=(server,), daemon=True)
    server_thread.start()

    child_processes = []
    settings_proc = [None]

    from services.settings import check_for_updates_on_startup
    check_for_updates_on_startup("config.json", child_processes)

    def on_quit(icon, item):
        logger.info("shutting down...")
        server.stop()
        for proc in child_processes:
            try:
                proc.terminate()
            except Exception:
                pass
        icon.stop()

    def on_settings(icon, item):
        if settings_proc[0] is not None and settings_proc[0].poll() is None:
            return
        proc = run_settings_editor_subprocess("config.json")
        if proc:
            settings_proc[0] = proc
            child_processes.append(proc)

    def create_menu():
        return pystray.Menu(
            pystray.MenuItem("Settings", on_settings),
            pystray.MenuItem("Exit", on_quit)
        )

    try:
        icon = pystray.Icon(
            "input_overlay",
            create_tray_icon(),
            "Input Overlay Server",
            menu=create_menu()
        )
        logger.info("starting tray icon...")
        icon.run()
    except Exception as e:
        logger.error(f"tray icon error: {e}")
        input("press enter to exit...")


if __name__ == "__main__":
    main()