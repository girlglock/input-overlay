from __future__ import annotations

import asyncio
import json
import logging
import secrets
import sys
import threading
from pathlib import Path
from queue import Queue
from typing import Set

import websockets

from services.consts import (
    MOUSE_BUTTON_NAMES,
    MOUSE_SCROLL_NAMES,
    RAW_CODE_TO_KEY_NAME,
    RAW_MOUSE_FLUSH_HZ,
)
from services.http_server import LocalHTTPServer
from services.logger import flush_log
from services.utils import get_resource_path, get_web_root, spawn_subprocess

if sys.platform == "win32":
    from services.pynput_input import PynputInputListener
    from services.analog import AnalogHandler, enum_analog_devices

    try:
        from services.rawinput import RawMouseThread
        RAW_MOUSE_AVAILABLE = True
    except Exception as _e:
        logging.getLogger(__name__).warning("raw mouse unavailable: %s", _e)
        RAW_MOUSE_AVAILABLE = False

    try:
        from winotify import Notification, audio as _wn_audio
        TOAST_AVAILABLE = True
    except ImportError:
        TOAST_AVAILABLE = False
        logging.getLogger(__name__).warning("winotify not installed - toast notifications disabled")

    ANALOG_AVAILABLE = True

else:
    TOAST_AVAILABLE = False

    import shutil as _shutil
    NOTIFY_SEND_AVAILABLE = _shutil.which("notify-send") is not None
    if not NOTIFY_SEND_AVAILABLE:
        logging.getLogger(__name__).warning("notify-send not found no notis for now")

    try:
        from services.evdev_input import EvdevInputListener
        EVDEV_AVAILABLE = True
    except ImportError:
        EVDEV_AVAILABLE = False
        logging.getLogger(__name__).warning("evdev_input not found - input will not work")

    try:
        from services.analog import AnalogHandler, enum_analog_devices
        ANALOG_AVAILABLE = True
    except ImportError:
        ANALOG_AVAILABLE = False
        logging.getLogger(__name__).warning("analog module not found")

    try:
        from services.rawinput_linux import RawMouseLinuxThread as RawMouseThread
        RAW_MOUSE_AVAILABLE = True
    except ImportError:
        RAW_MOUSE_AVAILABLE = False
        logging.getLogger(__name__).warning("rawinput_linux not found - raw mouse disabled")

logger = logging.getLogger(__name__)

_seen_events: set = set()

