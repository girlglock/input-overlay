from __future__ import annotations

import logging
import subprocess
import sys
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)

def get_resource_path(relative_path: str) -> Path:
    try:
        base = Path(sys._MEIPASS)
    except AttributeError:
        base = Path(__file__).resolve().parent.parent
    return base / relative_path


def get_web_root() -> Path:
    try:
        bundled = Path(sys._MEIPASS) / "web"
        if bundled.is_dir():
            return bundled
    except AttributeError:
        pass
    # development: repo root is parent of ws-server/
    repo_root = Path(__file__).resolve().parent.parent.parent
    if (repo_root / "index.html").exists():
        return repo_root
    return Path.cwd()


def get_exe_path() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable)
    return Path(__file__).resolve().parent.parent / "input-overlay-ws.py"

def spawn_subprocess(
    *cli_args: str,
    env: Optional[dict] = None,
    no_window: bool = True,
) -> Optional[subprocess.Popen]:
    exe = get_exe_path()
    if getattr(sys, "frozen", False):
        cmd = [str(exe), *cli_args]
    else:
        cmd = [sys.executable, str(exe), *cli_args]

    flags = 0
    if no_window and sys.platform == "win32":
        flags = subprocess.CREATE_NO_WINDOW

    try:
        proc = subprocess.Popen(cmd, env=env, creationflags=flags)
        logger.debug("spawned subprocess: %s (pid=%d)", " ".join(cmd), proc.pid)
        return proc
    except Exception:
        logger.exception("failed to spawn subprocess with args: %s", cli_args)
        return None

_TASK_NAME = "InputOverlayWS"


def _get_linux_autostart_path() -> Path:
    autostart_dir = Path.home() / ".config" / "autostart"
    autostart_dir.mkdir(parents=True, exist_ok=True)
    return autostart_dir / "input-overlay-ws.desktop"


def is_autostart_enabled() -> bool:
    try:
        if sys.platform == "win32":
            result = subprocess.run(
                ["schtasks", "/Query", "/TN", _TASK_NAME],
                capture_output=True,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
            return result.returncode == 0
        if sys.platform.startswith("linux"):
            return _get_linux_autostart_path().exists()
    except Exception:
        pass
    return False


def set_autostart(enabled: bool) -> None:
    try:
        if sys.platform == "win32":
            _set_autostart_windows(enabled)
        elif sys.platform.startswith("linux"):
            _set_autostart_linux(enabled, _get_linux_autostart_path())
        else:
            logger.warning("set_autostart: unsupported on whatever this is: %s", sys.platform)
    except Exception:
        logger.exception("set_autostart error")


def _set_autostart_windows(enabled: bool) -> None:
    if enabled:
        import tempfile
        import os
        exe = str(get_exe_path())
        work = str(get_exe_path().parent)
        xml = (
            '<?xml version="1.0" encoding="UTF-16"?>'
            '<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">'
            "<Triggers><LogonTrigger><Enabled>true</Enabled></LogonTrigger></Triggers>"
            "<Principals><Principal><LogonType>InteractiveToken</LogonType>"
            "<RunLevel>HighestAvailable</RunLevel></Principal></Principals>"
            "<Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"
            "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>"
            "<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>"
            "<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"
            "<Priority>7</Priority></Settings>"
            f"<Actions><Exec><Command>{exe}</Command>"
            f"<WorkingDirectory>{work}</WorkingDirectory></Exec></Actions>"
            "</Task>"
        )
        tmp = None
        try:
            fd, tmp = tempfile.mkstemp(suffix=".xml")
            with os.fdopen(fd, "w", encoding="utf-16") as f:
                f.write(xml)
            result = subprocess.run(
                ["schtasks", "/Create", "/F", "/TN", _TASK_NAME, "/XML", tmp],
                capture_output=True,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
            if result.returncode != 0:
                logger.error("schtasks create failed: %s", result.stderr.decode(errors="replace").strip())
            else:
                logger.info("autostart task created")
        finally:
            if tmp:
                try:
                    os.unlink(tmp)
                except Exception:
                    pass
    else:
        result = subprocess.run(
            ["schtasks", "/Delete", "/F", "/TN", _TASK_NAME],
            capture_output=True,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        if result.returncode != 0:
            logger.warning("schtasks delete failed: %s", result.stderr.decode(errors="replace").strip())
        else:
            logger.info("autostart task removed")


def _set_autostart_linux(enabled: bool, target: Path) -> None:
    if enabled:
        exe_path = get_exe_path()
        desktop_content = (
            "[Desktop Entry]\n"
            "Type=Application\n"
            "Name=Input Overlay Server\n"
            f"Exec={exe_path}\n"
            f"Path={exe_path.parent}\n"
            "Hidden=false\n"
            "NoDisplay=false\n"
            "X-GNOME-Autostart-enabled=true\n"
        )
        target.write_text(desktop_content)
    else:
        if target.exists():
            target.unlink()