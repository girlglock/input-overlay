from __future__ import annotations

import json
import logging
import os
import sys
from pathlib import Path

from PyQt6.QtCore import QObject, QSize, QTimer, Qt, pyqtSignal
from PyQt6.QtGui import QIcon, QMovie
from PyQt6.QtWidgets import (
    QApplication,
    QCheckBox,
    QComboBox,
    QFrame,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMainWindow,
    QPushButton,
    QScrollArea,
    QVBoxLayout,
    QWidget,
)
import sys

from services.analog import enum_analog_devices
from services.consts import (
    MOUSE_BUTTON_NAMES,
    RAW_CODE_TO_KEY_NAME,
    WS_SERVER_VERSION,
)

if sys.platform == "win32":
    from services.pynput_input import PynputInputListener
    _PYNPUT_AVAILABLE = True
else:
    _PYNPUT_AVAILABLE = False
    try:
        from services.evdev_input import EvdevInputListener as _EvdevInputListener
        _EVDEV_AVAILABLE = True
    except ImportError:
        _EVDEV_AVAILABLE = False
from services.dialogs import (
    CS16,
    GITHUB_RELEASES_URL,
    InstantTooltipCheckBox,
    TitleBar,
    UpdateChecker,
    _load_pixel_font,
    _run_port_error_process,
    _run_update_popup_process,
)
from services.utils import get_resource_path, is_autostart_enabled, set_autostart

logger = logging.getLogger(__name__)

class InputSignals(QObject):
    key_detected   = pyqtSignal(str)
    stop_listening = pyqtSignal()

