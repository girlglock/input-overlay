import asyncio
import json
import logging
import secrets
import threading
import sys
from typing import Set
from pathlib import Path

import websockets
from pynput import keyboard, mouse
import pystray
from PIL import Image, ImageDraw

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

KEY_CODE_MAP = {
    keyboard.Key.esc: 27,
    keyboard.KeyCode.from_char('1'): 49,
    keyboard.KeyCode.from_char('2'): 50,
    keyboard.KeyCode.from_char('3'): 51,
    keyboard.KeyCode.from_char('4'): 52,
    keyboard.KeyCode.from_char('5'): 53,
    keyboard.KeyCode.from_char('6'): 54,
    keyboard.KeyCode.from_char('7'): 55,
    keyboard.KeyCode.from_char('8'): 56,
    keyboard.KeyCode.from_char('9'): 57,
    keyboard.KeyCode.from_char('0'): 48,
    keyboard.KeyCode.from_char('-'): 189,
    keyboard.KeyCode.from_char('='): 187,
    keyboard.Key.backspace: 8,
    keyboard.Key.f1: 112,
    keyboard.Key.f2: 113,
    keyboard.Key.f3: 114,
    keyboard.Key.f4: 115,
    keyboard.Key.f5: 116,
    keyboard.Key.f6: 117,
    keyboard.Key.f7: 118,
    keyboard.Key.f8: 119,
    keyboard.Key.f9: 120,
    keyboard.Key.f10: 121,
    keyboard.Key.f11: 122,
    keyboard.Key.f12: 123,
    keyboard.Key.print_screen: 44,
    keyboard.Key.scroll_lock: 145,
    keyboard.Key.pause: 19,
    keyboard.Key.insert: 45,
    keyboard.Key.delete: 46,
    keyboard.Key.home: 36,
    keyboard.Key.end: 35,
    keyboard.Key.page_up: 33,
    keyboard.Key.page_down: 34,
    keyboard.Key.tab: 9,
    keyboard.KeyCode.from_char('q'): 81,
    keyboard.KeyCode.from_char('w'): 87,
    keyboard.KeyCode.from_char('e'): 69,
    keyboard.KeyCode.from_char('r'): 82,
    keyboard.KeyCode.from_char('t'): 84,
    keyboard.KeyCode.from_char('y'): 89,
    keyboard.KeyCode.from_char('u'): 85,
    keyboard.KeyCode.from_char('i'): 73,
    keyboard.KeyCode.from_char('o'): 79,
    keyboard.KeyCode.from_char('p'): 80,
    keyboard.KeyCode.from_char('['): 219,
    keyboard.KeyCode.from_char(']'): 221,
    keyboard.KeyCode.from_char('\\'): 220,
    keyboard.Key.caps_lock: 20,
    keyboard.KeyCode.from_char('a'): 65,
    keyboard.KeyCode.from_char('s'): 83,
    keyboard.KeyCode.from_char('d'): 68,
    keyboard.KeyCode.from_char('f'): 70,
    keyboard.KeyCode.from_char('g'): 71,
    keyboard.KeyCode.from_char('h'): 72,
    keyboard.KeyCode.from_char('j'): 74,
    keyboard.KeyCode.from_char('k'): 75,
    keyboard.KeyCode.from_char('l'): 76,
    keyboard.KeyCode.from_char(';'): 186,
    keyboard.KeyCode.from_char("'"): 222,
    keyboard.Key.enter: 13,
    keyboard.Key.shift: 160,
    keyboard.Key.shift_l: 160,
    keyboard.Key.shift_r: 161,
    keyboard.KeyCode.from_char('z'): 90,
    keyboard.KeyCode.from_char('x'): 88,
    keyboard.KeyCode.from_char('c'): 67,
    keyboard.KeyCode.from_char('v'): 86,
    keyboard.KeyCode.from_char('b'): 66,
    keyboard.KeyCode.from_char('n'): 78,
    keyboard.KeyCode.from_char('m'): 77,
    keyboard.KeyCode.from_char(','): 188,
    keyboard.KeyCode.from_char('.'): 190,
    keyboard.KeyCode.from_char('/'): 191,
    keyboard.Key.ctrl: 162,
    keyboard.Key.ctrl_l: 162,
    keyboard.Key.ctrl_r: 163,
    keyboard.Key.cmd: 91,
    keyboard.Key.cmd_l: 91,
    keyboard.Key.cmd_r: 92,
    keyboard.Key.alt: 164,
    keyboard.Key.alt_l: 164,
    keyboard.Key.alt_r: 165,
    keyboard.Key.space: 32,
    keyboard.Key.menu: 93,
    keyboard.Key.left: 37,
    keyboard.Key.up: 38,
    keyboard.Key.right: 39,
    keyboard.Key.down: 40,
    keyboard.Key.num_lock: 144,
}

