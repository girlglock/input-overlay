import os
import sys
import json
import logging
import winreg
import threading
from pathlib import Path
from PyQt6.QtWidgets import (QApplication, QMainWindow, QWidget, QVBoxLayout, 
                             QHBoxLayout, QPushButton, QLabel, QScrollArea, QFrame,
                             QLineEdit, QCheckBox, QComboBox, QGroupBox, QDialog,
                             QDialogButtonBox, QTextEdit)
from PyQt6.QtCore import Qt, pyqtSignal, QObject, QThread
from PyQt6.QtGui import QIcon, QMovie, QDesktopServices
from PyQt6.QtCore import QUrl
from pynput import keyboard, mouse

try:
    from services.consts import (WS_SERVER_VERSION, MOUSE_BUTTON_MAP, RAW_CODE_TO_KEY_NAME, 
                                 MOUSE_BUTTON_NAMES, get_rawcode)
except ImportError:
    import sys
    sys.path.append(str(Path(__file__).parent.parent))
    from services.consts import (WS_SERVER_VERSION, MOUSE_BUTTON_MAP, RAW_CODE_TO_KEY_NAME, 
                                 MOUSE_BUTTON_NAMES, get_rawcode)

logger = logging.getLogger(__name__)

GITHUB_RELEASES_URL = "https://github.com/girlglock/input-overlay/releases"
GITHUB_API_URL = "https://api.github.com/repos/girlglock/input-overlay/releases/latest"

CS16 = """
QMainWindow {
    background-color: #4a5942;
}

QWidget {
    color: #dedfd6;
    font-family: "Arial", sans-serif;
}

QLabel {
    color: #d8ded3;
}

QPushButton {
    background-color: #4a5942;
    color: #ffffff;
    border: 1px solid;
    border-color: #8c9284 #292c21 #292c21 #8c9284;
    padding: 4px 4px;
    font-size: 13px;
}

QPushButton:hover {
    color: #c4b550;
}

QPushButton:pressed {
    border-color: #292c21 #8c9284 #8c9284 #292c21;
}

QLineEdit {
    background-color: #3e4637;
    color: #dedfd6;
    border: 1px solid;
    border-color: #292c21 #8c9284 #8c9284 #292c21;
    padding: 5px;
    font-size: 13px;
}

QLineEdit:focus {
    border-color: #c4b550;
}

QCheckBox {
    color: #dedfd6;
    spacing: 8px;
}

QCheckBox::indicator {
    width: 18px;
    height: 18px;
    border: 1px solid;
    border-color: #292c21 #8c9284 #8c9284 #292c21;
    background-color: #3e4637;
}

QCheckBox::indicator:checked {
    background-color: #c4b550;
    border-color: #8c9284 #292c21 #292c21 #8c9284;
}

QComboBox {
    background-color: #3e4637;
    color: #dedfd6;
    border: 1px solid;
    border-color: #292c21 #8c9284 #8c9284 #292c21;
    padding: 5px;
    font-size: 13px;
}

QComboBox:hover {
    border-color: #c4b550;
}

QComboBox::drop-down {
    border: none;
    width: 20px;
}

QComboBox::down-arrow {
    image: none;
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 6px solid #dedfd6;
    margin-right: 5px;
}

QComboBox QAbstractItemView {
    background-color: #3e4637;
    color: #dedfd6;
    selection-background-color: #4a5942;
    border: 1px solid #292c21;
}

QGroupBox {
    color: #c4b550;
    font-weight: bold;
    font-size: 14px;
    border: 1px solid #8c9284;
    margin-top: 10px;
    padding-top: 10px;
}

QGroupBox::title {
    subcontrol-origin: margin;
    subcontrol-position: top left;
    padding: 0 5px;
}

QScrollArea {
    background-color: #4a5942;
}

#ScrollContent {
    background-color: #3e4637;
    border: 1px solid;
    border-top: 1px solid #292c21;
    border-right: none;
    border-bottom: 1px solid #8c9284;
    border-left: 1px solid #292c21;
}

QFrame#ItemFrame {
    background-color: #3e4637;
    border: 1px solid;
    border-color: #292c21 #8c9284 #8c9284 #292c21;
    margin: 2px;
}

QScrollBar:vertical {
    border: 1px solid #292c21;
    background: #5a6a50;
    width: 18px;
    margin: 0px;
}

QScrollBar::handle:vertical {
    background: #4a5942;
    border: 1px solid;
    border-color: #8c9284 #292c21 #292c21 #8c9284;
    min-height: 20px;
}

QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
    height: 0px;
}

QDialog {
    background-color: #4a5942;
}
"""