class SettingsEditor(QMainWindow):
    def __init__(self, config_path: str) -> None:
        super().__init__()
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint)
        self.config_path = config_path

        icon_path = get_resource_path("assets/icon.ico")
        if icon_path.exists():
            self.setWindowIcon(QIcon(str(icon_path)))

        self.temp_whitelist: list  = []
        self.is_listening: bool    = False
        self._capture_listener     = None
        self.signals               = InputSignals()
        self._latest_version: str  = ""

        self.signals.key_detected.connect(self.on_key_detected)
        self.signals.stop_listening.connect(self.stop_listening)

        self.load_config()
        self.setup_ui()
        self.setStyleSheet(CS16)

        self._update_checker = UpdateChecker()
        self._update_checker.update_available.connect(self._on_update_available)
        self._update_checker.check(self.dismissed_versions)

    def load_config(self) -> None:
        try:
            with open(self.config_path, "r") as f:
                config = json.load(f)
            self.temp_whitelist          = list(config.get("key_whitelist", []))
            self.host                    = config.get("host", "0.0.0.0")
            self.port                    = config.get("port", 8080)
            self._original_host          = self.host
            self._original_port          = self.port
            self.http_enabled            = config.get("http_enabled", False)
            self.http_port               = config.get("http_port", 4456)
            self.auth_token              = config.get("auth_token", "")
            self.analog_enabled          = config.get("analog_enabled", False)
            self.analog_device           = config.get("analog_device", None)
            self.balloon_enabled         = config.get("balloon_notifications", True)
            self.raw_mouse_enabled       = config.get("raw_mouse_enabled", False)
            self.linux_raw_mouse_device  = config.get("linux_raw_mouse_device", "")
            self.autostart_enabled       = is_autostart_enabled()
            self.dismissed_versions      = config.get("dismissed_versions", [])
        except Exception:
            self.temp_whitelist          = []
            self.host                    = "0.0.0.0"
            self.port                    = 8080
            self._original_host          = self.host
            self._original_port          = self.port
            self.http_enabled            = False
            self.http_port               = 4456
            self.auth_token              = ""
            self.analog_enabled          = False
            self.analog_device           = None
            self.balloon_enabled         = True
            self.raw_mouse_enabled       = False
            self.linux_raw_mouse_device  = ""
            self.autostart_enabled       = is_autostart_enabled()
            self.dismissed_versions      = []

    def save_config(self) -> None:
        try:
            with open(self.config_path, "r") as f:
                config = json.load(f)

            new_host       = self.host_input.text()
            new_port       = int(self.port_input.text() or 8080)
            server_changed = new_host != self._original_host or new_port != self._original_port

            config["key_whitelist"]         = self.temp_whitelist
            config["host"]                  = new_host
            config["port"]                  = new_port
            config["http_enabled"]          = self.http_checkbox.isChecked()
            config["http_port"]             = int(self.http_port_input.text() or 4456)
            config["auth_token"]            = self.auth_input.text()
            config["analog_enabled"]        = self.analog_checkbox.isChecked()
            config["balloon_notifications"] = self.balloon_checkbox.isChecked()
            config["dismissed_versions"]    = self.dismissed_versions
            if sys.platform == "win32":
                config["raw_mouse_enabled"] = self.raw_mouse_checkbox.isChecked()
            else:
                config["linux_raw_mouse_device"] = self.linux_mouse_combo.currentData() or ""
            if self.device_combo.currentData():
                config["analog_device"] = self.device_combo.currentData()

            with open(self.config_path, "w") as f:
                json.dump(config, f, indent=4)

            if server_changed:
                restart_flag = Path(self.config_path).parent / "restart.flag"
                restart_flag.touch()
                logger.info("server address changed to %s:%d, restart flag written", new_host, new_port)

            set_autostart(self.autostart_checkbox.isChecked())

        except Exception:
            logger.exception("error saving config")

    def get_analog_devices(self) -> list:
        return [{"id": d["id"], "name": d["name"]} for d in enum_analog_devices()]

    def setup_ui(self) -> None:
        self.setWindowTitle("SETTINGS")
        self.setFixedSize(1000, 960)

        central = QWidget()
        self.setCentralWidget(central)
        main_layout = QVBoxLayout(central)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)

        self._title_bar = TitleBar("SETTINGS", self, minimizable=True)
        main_layout.addWidget(self._title_bar)

        content = QWidget()
        content_layout = QVBoxLayout(content)
        content_layout.setContentsMargins(15, 15, 15, 15)
        main_layout.addWidget(content)

        columns_layout = QHBoxLayout()
        content_layout.addLayout(columns_layout)

        #left side
        left_column = QVBoxLayout()
        left_column.setSpacing(10)

        server_group  = QGroupBox("SERVER")
        server_layout = QVBoxLayout()

        ws_lbl = QLabel("WebSocket Server:")
        ws_lbl.setStyleSheet("color: #a0aa95; font-weight: normal;")
        server_layout.addWidget(ws_lbl)

        server_grid   = QHBoxLayout()
        server_grid.setSpacing(10)

        host_col = QVBoxLayout()
        host_lbl = QLabel("Host:")
        host_lbl.setStyleSheet("color: #a0aa95; font-weight: normal;")
        host_col.addWidget(host_lbl)
        self.host_input = QLineEdit(self.host)
        host_col.addWidget(self.host_input)
        server_grid.addLayout(host_col)

        port_col = QVBoxLayout()
        port_lbl = QLabel("Port:")
        port_lbl.setStyleSheet("color: #a0aa95; font-weight: normal;")
        port_col.addWidget(port_lbl)
        self.port_input = QLineEdit(str(self.port))
        port_col.addWidget(self.port_input)
        server_grid.addLayout(port_col)

        server_layout.addLayout(server_grid)

        self.http_lbl = QLabel("Local HTTP Server:")
        self.http_lbl.setStyleSheet("color: #a0aa95; font-weight: normal; margin-top: 8px;")
        server_layout.addWidget(self.http_lbl)

        http_grid = QHBoxLayout()
        http_grid.setSpacing(10)

        http_host_col = QVBoxLayout()
        self.http_host_lbl = QLabel("Host:")
        self.http_host_lbl.setStyleSheet("color: #a0aa95; font-weight: normal;")
        http_host_col.addWidget(self.http_host_lbl)
        self.http_host_display = QLineEdit(self.host)
        self.http_host_display.setReadOnly(True)
        self.http_host_display.setToolTip("Uses the same host as the WebSocket server")
        http_host_col.addWidget(self.http_host_display)
        http_grid.addLayout(http_host_col)

        http_port_col = QVBoxLayout()
        self.http_port_lbl = QLabel("Port:")
        self.http_port_lbl.setStyleSheet("color: #a0aa95; font-weight: normal;")
        http_port_col.addWidget(self.http_port_lbl)
        self.http_port_input = QLineEdit(str(self.http_port))
        http_port_col.addWidget(self.http_port_input)
        http_grid.addLayout(http_port_col)

        server_layout.addLayout(http_grid)

        self.host_input.textChanged.connect(self.http_host_display.setText)
        self._apply_http_enabled_state(self.http_enabled)

        server_group.setLayout(server_layout)
        left_column.addWidget(server_group)

        auth_group  = QGroupBox("AUTHENTICATION")
        auth_layout = QVBoxLayout()
        auth_lbl    = QLabel("Auth Token:")
        auth_lbl.setStyleSheet("color: #a0aa95; font-weight: normal;")
        auth_layout.addWidget(auth_lbl)
        self.auth_input = QLineEdit(self.auth_token)
        self.auth_input.setEchoMode(QLineEdit.EchoMode.Password)
        auth_layout.addWidget(self.auth_input)

        token_btns = QHBoxLayout()
        token_btns.setSpacing(4)
        for label, slot in [
            ("SHOW/HIDE TOKEN",   self.toggle_token_visibility),
            ("COPY TOKEN",        self.copy_token),
            ("REGENERATE TOKEN",  self.regenerate_token),
        ]:
            btn = QPushButton(label)
            btn.clicked.connect(slot)
            token_btns.addWidget(btn)
        auth_layout.addLayout(token_btns)
        auth_group.setLayout(auth_layout)
        left_column.addWidget(auth_group)

        analog_group  = QGroupBox("ANALOG SUPPORT")
        analog_layout = QVBoxLayout()

        self.analog_checkbox = QCheckBox("Enable Analog Mode")
        self.analog_checkbox.setChecked(self.analog_enabled)
        self.analog_checkbox.stateChanged.connect(self.on_analog_toggled)
        analog_layout.addWidget(self.analog_checkbox)

        device_lbl = QLabel("Analog Device:")
        device_lbl.setStyleSheet("color: #a0aa95; font-weight: normal; margin-top: 10px;")
        analog_layout.addWidget(device_lbl)

        self.device_combo = QComboBox()
        self.device_combo.addItem("No device selected", None)
        for dev in self.get_analog_devices():
            self.device_combo.addItem(dev["name"], dev["id"])
        if self.analog_device:
            idx = self.device_combo.findData(self.analog_device)
            if idx >= 0:
                self.device_combo.setCurrentIndex(idx)
        self.device_combo.setEnabled(self.analog_enabled)

        device_row = QHBoxLayout()
        device_row.setSpacing(4)
        refresh_btn = QPushButton("REFRESH DEVICES")
        refresh_btn.clicked.connect(self.refresh_devices)
        device_row.addWidget(self.device_combo, 3)
        device_row.addWidget(refresh_btn, 2)
        analog_layout.addLayout(device_row)

        analog_layout.addStretch()
        analog_group.setLayout(analog_layout)
        left_column.addWidget(analog_group)

        app_group  = QGroupBox("APPLICATION")
        app_layout = QVBoxLayout()

        self.balloon_checkbox = QCheckBox("Enable balloon notifications")
        self.balloon_checkbox.setChecked(self.balloon_enabled)
        app_layout.addWidget(self.balloon_checkbox)

        autostart_label = "Start with Windows" if sys.platform == "win32" else "Start on login"
        self.autostart_checkbox = QCheckBox(autostart_label)
        self.autostart_checkbox.setChecked(self.autostart_enabled)
        app_layout.addWidget(self.autostart_checkbox)

        self.http_checkbox = QCheckBox("Enable local HTTP server\n(serve overlay without internet)")
        self.http_checkbox.setToolTip("Hosts the overlay configurator locally to enable usage without a network connection")
        self.http_checkbox.setChecked(self.http_enabled)
        self.http_checkbox.stateChanged.connect(self._on_http_toggled)
        app_layout.addWidget(self.http_checkbox)

        if sys.platform == "win32":
            self.raw_mouse_checkbox = InstantTooltipCheckBox("Enable RawInputBuffer reads from the Windows API\n(mouse movement for mouse_pad element)")
            self.raw_mouse_checkbox.setToolTip("Sometimes requires the ws server to run with admin privileges depending on foreground window privileges")
            self.raw_mouse_checkbox.setChecked(self.raw_mouse_enabled)
            app_layout.addWidget(self.raw_mouse_checkbox)
        else:
            raw_mouse_lbl = QLabel("Raw Mouse Device\n(mouse movement for mouse_pad element):")
            raw_mouse_lbl.setStyleSheet("color: #a0aa95; font-weight: normal; margin-top: 4px;")
            app_layout.addWidget(raw_mouse_lbl)

            mouse_row = QHBoxLayout()
            mouse_row.setSpacing(4)

            self.linux_mouse_combo = QComboBox()
            self.linux_mouse_combo.setToolTip(
                "Select a raw evdev mouse device for the mouse_pad element\n"
                "Linux has no RawInputBuffer API... this reads directly from /dev/input\n"
                "Pick the specific hardware device to avoid double counting.\n"
            )
            self._populate_linux_mouse_combo()

            refresh_mouse_btn = QPushButton("REFRESH")
            refresh_mouse_btn.setFixedWidth(80)
            refresh_mouse_btn.clicked.connect(self._refresh_linux_mouse_devices)

            mouse_row.addWidget(self.linux_mouse_combo, 1)
            mouse_row.addWidget(refresh_mouse_btn)
            app_layout.addLayout(mouse_row)

        app_group.setLayout(app_layout)
        left_column.addWidget(app_group)

        info_group = QGroupBox("ABOUT")
        about_h_layout = QHBoxLayout()

        links_container = QVBoxLayout()
        self.version_label = QLabel(f"Input-Overlay WebSocket Server | Version: {WS_SERVER_VERSION}<br>(latest)")
        self.version_label.setStyleSheet("color: #a0aa95; font-weight: normal;")
        self.version_label.setOpenExternalLinks(True)
        self.version_label.setMinimumHeight(32)
        links_container.addSpacing(-8)
        links_container.addWidget(self.version_label)

        for text, url in [
            ("GitHub",        "https://github.com/girlglock/input-overlay"),
            ("Twitter",       "https://twitter.com/girlglock_"),
            ("girlglock.com", "https://girlglock.com"),
            (" ", " "),
            (" ", " "),
        ]:
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
            self.movie.setScaledSize(QSize(128, 128))
            image_label.setMovie(self.movie)
            self.movie.start()
            image_label.setContentsMargins(0, 0, 10, 0)
            image_label.setAlignment(Qt.AlignmentFlag.AlignBottom | Qt.AlignmentFlag.AlignRight)
        about_h_layout.addWidget(image_label)

        info_group.setLayout(about_h_layout)
        info_group.setFixedHeight(206)
        left_column.addWidget(info_group)

        left_column.addStretch()

        #right side
        right_column = QVBoxLayout()
        right_column.setSpacing(10)

        whitelist_group  = QGroupBox("KEY WHITELIST")
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

        clients_group  = QGroupBox("CONNECTED CLIENTS")
        clients_layout = QVBoxLayout()

        self.clients_scroll = QScrollArea()
        self.clients_scroll.setWidgetResizable(True)
        self.clients_scroll.setFixedHeight(60)
        self.clients_content = QWidget()
        self.clients_content.setObjectName("ScrollContent")
        self.clients_inner = QVBoxLayout(self.clients_content)
        self.clients_inner.setAlignment(Qt.AlignmentFlag.AlignTop)
        self.clients_scroll.setWidget(self.clients_content)
        clients_layout.addWidget(self.clients_scroll)

        clients_group.setLayout(clients_layout)
        clients_group.setFixedHeight(100)
        right_column.addWidget(clients_group)

        self._clients_timer = QTimer(self)
        self._clients_timer.timeout.connect(self.refresh_clients)
        self._clients_timer.start(1000)
        self.refresh_clients()

        columns_layout.addLayout(left_column, 1)
        columns_layout.addLayout(right_column, 1)

        footer_layout = QHBoxLayout()
        self.save_btn   = QPushButton("SAVE")
        self.cancel_btn = QPushButton("CANCEL")
        self.save_btn.clicked.connect(self.save_and_close)
        self.cancel_btn.clicked.connect(self.cancel)
        footer_layout.addWidget(self.save_btn)
        footer_layout.addWidget(self.cancel_btn)
        content_layout.addLayout(footer_layout)

        self.refresh_list()

    def _on_update_available(self, latest_version: str, release_body: str) -> None:
        self._latest_version = latest_version
        self.version_label.setText(
            f'Input-Overlay WebSocket Server | Version: {WS_SERVER_VERSION}<br>'
            f'(<a href="{GITHUB_RELEASES_URL}" style="color: #c4b550;">'
            f'New version: {latest_version} available</a>)'
        )

    def toggle_token_visibility(self) -> None:
        if self.auth_input.echoMode() == QLineEdit.EchoMode.Password:
            self.auth_input.setEchoMode(QLineEdit.EchoMode.Normal)
        else:
            self.auth_input.setEchoMode(QLineEdit.EchoMode.Password)

    def copy_token(self) -> None:
        token = self.auth_input.text()
        if not token:
            return
        #do pyperclip then try qt
        copied = False
        try:
            import pyperclip
            pyperclip.copy(token)
            copied = True
        except ImportError:
            logger.debug("pyperclip exploded")
        except Exception:
            logger.debug("pyperclip exploded, using qt: %s",
                         type(Exception).__name__)
        if not copied:
            try:
                QApplication.clipboard().setText(token)
                copied = True
            except Exception:
                logger.exception("failed to copy token via qt clipboard")
        if not copied:
            logger.error("no clipboard mechanism available")

    def regenerate_token(self) -> None:
        import secrets
        self.auth_input.setText(secrets.token_urlsafe(32))

    def _on_http_toggled(self, state: int) -> None:
        self._apply_http_enabled_state(state == Qt.CheckState.Checked.value)

    def _apply_http_enabled_state(self, enabled: bool) -> None:
        label_style = "color: #a0aa95; font-weight: normal;"
        self.http_lbl.setStyleSheet(label_style + " margin-top: 8px;")
        self.http_host_lbl.setStyleSheet(label_style)
        self.http_port_lbl.setStyleSheet(label_style)
        if enabled:
            self.http_host_display.setStyleSheet("")
            self.http_port_input.setStyleSheet("")
        else:
            greyed = "background-color: #3a3a3a; color: #606060;"
            self.http_host_display.setStyleSheet(greyed)
            self.http_port_input.setStyleSheet(greyed)
        self.http_host_display.setEnabled(enabled)
        self.http_port_input.setEnabled(enabled)

    def on_analog_toggled(self, state: int) -> None:
        self.device_combo.setEnabled(state == Qt.CheckState.Checked.value)

    def refresh_devices(self) -> None:
        current = self.device_combo.currentData()
        self.device_combo.clear()
        self.device_combo.addItem("No device selected", None)
        for dev in self.get_analog_devices():
            self.device_combo.addItem(dev["name"], dev["id"])
        if current:
            idx = self.device_combo.findData(current)
            if idx >= 0:
                self.device_combo.setCurrentIndex(idx)

    def _populate_linux_mouse_combo(self) -> None:
        if sys.platform == "win32":
            return
        try:
            from services.rawinput_linux import enum_raw_mouse_devices
            mouse_devs = enum_raw_mouse_devices()
        except Exception:
            mouse_devs = []

        self.linux_mouse_combo.clear()
        self.linux_mouse_combo.addItem("Disabled", "")
        for dev in mouse_devs:
            label = f"{dev['name']}  [{dev['path']}]"
            self.linux_mouse_combo.addItem(label, dev["path"])

        if self.linux_raw_mouse_device:
            idx = self.linux_mouse_combo.findData(self.linux_raw_mouse_device)
            if idx >= 0:
                self.linux_mouse_combo.setCurrentIndex(idx)

    def _refresh_linux_mouse_devices(self) -> None:
        if sys.platform == "win32":
            return
        current_path = self.linux_mouse_combo.currentData()
        self._populate_linux_mouse_combo()
        if current_path:
            idx = self.linux_mouse_combo.findData(current_path)
            if idx >= 0:
                self.linux_mouse_combo.setCurrentIndex(idx)

    def toggle_listen(self) -> None:
        if self.is_listening:
            self.stop_listening()
        else:
            self.start_listening()

    def start_listening(self) -> None:
        self.is_listening = True
        self.add_btn.setText("LISTENING... [ESC TO CANCEL]")

        if _PYNPUT_AVAILABLE:
            self._start_listening_pynput()
        elif _EVDEV_AVAILABLE:
            self._start_listening_evdev()
        else:
            logger.warning("no input backend there for getting keys")
            self.stop_listening()

    def _start_listening_pynput(self) -> None:
        _ESC_VK = 27

        def on_key_press(rawcode: int):
            if rawcode == _ESC_VK:
                self.signals.stop_listening.emit()
                return
            name = RAW_CODE_TO_KEY_NAME.get(rawcode)
            if name:
                self.signals.key_detected.emit(name)
                self.signals.stop_listening.emit()

        def on_key_release(rawcode: int):
            pass

        def on_mouse_click(btn_code: int, pressed: bool):
            if pressed:
                name = MOUSE_BUTTON_NAMES.get(btn_code)
                if name:
                    self.signals.key_detected.emit(name)
                    self.signals.stop_listening.emit()

        def on_mouse_scroll(rotation: int):
            self.signals.key_detected.emit("mouse_wheel")
            self.signals.stop_listening.emit()

        self._capture_listener = PynputInputListener(
            on_key_press=on_key_press,
            on_key_release=on_key_release,
            on_mouse_click=on_mouse_click,
            on_mouse_scroll=on_mouse_scroll,
        )
        self._capture_listener.start()

    def _start_listening_evdev(self) -> None:
        _ESC_VK = 27

        def on_key_press(vk: int):
            if vk == _ESC_VK:
                self.signals.stop_listening.emit()
                return
            name = RAW_CODE_TO_KEY_NAME.get(vk)
            if name:
                self.signals.key_detected.emit(name)
                self.signals.stop_listening.emit()

        def on_key_release(vk: int):
            pass

        def on_mouse_click(btn_code: int, pressed: bool):
            if pressed:
                name = MOUSE_BUTTON_NAMES.get(btn_code)
                if name:
                    self.signals.key_detected.emit(name)
                    self.signals.stop_listening.emit()

        def on_mouse_scroll(rotation: int):
            self.signals.key_detected.emit("mouse_wheel")
            self.signals.stop_listening.emit()

        self._capture_listener = _EvdevInputListener(
            on_key_press=on_key_press,
            on_key_release=on_key_release,
            on_mouse_click=on_mouse_click,
            on_mouse_scroll=on_mouse_scroll,
        )
        self._capture_listener.start()

    def stop_listening(self) -> None:
        self.is_listening = False
        self.add_btn.setText("ADD KEY")
        listener = getattr(self, "_capture_listener", None)
        if listener:
            listener.stop()
            self._capture_listener = None

    def on_key_detected(self, name: str) -> None:
        if name not in self.temp_whitelist:
            self.temp_whitelist.append(name)
            self.refresh_list()

    def refresh_list(self) -> None:
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

    def remove_key(self, key: str) -> None:
        if key in self.temp_whitelist:
            self.temp_whitelist.remove(key)
            self.refresh_list()

    def refresh_clients(self) -> None:
        clients_file = Path(self.config_path).parent / "clients.json"
        clients: list = []
        try:
            if clients_file.exists():
                with open(clients_file, "r") as f:
                    clients = json.load(f)
        except Exception:
            pass

        while self.clients_inner.count():
            child = self.clients_inner.takeAt(0)
            if child.widget():
                child.widget().deleteLater()

        if not clients:
            lbl = QLabel("no clients connected")
            lbl.setStyleSheet("color: #a0aa95; font-weight: normal; font-size: 14px;")
            self.clients_inner.addWidget(lbl)
        else:
            for c in clients:
                lbl = QLabel(f"{c.get('ip', 'unknown')}:{c.get('port', '?')}")
                lbl.setStyleSheet("color: #dedfd6; font-size: 14px;")
                self.clients_inner.addWidget(lbl)

    def save_and_close(self) -> None:
        self.save_config()
        self.close()

    def cancel(self) -> None:
        self.close()

    def closeEvent(self, event) -> None:
        self.stop_listening()
        self._clients_timer.stop()
        event.accept()

