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

def get_startup_shortcut_path() -> Path:
    if sys.platform == "win32":
        import winreg
        key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders")
        startup_dir = winreg.QueryValueEx(key, "Startup")[0]
        winreg.CloseKey(key)
        return Path(startup_dir) / "input-overlay-ws.lnk"

    if sys.platform.startswith("linux"):
        autostart_dir = Path.home() / ".config" / "autostart"
        autostart_dir.mkdir(parents=True, exist_ok=True)
        return autostart_dir / "input-overlay-ws.desktop"

    raise NotImplementedError(f"autostart not supported on {sys.platform}")


def is_autostart_enabled() -> bool:
    try:
        return get_startup_shortcut_path().exists()
    except NotImplementedError:
        return False
    except Exception:
        return False


def set_autostart(enabled: bool) -> None:
    try:
        target = get_startup_shortcut_path()
    except NotImplementedError:
        logger.warning("set_autostart: unsupported on whatever this is: %s", sys.platform)
        return

    try:
        if sys.platform == "win32":
            _set_autostart_windows(enabled, target)
        elif sys.platform.startswith("linux"):
            _set_autostart_linux(enabled, target)
    except Exception:
        logger.exception("set_autostart error")


def _set_autostart_windows(enabled: bool, target: Path) -> None:
    if enabled:
        exe = str(get_exe_path()).replace("'", "''")
        lnk = str(target).replace("'", "''")
        work = str(get_exe_path().parent).replace("'", "''")
        ps_cmd = (
            f"$s=(New-Object -COM WScript.Shell).CreateShortcut('{lnk}');"
            f"$s.TargetPath='{exe}';"
            f"$s.WorkingDirectory='{work}';"
            f"$s.Save()"
        )
        subprocess.run(
            ["powershell", "-NoProfile", "-NonInteractive", "-Command", ps_cmd],
            capture_output=True,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
    else:
        if target.exists():
            target.unlink()


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