def get_resource_path(relative_path):
                                               
    try:
        base_path = sys._MEIPASS
    except Exception:
        base_path = Path(__file__).parent.parent
    return Path(base_path) / relative_path

def get_exe_path() -> Path:
                                                                       
    if getattr(sys, 'frozen', False):
        return Path(sys.executable)
    return Path(__file__).parent.parent / "input-overlay-ws.py"

def get_startup_shortcut_path() -> Path:
                                                                        
    import winreg as _wr
    key = _wr.OpenKey(_wr.HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders")
    startup_dir = _wr.QueryValueEx(key, "Startup")[0]
    _wr.CloseKey(key)
    return Path(startup_dir) / "input-overlay-ws.lnk"

def is_autostart_enabled() -> bool:
    try:
        lnk = get_startup_shortcut_path()
        return lnk.exists()
    except Exception:
        return False

def set_autostart(enabled: bool):
                                                          
    try:
        lnk_path = get_startup_shortcut_path()
        if enabled:
            import win32com.client                                                  
            shell = win32com.client.Dispatch("WScript.Shell")
            shortcut = shell.CreateShortCut(str(lnk_path))
            shortcut.Targetpath = str(get_exe_path())
            shortcut.WorkingDirectory = str(get_exe_path().parent)
            shortcut.IconLocation = str(get_exe_path())
            shortcut.save()
            logger.info(f"autostart shortcut created: {lnk_path}")
        else:
            if lnk_path.exists():
                lnk_path.unlink()
                logger.info(f"autostart shortcut removed: {lnk_path}")
    except ImportError:
        if enabled:
            exe = str(get_exe_path()).replace("'", "''")
            lnk = str(get_startup_shortcut_path()).replace("'", "''")
            ps_cmd = (
                f"$s=(New-Object -COM WScript.Shell).CreateShortcut('{lnk}');"
                f"$s.TargetPath='{exe}';"
                f"$s.WorkingDirectory='{str(get_exe_path().parent)}';"
                f"$s.Save()"
            )
            import subprocess
            subprocess.run(
                ["powershell", "-NoProfile", "-NonInteractive", "-Command", ps_cmd],
                capture_output=True
            )
            logger.info(f"autostart shortcut created via PowerShell: {lnk}")
        else:
            lnk_path = get_startup_shortcut_path()
            if lnk_path.exists():
                lnk_path.unlink()
    except Exception as e:
        logger.error(f"set_autostart error: {e}")

class UpdateChecker(QObject):
    update_available = pyqtSignal(str, str)               # version, release_body
    check_done      = pyqtSignal()

    def check(self, dismissed: list):
                                                               
        def _run():
            try:
                import urllib.request
                req = urllib.request.Request(
                    GITHUB_API_URL,
                    headers={"User-Agent": "input-overlay-ws"}
                )
                with urllib.request.urlopen(req, timeout=5) as resp:
                    data = json.loads(resp.read().decode())
                latest = data.get("tag_name", "").lstrip("v")
                body   = data.get("body", "").strip()
                if latest and latest != WS_SERVER_VERSION and latest not in dismissed:
                    self.update_available.emit(latest, body)
            except Exception as e:
                logger.debug(f"update check failed: {e}")
            finally:
                self.check_done.emit()
        threading.Thread(target=_run, daemon=True).start()

class UpdateDialog(QDialog):
    def __init__(self, latest_version: str, release_body: str = "", parent=None):
        super().__init__(parent)
        self.latest_version = latest_version
        self.release_body   = release_body
        self.dismissed = False
        self.setWindowTitle("UPDATE AVAILABLE")
        self.setFixedWidth(480)
        icon_path = get_resource_path("assets/icon.ico")
        if icon_path.exists():
            self.setWindowIcon(QIcon(str(icon_path)))
        self.setStyleSheet(CS16)
        self._build_ui()

    def _build_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(20, 20, 20, 20)
        layout.setSpacing(12)

        content_row = QHBoxLayout()
        content_row.setSpacing(12)

        text_col = QVBoxLayout()
        text_col.setSpacing(8)

        title = QLabel("A new version of Input Overlay is available!")
        title.setStyleSheet("color: #c4b550; font-weight: bold; font-size: 14px;")
        title.setWordWrap(True)
        text_col.addWidget(title)

        body = QLabel(
            f"Current version: <b>{WS_SERVER_VERSION}</b><br>"
            f"Latest version:  <b>{self.latest_version}</b>"
        )
        body.setStyleSheet("color: #dedfd6; font-size: 13px;")
        body.setTextFormat(Qt.TextFormat.RichText)
        text_col.addWidget(body)
        text_col.addStretch()

        content_row.addLayout(text_col)

        gif_label = QLabel()
        gif_path = get_resource_path("assets/update.gif")
        if gif_path.exists():
            self.movie = QMovie(str(gif_path))
            from PyQt6.QtCore import QSize
            self.movie.setScaledSize(QSize(80, 80))
            gif_label.setMovie(self.movie)
            self.movie.start()
        gif_label.setAlignment(Qt.AlignmentFlag.AlignBottom | Qt.AlignmentFlag.AlignRight)
        content_row.addWidget(gif_label)

        layout.addLayout(content_row)

        if self.release_body:
            notes_label = QLabel("PATCH NOTES")
            notes_label.setStyleSheet("color: #c4b550; font-weight: bold; font-size: 12px;")
            layout.addWidget(notes_label)

            notes_box = QTextEdit()
            notes_box.setReadOnly(True)
            notes_box.setPlainText(self.release_body)
            notes_box.setFixedHeight(160)
            layout.addWidget(notes_box)

        btn_row = QHBoxLayout()
        download_btn = QPushButton("DOWNLOAD")
        download_btn.setMinimumHeight(32)
        download_btn.clicked.connect(self._on_download)
        btn_row.addWidget(download_btn)

        dismiss_btn = QPushButton("DISMISS THIS VERSION")
        dismiss_btn.setMinimumHeight(32)
        dismiss_btn.clicked.connect(self._on_dismiss)
        btn_row.addWidget(dismiss_btn)

        layout.addLayout(btn_row)

    def _on_download(self):
        QDesktopServices.openUrl(QUrl(GITHUB_RELEASES_URL))
        self.accept()

    def _on_dismiss(self):
        self.dismissed = True
        self.accept()

class InputSignals(QObject):
    key_detected   = pyqtSignal(str)
    stop_listening = pyqtSignal()

class SettingsEditor(QMainWindow):
    def __init__(self, config_path):
        super().__init__()
        self.config_path = config_path
        icon_path = get_resource_path("assets/icon.ico")
        if icon_path.exists():
            self.setWindowIcon(QIcon(str(icon_path)))
        self.temp_whitelist = []
        self.is_listening = False
        self.kb_listener = None
        self.ms_listener = None
        self.signals = InputSignals()
        self._latest_version = None

        self.signals.key_detected.connect(self.on_key_detected)
        self.signals.stop_listening.connect(self.stop_listening)

        self.load_config()
        self.setup_ui()
        self.setStyleSheet(CS16)

        self._update_checker = UpdateChecker()
        self._update_checker.update_available.connect(self._on_update_available)
        self._update_checker.check(self.dismissed_versions)

    def load_config(self):
        try:
            with open(self.config_path, 'r') as f:
                config = json.load(f)
                self.temp_whitelist      = list(config.get('key_whitelist', []))
                self.auth_token          = config.get('auth_token', '')
                self.analog_enabled      = config.get('analog_enabled', False)
                self.analog_device       = config.get('analog_device', None)
                self.balloon_enabled     = config.get('balloon_notifications', True)
                self.autostart_enabled   = is_autostart_enabled()
                self.dismissed_versions  = config.get('dismissed_versions', [])
        except Exception:
            self.temp_whitelist     = []
            self.auth_token         = ''
            self.analog_enabled     = False
            self.analog_device      = None
            self.balloon_enabled    = True
            self.autostart_enabled  = is_autostart_enabled()
            self.dismissed_versions = []

    def save_config(self):
        try:
            with open(self.config_path, 'r') as f:
                config = json.load(f)

            config['key_whitelist']        = self.temp_whitelist
            config['auth_token']           = self.auth_input.text()
            config['analog_enabled']       = self.analog_checkbox.isChecked()
            config['balloon_notifications'] = self.balloon_checkbox.isChecked()
            config['dismissed_versions']   = self.dismissed_versions
            if self.device_combo.currentData():
                config['analog_device'] = self.device_combo.currentData()

            with open(self.config_path, 'w') as f:
                json.dump(config, f, indent=4)

            set_autostart(self.autostart_checkbox.isChecked())

        except Exception as e:
            logger.error(f"error saving config: {e}")

    def get_analog_devices(self):
        try:
            import hid
            devices = []

            analog_keyboards = [
                (0x31E3, None,   "Wooting",                    0xFF54),
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

            all_devices = hid.enumerate()
            seen_vidpid = set()

            for device_dict in all_devices:
                vid        = device_dict['vendor_id']
                pid        = device_dict['product_id']
                usage_page = device_dict.get('usage_page', 0)
                interface  = device_dict.get('interface_number', -1)

                for known_vid, known_pid, name, required_usage in analog_keyboards:
                    if vid == known_vid and (known_pid is None or pid == known_pid):
                        if required_usage is not None and usage_page != required_usage:
                            continue
                        if required_usage is None:
                            vidpid_key = (vid, pid)
                            if vidpid_key in seen_vidpid:
                                break
                            seen_vidpid.add(vidpid_key)

                        device_str   = f"{vid:04x}:{pid:04x}:{interface}" if interface >= 0 else f"{vid:04x}:{pid:04x}"
                        product_name = device_dict.get('product_string', name)
                        devices.append({'id': device_str, 'name': f"{product_name} ({device_str})"})
                        break

            return devices
        except ImportError:
            logger.error("hidapi not installed")
            return []
        except Exception as e:
            logger.error(f"error enumerating devices: {e}")
            return []

    def setup_ui(self):
        self.setWindowTitle("SETTINGS")
        self.setFixedSize(1000, 660)

        central_widget = QWidget()
        self.setCentralWidget(central_widget)
        main_layout = QVBoxLayout(central_widget)
        main_layout.setContentsMargins(15, 15, 15, 15)

        columns_layout = QHBoxLayout()

        left_column = QVBoxLayout()
        left_column.setSpacing(10)

        auth_group = QGroupBox("AUTHENTICATION")
        auth_layout = QVBoxLayout()

        auth_label = QLabel("Auth Token:")
        auth_label.setStyleSheet("color: #a0aa95; font-weight: normal;")
        auth_layout.addWidget(auth_label)

        self.auth_input = QLineEdit()
        self.auth_input.setText(self.auth_token)
        self.auth_input.setEchoMode(QLineEdit.EchoMode.Password)
        auth_layout.addWidget(self.auth_input)

        show_token_btn = QPushButton("SHOW/HIDE TOKEN")
        show_token_btn.clicked.connect(self.toggle_token_visibility)
        auth_layout.addWidget(show_token_btn)

        copy_token_btn = QPushButton("COPY TOKEN")
        copy_token_btn.clicked.connect(self.copy_token)
        auth_layout.addWidget(copy_token_btn)

        auth_group.setLayout(auth_layout)
        left_column.addWidget(auth_group)

        analog_group = QGroupBox("ANALOG SUPPORT")
        analog_layout = QVBoxLayout()

        self.analog_checkbox = QCheckBox("Enable Analog Mode")
        self.analog_checkbox.setChecked(self.analog_enabled)
        self.analog_checkbox.stateChanged.connect(self.on_analog_toggled)
        analog_layout.addWidget(self.analog_checkbox)

        device_label = QLabel("Analog Device:")
        device_label.setStyleSheet("color: #a0aa95; font-weight: normal; margin-top: 10px;")
        analog_layout.addWidget(device_label)

        self.device_combo = QComboBox()
        self.device_combo.addItem("No device selected", None)

        devices = self.get_analog_devices()
        for device in devices:
            self.device_combo.addItem(device['name'], device['id'])

        if self.analog_device:
            index = self.device_combo.findData(self.analog_device)
            if index >= 0:
                self.device_combo.setCurrentIndex(index)

        self.device_combo.setEnabled(self.analog_enabled)
        analog_layout.addWidget(self.device_combo)

        refresh_btn = QPushButton("REFRESH DEVICES")
        refresh_btn.clicked.connect(self.refresh_devices)
        analog_layout.addWidget(refresh_btn)

        analog_layout.addStretch()
        analog_group.setLayout(analog_layout)
        left_column.addWidget(analog_group)

        app_group = QGroupBox("APPLICATION")
        app_layout = QVBoxLayout()
        app_layout.setSpacing(8)

        self.balloon_checkbox = QCheckBox("Enable balloon notifications")
        self.balloon_checkbox.setChecked(self.balloon_enabled)
        app_layout.addWidget(self.balloon_checkbox)

        self.autostart_checkbox = QCheckBox("Start with Windows")
        self.autostart_checkbox.setChecked(self.autostart_enabled)
        app_layout.addWidget(self.autostart_checkbox)

        app_group.setLayout(app_layout)
        left_column.addWidget(app_group)

        info_group = QGroupBox("ABOUT")
        about_h_layout = QHBoxLayout()

        links_container = QVBoxLayout()
        self.version_label = QLabel(f"Input-Overlay WebSocket Server | Version: {WS_SERVER_VERSION}")
        self.version_label.setStyleSheet("color: #a0aa95; font-weight: normal;")
        self.version_label.setOpenExternalLinks(True)
        links_container.addWidget(self.version_label)

        links = [
            ("GitHub",        "https://github.com/girlglock/input-overlay"),
            ("Twitter",       "https://twitter.com/girlglock"),
            ("girlglock.com", "https://girlglock.com"),
            (" ", " "),
            (" ", " "),
        ]

        for text, url in links:
            link_label = QLabel(f'<a href="{url}" style="color: #c4b550;">{text}</a>')
            link_label.setOpenExternalLinks(True)
            link_label.setStyleSheet("margin-top: 4px;")
            links_container.addWidget(link_label)

        about_h_layout.addLayout(links_container)
        about_h_layout.addStretch()

        image_label = QLabel()
        img_path = get_resource_path("assets/steamhappy.gif")

        if img_path.exists():
            self.movie = QMovie(str(img_path))
            from PyQt6.QtCore import QSize
            self.movie.setScaledSize(QSize(128, 128))
            image_label.setMovie(self.movie)
            self.movie.start()
            image_label.setContentsMargins(0, 0, 10, 0)
            image_label.setAlignment(Qt.AlignmentFlag.AlignBottom | Qt.AlignmentFlag.AlignRight)

        about_h_layout.addWidget(image_label)
        info_group.setLayout(about_h_layout)
        left_column.addWidget(info_group)

        left_column.addStretch()

        right_column = QVBoxLayout()
        right_column.setSpacing(10)

        whitelist_group = QGroupBox("KEY WHITELIST")
        whitelist_layout = QVBoxLayout()

        instruction_label = QLabel(
            "Click ADD KEY then press the key you want to add.\n"
            "Empty list means all keys are allowed."
        )
        instruction_label.setStyleSheet("color: #a0aa95; font-weight: normal;")
        whitelist_layout.addWidget(instruction_label)

        self.add_btn = QPushButton("ADD KEY")
        self.add_btn.setMinimumHeight(40)
        self.add_btn.clicked.connect(self.toggle_listen)
        whitelist_layout.addWidget(self.add_btn)

        self.scroll_area = QScrollArea()
        self.scroll_area.setWidgetResizable(True)

        self.scroll_content = QWidget()
        self.scroll_content.setObjectName("ScrollContent")
        self.scroll_layout = QVBoxLayout(self.scroll_content)
        self.scroll_layout.setAlignment(Qt.AlignmentFlag.AlignTop)

        self.scroll_area.setWidget(self.scroll_content)
        whitelist_layout.addWidget(self.scroll_area)

        whitelist_group.setLayout(whitelist_layout)
        right_column.addWidget(whitelist_group)

        columns_layout.addLayout(left_column, 1)
        columns_layout.addLayout(right_column, 1)
        main_layout.addLayout(columns_layout)

        footer_layout = QHBoxLayout()
        self.save_btn = QPushButton("SAVE")
        self.save_btn.clicked.connect(self.save_and_close)

        self.cancel_btn = QPushButton("CANCEL")
        self.cancel_btn.clicked.connect(self.cancel)

        footer_layout.addWidget(self.save_btn)
        footer_layout.addWidget(self.cancel_btn)
        main_layout.addLayout(footer_layout)

        self.refresh_list()

    def _on_update_available(self, latest_version: str, release_body: str):
        self._latest_version = latest_version
        self.version_label.setText(
            f'Input-Overlay WebSocket Server | Version: {WS_SERVER_VERSION} '
            f'(<a href="{GITHUB_RELEASES_URL}" style="color: #c4b550;">New version: {latest_version} available</a>)'
        )

    def toggle_token_visibility(self):
        if self.auth_input.echoMode() == QLineEdit.EchoMode.Password:
            self.auth_input.setEchoMode(QLineEdit.EchoMode.Normal)
        else:
            self.auth_input.setEchoMode(QLineEdit.EchoMode.Password)

    def copy_token(self):
        try:
            import pyperclip
            token = self.auth_input.text()
            if token:
                pyperclip.copy(token)
                logger.info("auth token copied to clipboard")
        except ImportError:
            logger.error("pyperclip not installed")
        except Exception as e:
            logger.error(f"failed to copy token: {e}")

    def on_analog_toggled(self, state):
        self.device_combo.setEnabled(state == Qt.CheckState.Checked.value)

    def refresh_devices(self):
        current_device = self.device_combo.currentData()
        self.device_combo.clear()
        self.device_combo.addItem("No device selected", None)

        devices = self.get_analog_devices()
        for device in devices:
            self.device_combo.addItem(device['name'], device['id'])

        if current_device:
            index = self.device_combo.findData(current_device)
            if index >= 0:
                self.device_combo.setCurrentIndex(index)

    def toggle_listen(self):
        if not self.is_listening:
            self.start_listening()
        else:
            self.stop_listening()

    def start_listening(self):
        self.is_listening = True
        self.add_btn.setText("LISTENING... [ESC TO CANCEL]")

        def on_press(key):
            if key == keyboard.Key.esc:
                self.signals.stop_listening.emit()
                return False
            rawcode = get_rawcode(key)
            name = RAW_CODE_TO_KEY_NAME.get(rawcode)
            if name:
                self.signals.key_detected.emit(name)
                self.signals.stop_listening.emit()
            return False

        def on_click(x, y, button, pressed):
            if pressed:
                btn_code = MOUSE_BUTTON_MAP.get(button)
                name = MOUSE_BUTTON_NAMES.get(btn_code)
                if name:
                    self.signals.key_detected.emit(name)
                    self.signals.stop_listening.emit()
                return False

        def on_scroll(x, y, dx, dy):
            self.signals.key_detected.emit("mouse_wheel")
            self.signals.stop_listening.emit()
            return False

        self.kb_listener = keyboard.Listener(on_press=on_press, suppress=False)
        self.ms_listener = mouse.Listener(on_click=on_click, on_scroll=on_scroll, suppress=False)
        self.kb_listener.start()
        self.ms_listener.start()

    def stop_listening(self):
        self.is_listening = False
        self.add_btn.setText("ADD KEY")
        if self.kb_listener: self.kb_listener.stop()
        if self.ms_listener: self.ms_listener.stop()

    def on_key_detected(self, name):
        if name not in self.temp_whitelist:
            self.temp_whitelist.append(name)
            self.refresh_list()

    def refresh_list(self):
        while self.scroll_layout.count():
            child = self.scroll_layout.takeAt(0)
            if child.widget():
                child.widget().deleteLater()

        for key in self.temp_whitelist:
            row_widget = QWidget()
            row_layout = QHBoxLayout(row_widget)
            row_layout.setContentsMargins(0, 0, 0, 0)
            row_layout.setSpacing(5)

            item_frame = QFrame()
            item_frame.setObjectName("ItemFrame")
            item_frame.setFixedHeight(35)

            frame_layout = QHBoxLayout(item_frame)
            frame_layout.setContentsMargins(5, 2, 5, 2)
            frame_layout.setSpacing(5)

            label = QLabel(key.upper())
            label.setStyleSheet("color: #dedfd6; font-weight: bold;")
            frame_layout.addWidget(label)

            remove_btn = QPushButton("X")
            remove_btn.setFixedWidth(31)
            remove_btn.setFixedHeight(31)
            remove_btn.clicked.connect(lambda checked, k=key: self.remove_key(k))

            row_layout.addWidget(item_frame)
            row_layout.addWidget(remove_btn)

            self.scroll_layout.addWidget(row_widget)

    def remove_key(self, key):
        if key in self.temp_whitelist:
            self.temp_whitelist.remove(key)
            self.refresh_list()

    def save_and_close(self):
        self.save_config()
        self.close()

    def cancel(self):
        self.close()

    def closeEvent(self, event):
        self.stop_listening()
        event.accept()

def check_for_updates_on_startup(config_path: str = "config.json", child_processes: list = None):
    try:
        with open(config_path, 'r') as f:
            config = json.load(f)
        dismissed = config.get('dismissed_versions', [])
    except Exception:
        dismissed = []

    def _run():
        import urllib.request
        try:
            req = urllib.request.Request(
                GITHUB_API_URL,
                headers={"User-Agent": "input-overlay-ws"}
            )
            with urllib.request.urlopen(req, timeout=5) as resp:
                data = json.loads(resp.read().decode())
            latest = data.get("tag_name", "").lstrip("v")
            body   = data.get("body", "").strip()
            if latest and latest != WS_SERVER_VERSION and latest not in dismissed:
                import subprocess
                if getattr(sys, 'frozen', False):
                    proc = subprocess.Popen([sys.executable, "--update-popup", latest, config_path, body])
                else:
                    script_path = Path(__file__).resolve()
                    proc = subprocess.Popen([sys.executable, str(script_path), "--update-popup", latest, config_path, body])
                if child_processes is not None:
                    child_processes.append(proc)
        except Exception as e:
            logger.debug(f"startup update check failed: {e}")

    threading.Thread(target=_run, daemon=True).start()


def _run_update_popup_process(latest_version: str, config_path: str, release_body: str = ""):
    app = QApplication(sys.argv)
    dlg = UpdateDialog(latest_version, release_body)
    dlg.setStyleSheet(CS16)
    dlg.exec()
    if dlg.dismissed:
        try:
            with open(config_path, 'r') as f:
                cfg = json.load(f)
            dismissed = cfg.get('dismissed_versions', [])
            if latest_version not in dismissed:
                dismissed.append(latest_version)
            cfg['dismissed_versions'] = dismissed
            with open(config_path, 'w') as f:
                json.dump(cfg, f, indent=4)
        except Exception as e:
            logger.error(f"could not save dismissed version: {e}")

def run_settings_editor(config_path="config.json"):
    app = QApplication(sys.argv)
    editor = SettingsEditor(config_path)
    editor.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "--update-popup":
        latest = sys.argv[2] if len(sys.argv) >= 3 else ""
        config_path = sys.argv[3] if len(sys.argv) >= 4 else "config.json"
        release_body = sys.argv[4] if len(sys.argv) >= 5 else ""
        _run_update_popup_process(latest, config_path, release_body)
    else:
        run_settings_editor()