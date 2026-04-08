from __future__ import annotations

import asyncio
import logging
import os
import socket
import sys
import threading

from services.logger import flush_log, setup_crash_handler, setup_signal_handlers
from services.utils import get_resource_path, spawn_subprocess

logger = logging.getLogger(__name__)

try:
    import pystray
    from PIL import Image, ImageDraw
    _PYSTRAY_AVAILABLE = True
except ImportError:
    _PYSTRAY_AVAILABLE = False


def _run_server(server, shutdown_callback=None) -> None:
    try:
        asyncio.run(server.start())
    except OSError as e:
        if e.errno in (10048, 98):  # WSAEADDRINUSE, EADDRINUSE
            logger.error("port %d already in use", server.port)
            proc = spawn_subprocess("--port-error", "inuse", server.host, str(server.port), server.config_path)
            _track_child(server, proc)
        elif e.errno == 13:  # EACCES
            logger.error("port %d access denied", server.port)
            proc = spawn_subprocess("--port-error", "denied", server.host, str(server.port), server.config_path)
            _track_child(server, proc)
        elif isinstance(e, socket.gaierror):
            logger.error("invalid host %r: %s", server.host, e)
            proc = spawn_subprocess("--port-error", "badhost", server.host, str(server.port), server.config_path)
            _track_child(server, proc)
        else:
            logger.exception("server OSError")
            proc = spawn_subprocess("--port-error", "oserror", server.host, str(server.port), server.config_path)
            _track_child(server, proc)
    except Exception:
        logger.exception("server error")

    if shutdown_callback:
        logger.info("server stopped, triggering shutdown")
        shutdown_callback()


def _track_child(server, proc) -> None:
    if proc and hasattr(server, "child_processes"):
        server.child_processes.append(proc)


def run_settings_editor_subprocess(config_path: str = "config.json"):
    return spawn_subprocess("--settings", config_path)


def _create_tray_icon() -> "Image.Image":
    from PIL import Image as _Image
    icon_path = get_resource_path("assets/icon.ico")
    if icon_path.exists():
        return _Image.open(icon_path)
    img = _Image.new("RGB", (64, 64), color="blue")
    from PIL import ImageDraw as _ID
    _ID.Draw(img).rectangle([16, 16, 48, 48], fill="white")
    return img

def _run_pystray_tray(server, child_processes, settings_proc) -> None:
    icon_ref: list = [None]

    def shutdown():
        flush_log()
        if icon_ref[0]:
            icon_ref[0].stop()

    server_thread = threading.Thread(
        target=_run_server,
        args=(server,),
        kwargs={"shutdown_callback": shutdown},
        daemon=True,
    )
    server_thread.start()

    from services.dialogs import check_for_updates_on_startup
    check_for_updates_on_startup("config.json", child_processes)

    def on_quit(icon, item):
        logger.info("shutting down...")
        server.stop()
        for proc in list(child_processes):
            try:
                if proc.poll() is None:
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

    menu = pystray.Menu(
        pystray.MenuItem("Settings", on_settings),
        pystray.MenuItem("Exit", on_quit),
    )

    try:
        icon = pystray.Icon("input_overlay", _create_tray_icon(), "Input Overlay Server", menu=menu)
        icon_ref[0] = icon
        logger.info("starting tray icon (windows/pystray)")
        icon.run()
    except Exception:
        logger.exception("tray icon error")
        input("press enter to exit...")