MOUSE_BUTTON_MAP = {
    mouse.Button.left: 1,
    mouse.Button.right: 2,
    mouse.Button.middle: 3,
    'x1': 4,
    'x2': 5,
}

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

    def load_config(self, config_path: str = "config.json") -> dict:
        try:
            config_file = Path(config_path)
            if config_file.exists():
                with open(config_file, 'r') as f:
                    config = json.load(f)
                    logger.info(f"loaded config from {config_path}")
                    return config
            else:
                random_token = secrets.token_urlsafe(32)
                default_config = {
                    "host": "localhost",
                    "port": 16899,
                    "auth_token": random_token
                }
                with open(config_file, 'w') as f:
                    json.dump(default_config, f, indent=4)
                logger.info(f"generated auth token: {random_token}")
                logger.info(f"add to overlay url: &wsauth={random_token}")
                return default_config
        except Exception as e:
            logger.error(f"error loading config: {e}")
            return {}

    def get_rawcode(self, key) -> int:
        if hasattr(key, 'vk') and key.vk:
            return key.vk
        if key in KEY_CODE_MAP:
            return KEY_CODE_MAP[key]
        if hasattr(key, 'char') and key.char:
            key_char = keyboard.KeyCode.from_char(key.char.lower())
            if key_char in KEY_CODE_MAP:
                return KEY_CODE_MAP[key_char]
        return 0

    async def broadcast(self, message: dict):
        if not self.authenticated_clients:
            return
        message_json = json.dumps(message)
        disconnected = set()
        for client in self.authenticated_clients:
            try:
                await client.send(message_json)
            except websockets.exceptions.ConnectionClosed:
                disconnected.add(client)
        for client in disconnected:
            self.authenticated_clients.discard(client)
            self.clients.discard(client)

    def on_key_press(self, key):
        rawcode = self.get_rawcode(key)
        if rawcode and self.loop:
            message = {"event_type": "key_pressed", "rawcode": rawcode}
            asyncio.run_coroutine_threadsafe(self.broadcast(message), self.loop)

    def on_key_release(self, key):
        rawcode = self.get_rawcode(key)
        if rawcode and self.loop:
            message = {"event_type": "key_released", "rawcode": rawcode}
            asyncio.run_coroutine_threadsafe(self.broadcast(message), self.loop)

    def on_mouse_click(self, x, y, button, pressed):
        button_code = MOUSE_BUTTON_MAP.get(button, 0)
        if button_code and self.loop:
            message = {
                "event_type": "mouse_pressed" if pressed else "mouse_released",
                "button": button_code
            }
            asyncio.run_coroutine_threadsafe(self.broadcast(message), self.loop)

    def on_mouse_scroll(self, x, y, dx, dy):
        rotation = 1 if dy > 0 else -1 if dy < 0 else 0
        if rotation != 0 and self.loop:
            message = {"event_type": "mouse_wheel", "rotation": rotation}
            asyncio.run_coroutine_threadsafe(self.broadcast(message), self.loop)

    async def handle_client(self, websocket):
        self.clients.add(websocket)
        try:
            async for message in websocket:
                try:
                    data = json.loads(message)
                    if data.get('type') == 'auth':
                        token = data.get('token', '')
                        if not self.auth_token or token == self.auth_token:
                            self.authenticated_clients.add(websocket)
                            await websocket.send(json.dumps({'type': 'auth_response', 'status': 'success'}))
                            logger.info(f"client authenticated")
                        else:
                            await websocket.send(json.dumps({'type': 'auth_response', 'status': 'failed'}))
                            logger.warning(f"auth failed")
                            await websocket.close()
                except json.JSONDecodeError:
                    pass
        except websockets.exceptions.ConnectionClosed:
            pass
        finally:
            self.clients.discard(websocket)
            self.authenticated_clients.discard(websocket)

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
        self.start_input_listeners()
        async with websockets.serve(self.handle_client, self.host, self.port):
            logger.info(f"server started on ws://{self.host}:{self.port}")
            if self.auth_token:
                logger.info(f"auth token: {self.auth_token}")
            else:
                logger.warning("auth disabled")
            while self.running:
                await asyncio.sleep(1)
            self.stop_input_listeners()

    def stop(self):
        self.running = False

def get_resource_path(relative_path):
    try:
        base_path = sys._MEIPASS
    except Exception:
        base_path = Path(__file__).parent
    return Path(base_path) / relative_path

def create_tray_icon():
    icon_path = get_resource_path("icon.ico")
    return Image.open(icon_path)

def run_server(server):
    try:
        asyncio.run(server.start())
    except Exception as e:
        logger.error(f"server error: {e}")

def main():
    server = InputOverlayServer()
    config = server.load_config()
    server.host = config.get('host', 'localhost')
    server.port = config.get('port', 16899)
    server.auth_token = config.get('auth_token', '')
    server_thread = threading.Thread(target=run_server, args=(server,), daemon=True)
    server_thread.start()

    def on_quit(icon, item):
        logger.info("shutting down...")
        server.stop()
        icon.stop()

    def on_copy_token(icon, item):
        try:
            import pyperclip
            if server.auth_token:
                pyperclip.copy(server.auth_token)
                logger.info("auth token copied")
            else:
                logger.warning("no auth token set")
        except ImportError:
            logger.error("pyperclip missing")

    try:
        icon = pystray.Icon(
            "input_overlay",
            create_tray_icon(),
            "Input Overlay Server",
            menu=pystray.Menu(
                pystray.MenuItem("copy auth token", on_copy_token),
                pystray.MenuItem("exit", on_quit)
            )
        )
        logger.info("starting tray icon...")
        icon.run()
    except Exception as e:
        logger.error(f"tray icon error: {e}")
        input("press enter to exit...")

if __name__ == "__main__":
    main()