def run_settings_editor(config_path: str = "config.json") -> None:
    app = QApplication(sys.argv)
    _load_pixel_font()
    editor = SettingsEditor(config_path)
    editor.show()
    sys.exit(app.exec())

if __name__ == "__main__":
    from services.dialogs import (
        _run_port_error_process,
        _run_rebind_failed_process,
        _run_update_popup_process,
    )

    args = sys.argv[1:]

    if args and args[0] == "--update-popup":
        latest       = args[1] if len(args) >= 2 else ""
        config_path  = args[2] if len(args) >= 3 else "config.json"
        release_body = os.environ.get("IOV_UPDATE_BODY", "")
        _run_update_popup_process(latest, config_path, release_body)

    elif args and args[0] == "--port-error":
        error_kind  = args[1] if len(args) >= 2 else "inuse"
        host        = args[2] if len(args) >= 3 else "localhost"
        port        = int(args[3]) if len(args) >= 4 else 4455
        config_path = args[4] if len(args) >= 5 else "config.json"
        _run_port_error_process(error_kind, host, port, config_path)

    elif args and args[0] == "--rebind-failed":
        kind        = args[1] if len(args) >= 2 else "inuse"
        failed_host = args[2] if len(args) >= 3 else "localhost"
        failed_port = int(args[3]) if len(args) >= 4 else 4455
        prev_host   = args[4] if len(args) >= 5 else "localhost"
        prev_port   = int(args[5]) if len(args) >= 6 else 4455
        _run_rebind_failed_process(kind, failed_host, failed_port, prev_host, prev_port)

    else:
        config_path = args[0] if args else "config.json"
        run_settings_editor(config_path)