def _run_qt_tray(server, child_processes, settings_proc) -> None:
    from PyQt6.QtCore import Qt, QTimer
    from PyQt6.QtGui import QIcon, QPixmap, QImage
    from PyQt6.QtWidgets import (
        QApplication, QHBoxLayout, QLabel, QMenu, QPushButton,
        QSystemTrayIcon, QVBoxLayout, QWidget,
    )

    app = QApplication.instance() or QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(False)

    from services.dialogs import CS16, _load_pixel_font
    try:
        _load_pixel_font()
    except Exception:
        pass

    icon_path = get_resource_path("assets/icon.ico")
    if icon_path.exists():
        q_icon = QIcon(str(icon_path))
    else:
        pil_img = _create_tray_icon().convert("RGBA")
        data = pil_img.tobytes("raw", "RGBA")
        qimg = QImage(data, pil_img.width, pil_img.height, QImage.Format.Format_RGBA8888)
        q_icon = QIcon(QPixmap.fromImage(qimg))

    if not QSystemTrayIcon.isSystemTrayAvailable():
        logger.warning("qt tray: system tray not available, falling back to control window")
        _run_linux_control_window(server, child_processes, settings_proc)
        return

    tray = QSystemTrayIcon(q_icon)
    tray.setToolTip("Input Overlay Server")

    win = QWidget()
    win.setWindowTitle("Input Overlay")
    win.setFixedSize(300, 110)
    win.setWindowFlags(
        Qt.WindowType.Window |
        Qt.WindowType.WindowMinimizeButtonHint |
        Qt.WindowType.WindowCloseButtonHint,
    )
    if icon_path.exists():
        win.setWindowIcon(QIcon(str(icon_path)))
    win.setStyleSheet(CS16 + "\nQWidget { background-color: #4a5942; }")

    layout = QVBoxLayout(win)
    layout.setContentsMargins(12, 12, 12, 12)
    layout.setSpacing(8)

    status_lbl = QLabel(f"Server running on {server.host}:{server.port}")
    status_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
    status_lbl.setStyleSheet("color: #a0aa95; font-size: 12px;")
    layout.addWidget(status_lbl)

    btn_row = QHBoxLayout()
    btn_row.setSpacing(8)
    settings_btn = QPushButton("SETTINGS")
    settings_btn.setMinimumHeight(36)
    exit_btn = QPushButton("EXIT")
    exit_btn.setMinimumHeight(36)
    btn_row.addWidget(settings_btn)
    btn_row.addWidget(exit_btn)
    layout.addLayout(btn_row)

    _settings_proc_ref = [None]

    def open_control_window():
        win.show()
        win.raise_()
        win.activateWindow()

    def on_settings():
        if _settings_proc_ref[0] is not None and _settings_proc_ref[0].poll() is None:
            return
        proc = run_settings_editor_subprocess("config.json")
        if proc:
            _settings_proc_ref[0] = proc
            child_processes.append(proc)

    def do_quit():
        logger.info("shutting down...")
        server.stop()
        for proc in list(child_processes):
            try:
                if proc.poll() is None:
                    proc.terminate()
            except Exception:
                pass
        flush_log()
        tray.hide()
        app.quit()

    settings_btn.clicked.connect(on_settings)
    exit_btn.clicked.connect(do_quit)

    win.closeEvent = lambda e: (e.ignore(), win.hide())

    tray_menu = QMenu()
    tray_menu.setStyleSheet(CS16)
    act_open     = tray_menu.addAction("Open")
    act_settings = tray_menu.addAction("Settings")
    tray_menu.addSeparator()
    act_exit     = tray_menu.addAction("Exit")

    act_open.triggered.connect(open_control_window)
    act_settings.triggered.connect(on_settings)
    act_exit.triggered.connect(do_quit)

    tray.setContextMenu(tray_menu)

    def _tray_activated(reason):
        if reason == QSystemTrayIcon.ActivationReason.Trigger:
            if win.isVisible():
                win.hide()
            else:
                open_control_window()

    tray.activated.connect(_tray_activated)
    tray.show()

    def _refresh_status():
        status_lbl.setText(f"Server running on {server.host}:{server.port}")

    poll_timer = QTimer()
    poll_timer.setInterval(2000)
    poll_timer.timeout.connect(_refresh_status)
    poll_timer.start()

    server_thread = threading.Thread(
        target=_run_server,
        args=(server,),
        kwargs={"shutdown_callback": lambda: QTimer.singleShot(0, app.quit)},
        daemon=True,
    )
    server_thread.start()

    from services.dialogs import check_for_updates_on_startup
    check_for_updates_on_startup("config.json", child_processes)

    logger.info("qt tray icon started")
    app.exec()

def _run_linux_control_window(server, child_processes, settings_proc) -> None:
    from PyQt6.QtCore import Qt, QTimer
    from PyQt6.QtGui import QIcon
    from PyQt6.QtWidgets import QApplication, QHBoxLayout, QLabel, QPushButton, QVBoxLayout, QWidget
    app = QApplication.instance() or QApplication(sys.argv)
    from services.dialogs import CS16, _load_pixel_font
    try:
        _load_pixel_font()
    except Exception:
        pass

    win = QWidget()
    win.setWindowTitle("Input Overlay")
    win.setFixedSize(300, 110)
    win.setWindowFlags(
        Qt.WindowType.Window |
        Qt.WindowType.WindowMinimizeButtonHint |
        Qt.WindowType.WindowCloseButtonHint,
    )

    icon_path = get_resource_path("assets/icon.ico")
    if icon_path.exists():
        win.setWindowIcon(QIcon(str(icon_path)))

    win.setStyleSheet(CS16 + "\nQWidget { background-color: #4a5942; }")

    layout = QVBoxLayout(win)
    layout.setContentsMargins(12, 12, 12, 12)
    layout.setSpacing(8)

    status_lbl = QLabel(f"Server running on {server.host}:{server.port}")
    status_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
    status_lbl.setStyleSheet("color: #a0aa95; font-size: 12px;")
    layout.addWidget(status_lbl)

    btn_row = QHBoxLayout()
    btn_row.setSpacing(8)

    settings_btn = QPushButton("SETTINGS")
    settings_btn.setMinimumHeight(36)
    exit_btn = QPushButton("EXIT")
    exit_btn.setMinimumHeight(36)

    btn_row.addWidget(settings_btn)
    btn_row.addWidget(exit_btn)
    layout.addLayout(btn_row)
    _settings_proc_ref = [None]

    def on_settings():
        if _settings_proc_ref[0] is not None and _settings_proc_ref[0].poll() is None:
            return
        proc = run_settings_editor_subprocess("config.json")
        if proc:
            _settings_proc_ref[0] = proc
            child_processes.append(proc)

    def on_exit():
        logger.info("shutting down...")
        server.stop()
        for proc in list(child_processes):
            try:
                if proc.poll() is None:
                    proc.terminate()
            except Exception:
                pass
        flush_log()
        app.quit()

    app.aboutToQuit.connect(lambda: server.stop())
    settings_btn.clicked.connect(on_settings)
    exit_btn.clicked.connect(on_exit)

    server_thread = threading.Thread(
        target=_run_server,
        args=(server,),
        kwargs={"shutdown_callback": lambda: QTimer.singleShot(0, app.quit)},
        daemon=True,
    )
    server_thread.start()

    def _refresh_status():
        status_lbl.setText(f"Server running on {server.host}:{server.port}")

    poll_timer = QTimer()
    poll_timer.setInterval(2000)
    poll_timer.timeout.connect(_refresh_status)
    poll_timer.start()

    win.show()
    logger.info("linux control window shown")
    app.exec()