class InputOverlayServer:
    def __init__(self) -> None:
        self.host: str  = "localhost"
        self.port: int  = 4455
        self.http_enabled: bool = False
        self.http_port: int  = 4456
        self.auth_token: str = ""

        self._http_server: LocalHTTPServer | None = None

        self.clients: Set[websockets.WebSocketServerProtocol]               = set()
        self.authenticated_clients: Set[websockets.WebSocketServerProtocol] = set()

        self._input_listener   = None
        self.running           = False
        self.loop: asyncio.AbstractEventLoop | None = None
        self.message_queue     = Queue()
        self.queue_processor_task = None

        self.analog_enabled    = False
        self.analog_device: str | None = None
        self._analog = None

        self.key_whitelist: list = []
        self.balloon_notifications = True
        self.config_path       = "config.json"
        self.config_last_modified: float = 0

        self.raw_mouse_enabled   = False
        self.raw_mouse_min_delta = 0
        self.linux_raw_mouse_device: str = ""
        self._raw_mouse_thread   = None

        self.child_processes: list = []

    def show_toast_notification(self, title: str, message: str, duration: str = "short") -> None:
        if not self.balloon_notifications:
            return
        if sys.platform == "win32":
            if not TOAST_AVAILABLE:
                return
            try:
                icon_path = get_resource_path("assets/icon.ico")
                toast = Notification(
                    app_id="Input Overlay",
                    title=title,
                    msg=message,
                    icon=str(icon_path) if icon_path.exists() else "",
                )
                toast.set_audio(_wn_audio.Reminder, loop=False)
                toast.show()
            except Exception:
                logger.exception("failed to show toast notification")
        else:
            if not NOTIFY_SEND_AVAILABLE:
                return
            try:
                expire_ms = "3000" if duration == "short" else "6000"
                icon_path = get_resource_path("assets/icon.ico")
                icon_arg  = str(icon_path) if icon_path.exists() else "dialog-information"
                import subprocess as _sp
                _sp.Popen(
                    ["notify-send", "--app-name=Input Overlay",
                     f"--expire-time={expire_ms}", f"--icon={icon_arg}",
                     title, message],
                    stdout=_sp.DEVNULL,
                    stderr=_sp.DEVNULL,
                )
            except Exception:
                logger.exception("failed to show linux notification")

    def load_config(self, config_path: str = "config.json") -> dict:
        config_file = Path(config_path)
        try:
            if config_file.exists():
                with open(config_file, "r") as f:
                    config = json.load(f)
                logger.info("loaded config from %s", config_path)
                self.config_last_modified = config_file.stat().st_mtime
                return config

            random_token = secrets.token_urlsafe(32)
            default_config = {
                "host":                  "localhost",
                "port":                  4455,
                "http_enabled":          False,
                "http_port":             4456,
                "auth_token":            random_token,
                "analog_enabled":        False,
                "analog_device":         None,
                "key_whitelist":         [],
                "balloon_notifications": False,
                "dismissed_versions":    [],
                "raw_mouse_enabled":     sys.platform == "win32",
                "raw_mouse_min_delta":   0,
                "linux_raw_mouse_device": "",
                "cpu_affinity":          [0, 1],
            }
            with open(config_file, "w") as f:
                json.dump(default_config, f, indent=4)
            logger.info("generated auth token: %s****", random_token[:4])
            logger.info("add to overlay url: &wsauth=%s", random_token)
            self.config_last_modified = config_file.stat().st_mtime
            return default_config
        except Exception:
            logger.exception("error loading config")
            return {}

    def reload_config_if_changed(self) -> bool:
        try:
            config_file   = Path(self.config_path)
            shutdown_flag = config_file.parent / "shutdown.flag"
            rebind_flag   = config_file.parent / "restart.flag"

            if shutdown_flag.exists():
                try:
                    shutdown_flag.unlink()
                except Exception:
                    pass
                logger.info("shutdown flag detected, shutting down for update...")
                self.stop()
                return False

            if rebind_flag.exists():
                try:
                    rebind_flag.unlink()
                except Exception:
                    pass
                config   = self.load_config(self.config_path)
                new_host = config.get("host", self.host)
                new_port = config.get("port", self.port)
                if new_host != self.host or new_port != self.port:
                    logger.info("rebind requested: %s:%d -> %s:%d", self.host, self.port, new_host, new_port)
                    self._prev_host = self.host
                    self._prev_port = self.port
                    self.host = new_host
                    self.port = new_port
                    if self.loop and hasattr(self, "_rebind_event"):
                        self.loop.call_soon_threadsafe(self._rebind_event.set)
                return True

            if config_file.exists():
                current_mtime = config_file.stat().st_mtime
                if current_mtime > self.config_last_modified:
                    logger.info("config file changed, reloading...")
                    config = self.load_config(self.config_path)

                    old_analog_enabled = self.analog_enabled
                    old_analog_device  = self.analog_device
                    old_raw_mouse      = self.raw_mouse_enabled
                    old_linux_device   = self.linux_raw_mouse_device
                    old_auth_token     = self.auth_token
                    old_http_enabled   = self.http_enabled
                    old_http_port      = self.http_port

                    self.auth_token              = config.get("auth_token", self.auth_token)
                    self.analog_enabled          = config.get("analog_enabled", False)
                    self.analog_device           = config.get("analog_device", None)
                    self.key_whitelist           = config.get("key_whitelist", [])
                    self.balloon_notifications   = config.get("balloon_notifications", True)
                    self.raw_mouse_enabled       = config.get("raw_mouse_enabled", False)
                    self.raw_mouse_min_delta     = config.get("raw_mouse_min_delta", 0)
                    self.linux_raw_mouse_device  = config.get("linux_raw_mouse_device", "")
                    self.http_enabled            = config.get("http_enabled", False)
                    self.http_port               = config.get("http_port", 4456)

                    logger.info("settings updated - analog: %s, device: %s", self.analog_enabled, self.analog_device)
                    logger.info("whitelist: %d keys", len(self.key_whitelist))

                    if old_analog_enabled != self.analog_enabled or old_analog_device != self.analog_device:
                        if old_analog_enabled:
                            self.stop_analog_support()
                        if self.analog_enabled:
                            self.start_analog_support()

                    if sys.platform == "win32":
                        if old_raw_mouse != self.raw_mouse_enabled:
                            if old_raw_mouse:
                                self.stop_raw_mouse()
                            if self.raw_mouse_enabled:
                                self.start_raw_mouse()
                    else:
                        if old_linux_device != self.linux_raw_mouse_device:
                            self.stop_raw_mouse()
                            if self.linux_raw_mouse_device:
                                self.start_raw_mouse()

                    if old_http_enabled != self.http_enabled or old_http_port != self.http_port:
                        self.stop_http_server()
                        if self.http_enabled:
                            self.start_http_server()

                    if old_auth_token != self.auth_token and self.authenticated_clients:
                        logger.info("auth token changed, kicking existing clients...")
                        if self.loop:
                            async def _kick_clients():
                                for ws in list(self.authenticated_clients):
                                    try:
                                        await ws.close(1008, "auth token changed")
                                    except Exception:
                                        pass
                                self.authenticated_clients.clear()
                            asyncio.run_coroutine_threadsafe(_kick_clients(), self.loop)

                    return True
        except Exception:
            logger.exception("error reloading config")
        return False

    def save_config(self, config_path: str = "config.json") -> None:
        try:
            config = {
                "host":           self.host,
                "port":           self.port,
                "auth_token":     self.auth_token,
                "analog_enabled": self.analog_enabled,
                "analog_device":  self.analog_device,
                "key_whitelist":  self.key_whitelist,
            }
            with open(config_path, "w") as f:
                json.dump(config, f, indent=4)
            self.config_last_modified = Path(config_path).stat().st_mtime
            logger.info("saved config to %s", config_path)
        except Exception:
            logger.exception("error saving config")

    def _revert_config(self, host: str, port: int) -> None:
        try:
            config_file = Path(self.config_path)
            if config_file.exists():
                with open(config_file, "r") as f:
                    config = json.load(f)
                config["host"] = host
                config["port"] = port
                with open(config_file, "w") as f:
                    json.dump(config, f, indent=4)
                self.config_last_modified = config_file.stat().st_mtime
                logger.info("config reverted to %s:%d", host, port)
        except Exception:
            logger.exception("failed to revert config")

    def _spawn_rebind_failed(self, failed_host: str, failed_port: int,
                              kind: str, prev_host: str, prev_port: int) -> None:
        proc = spawn_subprocess(
            "--rebind-failed", kind, failed_host, str(failed_port), prev_host, str(prev_port)
        )
        if proc:
            self.child_processes.append(proc)

    def _spawn_port_error(self, kind: str) -> None:
        proc = spawn_subprocess("--port-error", kind, self.host, str(self.port), self.config_path)
        if proc:
            self.child_processes.append(proc)

    def _write_clients_file(self) -> None:
        try:
            clients = []
            for ws in self.authenticated_clients:
                try:
                    addr = ws.remote_address
                    clients.append({"ip": addr[0] if addr else "unknown", "port": addr[1] if addr else 0})
                except Exception:
                    pass
            clients_file = Path(self.config_path).parent / "clients.json"
            with open(clients_file, "w") as f:
                json.dump(clients, f, indent=4)
        except Exception as e:
            logger.debug("failed to write clients file: %s", e)

    def is_allowed(self, code: int, is_mouse: bool = False,
                   is_scroll: bool = False, is_mouse_move: bool = False) -> bool:
        if not self.key_whitelist:
            return True
        if is_mouse_move and self.raw_mouse_enabled:
            return True
        if is_scroll:
            if "mouse_wheel" in self.key_whitelist:
                return True
            name = MOUSE_SCROLL_NAMES.get(code)
        elif is_mouse:
            name = MOUSE_BUTTON_NAMES.get(code)
        else:
            name = RAW_CODE_TO_KEY_NAME.get(code)
        return bool(name and name in self.key_whitelist)

    async def broadcast(self, message: dict) -> None:
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
                logger.warning("error sending to client: %s", e)
                disconnected.add(client)
        for client in disconnected:
            self.authenticated_clients.discard(client)
            self.clients.discard(client)

    def queue_message(self, message: dict) -> None:
        try:
            event_type = message.get("event_type")
            if event_type and event_type not in _seen_events:
                _seen_events.add(event_type)
                logger.info("%s detected", event_type.replace("_", " "))
            self.message_queue.put(message)
        except Exception as e:
            logger.error("error queuing message: %s", e)

    async def process_message_queue(self) -> None:
        logger.debug("message queue processor started")
        FLUSH_INTERVAL = 1.0 / RAW_MOUSE_FLUSH_HZ
        last_flush = asyncio.get_event_loop().time()
        pending_dx = pending_dy = 0
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
                if (pending_dx or pending_dy) and (now - last_flush) >= FLUSH_INTERVAL:
                    await self.broadcast({"event_type": "mouse_moved", "dx": pending_dx, "dy": pending_dy})
                    pending_dx = pending_dy = 0
                    last_flush = now
                await asyncio.sleep(0.001)
            except Exception as e:
                logger.error("error processing message queue: %s", e)
                await asyncio.sleep(0.01)
        logger.debug("message queue processor stopped")

    def on_key_press(self, rawcode: int) -> None:
        if rawcode and self.is_allowed(rawcode):
            self.queue_message({"event_type": "key_pressed", "rawcode": rawcode})

    def on_key_release(self, rawcode: int) -> None:
        if rawcode and self.is_allowed(rawcode):
            self.queue_message({"event_type": "key_released", "rawcode": rawcode})

    def on_mouse_click(self, button_code: int, pressed: bool) -> None:
        if button_code and self.is_allowed(button_code, is_mouse=True):
            event = "mouse_pressed" if pressed else "mouse_released"
            self.queue_message({"event_type": event, "button": button_code})

    def on_mouse_scroll(self, rotation: int) -> None:
        if rotation and self.is_allowed(rotation, is_scroll=True):
            self.queue_message({"event_type": "mouse_wheel", "rotation": rotation})

    def _on_raw_mouse_move(self, dx: int, dy: int) -> None:
        if not self.is_allowed(0, is_mouse_move=True):
            return
        self.queue_message({"event_type": "mouse_moved", "dx": dx, "dy": dy})

    def start_input_listeners(self) -> None:
        if sys.platform == "win32":
            self._input_listener = PynputInputListener(
                on_key_press=self.on_key_press,
                on_key_release=self.on_key_release,
                on_mouse_click=self.on_mouse_click,
                on_mouse_scroll=self.on_mouse_scroll,
            )
        else:
            if not EVDEV_AVAILABLE:
                logger.error("evdev not available - input listeners not started")
                return
            self._input_listener = EvdevInputListener(
                on_key_press=self.on_key_press,
                on_key_release=self.on_key_release,
                on_mouse_click=self.on_mouse_click,
                on_mouse_scroll=self.on_mouse_scroll,
            )
        self._input_listener.start()

    def stop_input_listeners(self) -> None:
        listener = getattr(self, "_input_listener", None)
        if listener:
            listener.stop()
            self._input_listener = None

    def start_raw_mouse(self) -> None:
        if not RAW_MOUSE_AVAILABLE:
            logger.warning("rawinput not available on this OS")
            return
        if self._raw_mouse_thread and self._raw_mouse_thread.is_alive():
            logger.debug("rawinput thread already running")
            return
        if sys.platform == "win32":
            self._raw_mouse_thread = RawMouseThread(
                callback=self._on_raw_mouse_move,
                min_delta=self.raw_mouse_min_delta,
            )
        else:
            if not self.linux_raw_mouse_device:
                logger.debug("raw mouse (linux): no device selected, not starting")
                return
            self._raw_mouse_thread = RawMouseThread(
                callback=self._on_raw_mouse_move,
                device_path=self.linux_raw_mouse_device,
                min_delta=self.raw_mouse_min_delta,
            )
        self._raw_mouse_thread.start()
        logger.info("rawinput thread started")

    def stop_raw_mouse(self) -> None:
        if self._raw_mouse_thread:
            self._raw_mouse_thread.stop()
            self._raw_mouse_thread = None
        logger.info("rawinput thread stopped")

    def start_http_server(self) -> None:
        if not self.http_enabled:
            return
        if self._http_server:
            logger.debug("HTTP server already running")
            return
        web_root = get_web_root()
        self._http_server = LocalHTTPServer(self.host, self.http_port, web_root)
        self._http_server.start()

    def stop_http_server(self) -> None:
        if self._http_server:
            self._http_server.stop()
            self._http_server = None

    def start_analog_support(self) -> None:
        if not ANALOG_AVAILABLE:
            if self.analog_enabled:
                logger.warning("analog support is Windows-only - skipping on this platform")
            return
        if not self.analog_enabled:
            return
        if self._analog and self._analog.is_running:
            logger.debug("analog handler already running")
            return
        self._analog = AnalogHandler(
            queue_message=self.queue_message,
            is_allowed=self.is_allowed,
        )
        self._analog.start(self.analog_device or "")

    def stop_analog_support(self) -> None:
        if self._analog:
            self._analog.stop()
            self._analog = None

    def get_analog_devices(self) -> list:
        if not ANALOG_AVAILABLE:
            return []
        return enum_analog_devices()

    async def handle_client(self, websocket) -> None:
        self.clients.add(websocket)
        try:
            remote      = websocket.remote_address
            client_ip   = remote[0] if remote else "unknown"
            client_port = remote[1] if remote else "unknown"
        except Exception:
            client_ip, client_port = "unknown", "unknown"
        try:
            origin     = websocket.request.headers.get("Origin", "N/A")
            user_agent = websocket.request.headers.get("User-Agent", "N/A")
        except Exception:
            origin = user_agent = "N/A"

        logger.info("new connection from %s:%s - origin: %s", client_ip, client_port, origin)
        try:
            async for message in websocket:
                try:
                    data     = json.loads(message)
                    msg_type = data.get("type")
                    logger.debug("message from %s:%s - type=%r", client_ip, client_port, msg_type)
                    if msg_type == "auth":
                        await self._handle_auth(websocket, data, client_ip, client_port, origin, user_agent)
                    else:
                        logger.debug("unhandled message type %r from %s:%s", msg_type, client_ip, client_port)
                except json.JSONDecodeError as e:
                    logger.warning("invalid json from %s:%s: %s", client_ip, client_port, e)
        except websockets.exceptions.ConnectionClosedOK as e:
            logger.info("connection closed cleanly: %s:%s code=%s reason=%r", client_ip, client_port, e.code, e.reason)
        except websockets.exceptions.ConnectionClosedError as e:
            logger.warning("connection closed with error: %s:%s code=%s reason=%r", client_ip, client_port, e.code, e.reason)
        except websockets.exceptions.ConnectionClosed as e:
            logger.info("connection closed: %s:%s code=%s reason=%r", client_ip, client_port, e.code, e.reason)
        except Exception:
            logger.exception("unexpected error handling %s:%s", client_ip, client_port)
        finally:
            was_authed = websocket in self.authenticated_clients
            self.clients.discard(websocket)
            self.authenticated_clients.discard(websocket)
            if was_authed:
                remaining = len(self.authenticated_clients)
                logger.info("authenticated client disconnected: %s:%s (remaining: %d)", client_ip, client_port, remaining)
                self._write_clients_file()
                self.show_toast_notification(
                    "client disconnected",
                    f"ip: {client_ip}:{client_port}\nremaining connections: {remaining}",
                )
            else:
                logger.debug("unauthenticated client removed: %s:%s", client_ip, client_port)

    async def _handle_auth(self, websocket, data: dict,
                            client_ip: str, client_port, origin: str, user_agent: str) -> None:
        token = data.get("token", "")
        if not token:
            logger.warning("auth rejected from %s:%s - no token provided", client_ip, client_port)
            await websocket.send(json.dumps({"type": "auth_response", "status": "failed"}))
            await websocket.close()
        elif not self.auth_token or token == self.auth_token:
            self.authenticated_clients.add(websocket)
            await websocket.send(json.dumps({"type": "auth_response", "status": "success"}))
            count = len(self.authenticated_clients)
            logger.info("client authenticated from %s:%s (total authed: %d)", client_ip, client_port, count)
            self._write_clients_file()
            self.show_toast_notification(
                "a new client connected",
                f"origin: {origin}\nactive connections: {count}",
            )
        else:
            logger.warning("auth rejected from %s:%s - token mismatch", client_ip, client_port)
            await websocket.send(json.dumps({"type": "auth_response", "status": "failed"}))
            self.show_toast_notification(
                "authentication failed",
                f"ip: {client_ip}:{client_port}\norigin: {origin}\nreason: bad token",
            )
            await websocket.close()

    async def _run_ws_server(self) -> None:
        while self.running:
            self._rebind_event = asyncio.Event()
            attempt_host = self.host
            attempt_port = self.port
            try:
                ws_server = await websockets.serve(self.handle_client, self.host, self.port)
                logger.info("server started on ws://%s:%d", self.host, self.port)
                self._prev_host = self.host
                self._prev_port = self.port
            except OSError as e:
                kind = "inuse" if e.errno in (10048, 98) else "denied" if e.errno == 13 else None
                if kind is None:
                    raise
                prev_host = getattr(self, "_prev_host", None)
                prev_port = getattr(self, "_prev_port", None)
                if prev_host and prev_port:
                    logger.error("port %d %s - reverting to %s:%d", attempt_port, kind, prev_host, prev_port)
                    self.host = prev_host
                    self.port = prev_port
                    self._revert_config(prev_host, prev_port)
                    self._spawn_rebind_failed(attempt_host, attempt_port, kind, prev_host, prev_port)
                    continue
                else:
                    logger.error("port %d %s on initial bind - waiting for port change...", attempt_port, kind)
                    self._spawn_port_error(kind)
                    while self.running and not self._rebind_event.is_set():
                        try:
                            await asyncio.wait_for(self._stop_event.wait(), timeout=1.0)
                        except asyncio.TimeoutError:
                            pass
                        self.reload_config_if_changed()
                    continue

            try:
                while self.running and not self._rebind_event.is_set():
                    try:
                        await asyncio.wait_for(self._stop_event.wait(), timeout=1.0)
                    except asyncio.TimeoutError:
                        pass
                    self.reload_config_if_changed()
            finally:
                ws_server.close()
                await ws_server.wait_closed()
                if self._rebind_event.is_set():
                    logger.info("rebound to ws://%s:%d", self.host, self.port)

    async def start(self) -> None:
        self.loop = asyncio.get_event_loop()
        self.running = True
        self._stop_event   = asyncio.Event()
        self._rebind_event = asyncio.Event()
        self.start_http_server()
        self.queue_processor_task = asyncio.create_task(self.process_message_queue())
        self.start_input_listeners()
        self.start_analog_support()
        if sys.platform == "win32":
            if self.raw_mouse_enabled:
                self.start_raw_mouse()
        else:
            if self.linux_raw_mouse_device:
                self.start_raw_mouse()

        if self.auth_token:
            logger.info("auth token: %s****", self.auth_token[:4] if len(self.auth_token) >= 4 else self.auth_token)
        else:
            logger.warning("auth disabled")
        logger.info("analog support: %s", "enabled" if self.analog_enabled else "disabled")
        if self.analog_enabled and not self.analog_device:
            logger.warning("no analog device selected")

        try:
            await self._run_ws_server()
        finally:
            if self.queue_processor_task:
                self.queue_processor_task.cancel()
                try:
                    await self.queue_processor_task
                except asyncio.CancelledError:
                    pass
            self.stop_input_listeners()
            self.stop_analog_support()
            self.stop_raw_mouse()
            self.stop_http_server()

    def stop(self) -> None:
        self.running = False
        if self.loop and hasattr(self, "_stop_event"):
            self.loop.call_soon_threadsafe(self._stop_event.set)

if __name__ == "__main__":
    from services.tray import main
    main(server_class=InputOverlayServer)