def main(server_class=None) -> None:
    setup_crash_handler()
    setup_signal_handlers()

    if "-debug" in sys.argv:
        if getattr(sys, "frozen", False) and sys.platform == "win32":
            import ctypes
            ctypes.windll.kernel32.AllocConsole()
            sys.stdout = open("CONOUT$", "w")
            sys.stderr = open("CONOUT$", "w")
            root = logging.getLogger()
            for h in root.handlers[:]:
                root.removeHandler(h)
            from services.logger import _RedactingHandler, _fmt
            root.addHandler(_RedactingHandler(logging.StreamHandler(sys.stdout)))
        logging.getLogger().setLevel(logging.DEBUG)

    args = sys.argv[1:]

    if args and args[0] == "--settings":
        config_path = args[1] if len(args) >= 2 else "config.json"
        from services.settings import run_settings_editor
        run_settings_editor(config_path)
        return

    if args and args[0] == "--port-error":
        error_kind  = args[1] if len(args) >= 2 else "inuse"
        host        = args[2] if len(args) >= 3 else "localhost"
        port        = int(args[3]) if len(args) >= 4 else 4455
        config_path = args[4] if len(args) >= 5 else "config.json"
        from services.dialogs import _run_port_error_process
        _run_port_error_process(error_kind, host, port, config_path)
        return

    if args and args[0] == "--rebind-failed":
        kind        = args[1] if len(args) >= 2 else "inuse"
        failed_host = args[2] if len(args) >= 3 else "localhost"
        failed_port = int(args[3]) if len(args) >= 4 else 4455
        prev_host   = args[4] if len(args) >= 5 else "localhost"
        prev_port   = int(args[5]) if len(args) >= 6 else 4455
        from services.dialogs import _run_rebind_failed_process
        _run_rebind_failed_process(kind, failed_host, failed_port, prev_host, prev_port)
        return

    if args and args[0] == "--update-popup":
        latest       = args[1] if len(args) >= 2 else ""
        config_path  = args[2] if len(args) >= 3 else "config.json"
        release_body = os.environ.get("IOV_UPDATE_BODY", "")
        from services.dialogs import _run_update_popup_process
        _run_update_popup_process(latest, config_path, release_body)
        return

    if sys.platform != "win32":
        from services.dialogs import run_linux_perms_check_and_block
        if not run_linux_perms_check_and_block():
            return

    server = server_class()
    config = server.load_config()

    server.host                   = config.get("host", "localhost")
    server.port                   = config.get("port", 4455)
    server.http_enabled           = config.get("http_enabled", False)
    server.http_port              = config.get("http_port", 4456)
    server.auth_token             = config.get("auth_token", "")
    server.analog_enabled         = config.get("analog_enabled", False)
    server.analog_device          = config.get("analog_device", None)
    server.key_whitelist          = config.get("key_whitelist", [])
    server.balloon_notifications  = config.get("balloon_notifications", True)
    server.raw_mouse_enabled      = config.get("raw_mouse_enabled", False)
    server.raw_mouse_min_delta    = config.get("raw_mouse_min_delta", 0)
    server.linux_raw_mouse_device = config.get("linux_raw_mouse_device", "")

    if sys.platform == "win32":
        _apply_cpu_affinity(config.get("cpu_affinity", [0, 1]))

    child_processes: list = []
    settings_proc: list   = [None]
    server.child_processes = child_processes

    if sys.platform == "win32":
        _run_pystray_tray(server, child_processes, settings_proc)
    else:
        try:
            _run_qt_tray(server, child_processes, settings_proc)
        except Exception:
            logger.warning("qt tray failed, falling back to control window", exc_info=True)
            _run_linux_control_window(server, child_processes, settings_proc)


def _apply_cpu_affinity(cpu_affinity) -> None:
    if not (isinstance(cpu_affinity, list) and cpu_affinity):
        return
    try:
        import ctypes
        mask = 0
        for core in cpu_affinity:
            mask |= 1 << core
        ctypes.windll.kernel32.SetProcessAffinityMask(
            ctypes.windll.kernel32.GetCurrentProcess(), mask
        )
    except Exception:
        logger.warning("failed to set CPU affinity")