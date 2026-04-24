from __future__ import annotations

import json
import logging
import os
import subprocess
import sys
import tempfile
import threading
import urllib.request
import zipfile
from pathlib import Path

from PyQt6.QtCore import QEvent, QObject, QPoint, QSize, Qt, QUrl, pyqtSignal
from PyQt6.QtGui import QColor, QDesktopServices, QFontDatabase, QIcon, QMovie, QPainter
from PyQt6.QtWidgets import (
    QApplication,
    QCheckBox,
    QDialog,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QTextBrowser,
    QToolTip,
    QVBoxLayout,
    QWidget,
)

from services.consts import WS_SERVER_VERSION
from services.utils import get_resource_path, spawn_subprocess

logger = logging.getLogger(__name__)

try:
    import certifi
    import ssl as _ssl
    _SSL_CTX = _ssl.create_default_context(cafile=certifi.where())
except Exception:
    _SSL_CTX = None

GITHUB_RELEASES_URL = "https://github.com/girlglock/input-overlay/releases"
GITHUB_API_URL      = "https://api.github.com/repos/girlglock/input-overlay/releases/latest"
GITHUB_ASSET_NAME   = "input-overlay-ws-windows.zip"
GITHUB_EXE_NAME     = "input-overlay-ws.exe"

class _SegmentedProgressBar(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self._value   = 0
        self._maximum = 100
        self.setFixedHeight(24)

    def setValue(self, value: int) -> None:
        self._value = max(0, min(value, self._maximum))
        self.update()

    def setMaximum(self, maximum: int) -> None:
        self._maximum = maximum
        self.update()

    def paintEvent(self, event) -> None:
        painter = QPainter(self)
        w, h = self.width(), self.height()

        painter.setPen(QColor("#292c21"))
        painter.drawLine(0, 0, w - 1, 0)
        painter.drawLine(0, 0, 0, h - 1)
        painter.setPen(QColor("#8c9284"))
        painter.drawLine(w - 1, 0, w - 1, h - 1)
        painter.drawLine(0, h - 1, w - 1, h - 1)

        pad = 4
        ix, iy = pad, pad
        iw, ih = w - pad * 2, h - pad * 2
        painter.fillRect(ix, iy, iw, ih, QColor("#3e4637"))

        if self._maximum > 0:
            filled = int(iw * self._value / self._maximum)
            seg_w, gap_w = 8, 2
            step = seg_w + gap_w
            x = ix
            while x < ix + filled:
                end = min(x + seg_w, ix + filled)
                if end > x:
                    painter.fillRect(x, iy, end - x, ih, QColor("#c4b550"))
                x += step

        painter.end()

def _render_markdown(text: str) -> str:
    import re as _re
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    text = _re.sub(r"</?(li|br|p|div|span)\b[^>]*>", "", text, flags=_re.IGNORECASE)
    def _dedent_fences(t: str) -> str:
        lines = t.split("\n")
        out = []
        in_fence = False
        fence_indent = 0
        for line in lines:
            stripped = line.lstrip()
            if not in_fence and stripped.startswith("```"):
                in_fence = True
                fence_indent = len(line) - len(stripped)
                out.append(stripped)
            elif in_fence:
                if stripped.startswith("```"):
                    in_fence = False
                    out.append(stripped)
                else:
                    out.append(line[fence_indent:] if line.startswith(" " * fence_indent) else line)
            else:
                out.append(line)
        return "\n".join(out)
    text = _dedent_fences(text)
    try:
        import markdown
        from markdown.extensions.fenced_code import FencedCodeExtension
        from markdown.extensions.nl2br import Nl2BrExtension
        html_body = markdown.markdown(text, extensions=[Nl2BrExtension(), FencedCodeExtension()])
    except ImportError:
        import html as _html
        html_body = "<br>".join(_html.escape(line) for line in text.split("\n"))

    PRE_STYLE  = "background-color:#292c21;padding:6px;margin:4px 0;font-family:monospace;font-size:12px;white-space:pre;border:1px solid #3a3d30;"
    CODE_STYLE = "background-color:#292c21;padding:1px 3px;font-family:monospace;font-size:12px;"

    import re as _re
    html_body = _re.sub(
        r'<pre><code(?:\s[^>]*)?>',
        f"<pre style='{PRE_STYLE}'><code>",
        html_body,
    )

    return (
        "<html><body style='"
        "color:#dedfd6;"
        "font-family:Arial,sans-serif;"
        "font-size:13px;"
        "background-color:#3e4637;"
        "margin:6px;"
        "'>"
        + html_body
        .replace("<h1>", "<h1 style='color:#c4b550;font-size:15px;margin:8px 0 4px 0;border-bottom:1px solid #8c9284;padding-bottom:3px;'>")
        .replace("<h2>", "<h2 style='color:#c4b550;font-size:14px;margin:6px 0 3px 0;border-bottom:1px solid #8c9284;padding-bottom:2px;'>")
        .replace("<h3>", "<h3 style='color:#c4b550;font-size:13px;margin:5px 0 2px 0;'>")
        .replace("<ul>", "<ul style='margin:2px 0 2px 16px;padding:0;'>")
        .replace("<ol>", "<ol style='margin:2px 0 2px 16px;padding:0;'>")
        .replace("<li>", "<li style='margin:1px 0;'>")
        .replace("<a ", "<a style='color:#c4b550;' ")
        .replace("<code>", f"<code style='{CODE_STYLE}'>")
        .replace("<hr />", "<hr style='border:none;border-top:1px solid #8c9284;margin:6px 0;'>")
        .replace("<p>", "<p style='margin:2px 0;'>")
        + "</body></html>"
    )

class _AutoUpdater(QObject):
    progress = pyqtSignal(int, str)
    finished = pyqtSignal(bool, str)

    def __init__(self, latest_version: str) -> None:
        super().__init__()
        self.latest_version = latest_version

    def run(self) -> None:
        try:
            logger.info("auto-update: starting download for v%s", self.latest_version)
            self.progress.emit(0, "fetching release info...")

            req = urllib.request.Request(GITHUB_API_URL, headers={"User-Agent": "input-overlay-ws"})
            with urllib.request.urlopen(req, timeout=10, context=_SSL_CTX) as resp:
                release_data = json.loads(resp.read().decode())

            asset_url = next(
                (a["browser_download_url"] for a in release_data.get("assets", [])
                 if a.get("name") == GITHUB_ASSET_NAME),
                None,
            )
            if not asset_url:
                self.finished.emit(False, f"release asset '{GITHUB_ASSET_NAME}' not found")
                return

            logger.info("auto-update: downloading from %s", asset_url)
            self.progress.emit(10, "downloading...")

            tmp_dir  = Path(tempfile.mkdtemp(prefix="iov_update_"))
            zip_path = tmp_dir / GITHUB_ASSET_NAME

            req2 = urllib.request.Request(asset_url, headers={"User-Agent": "input-overlay-ws"})
            with urllib.request.urlopen(req2, timeout=60, context=_SSL_CTX) as resp:
                total      = int(resp.headers.get("Content-Length", 0))
                downloaded = 0
                with open(zip_path, "wb") as f:
                    while chunk := resp.read(65536):
                        f.write(chunk)
                        downloaded += len(chunk)
                        if total:
                            pct = 10 + int((downloaded / total) * 60)
                            self.progress.emit(pct, f"downloading... {downloaded // 1024}kb / {total // 1024}kb")

            self.progress.emit(72, "extracting...")
            with zipfile.ZipFile(zip_path, "r") as zf:
                members = [m for m in zf.namelist() if Path(m).name == GITHUB_EXE_NAME]
                if not members:
                    self.finished.emit(False, f"'{GITHUB_EXE_NAME}' not found in zip")
                    return
                new_exe = tmp_dir / GITHUB_EXE_NAME
                with zf.open(members[0]) as src, open(new_exe, "wb") as dst:
                    dst.write(src.read())

            self.progress.emit(90, "preparing update...")

            if getattr(sys, "frozen", False) and sys.platform == "win32":
                current_exe = Path(sys.executable).resolve()
                logger.info("auto-update: scheduling replace of %s with %s", current_exe, new_exe)
                try:
                    _schedule_replace_windows(current_exe, new_exe)
                except Exception as e:
                    logger.error("auto-update: _schedule_replace_windows failed: %s", e, exc_info=True)
                    self.finished.emit(False, f"failed to launch updater: {e}")
                    return
                self.progress.emit(100, "restarting...")
                self.finished.emit(True, "")
            else:
                self.finished.emit(False, "_open_browser")

        except Exception as e:
            logger.error("auto-update: failed: %s", e, exc_info=True)
            self.finished.emit(False, str(e))


def _schedule_replace_windows(current_exe: Path, new_exe: Path) -> None:
    old_exe       = current_exe.with_suffix(".old")
    shutdown_flag = current_exe.parent / "shutdown.flag"
    try:
        shutdown_flag.touch()
        logger.info("auto-update: shutdown flag written")
    except Exception as e:
        logger.warning("auto-update: could not write shutdown flag: %s", e)

    def ps(p: Path) -> str:
        return str(p).replace("\\", "/")

    ps1 = Path(tempfile.mktemp(suffix=".ps1", prefix="iov_upd_"))
    script = (
        '$ErrorActionPreference = "Stop"\n'
        'trap { [System.Windows.Forms.MessageBox]::Show("Update failed:`n$_","Error",'
        '[System.Windows.Forms.MessageBoxButtons]::OK,[System.Windows.Forms.MessageBoxIcon]::Error); exit 1 }\n'
        'Add-Type -AssemblyName System.Windows.Forms\n'
        f'$cur = "{ps(current_exe)}"\n'
        f'$new = "{ps(new_exe)}"\n'
        f'$old = "{ps(old_exe)}"\n'
        "Start-Sleep -Milliseconds 800\n"
        "if (Test-Path $old) { Remove-Item $old -Force }\n"
        "Rename-Item -Path $cur -NewName $old -Force\n"
        "Copy-Item -Path $new -Destination $cur -Force\n"
        "Remove-Item $old -Force -ErrorAction SilentlyContinue\n"
        '[System.Windows.Forms.MessageBox]::Show("update finished! reopen input-overlay-ws.exe",'
        '"update finished",[System.Windows.Forms.MessageBoxButtons]::OK,'
        '[System.Windows.Forms.MessageBoxIcon]::Information)\n'
        "exit 0\n"
    )
    ps1.write_text(script, encoding="utf-8")
    logger.info("auto-update: spawning powershell (ps1=%s)", ps1)
    proc = subprocess.Popen(
        ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(ps1)],
        creationflags=subprocess.CREATE_NO_WINDOW,
    )
    logger.info("auto-update: powershell spawned (pid=%d)", proc.pid)


CS16 = """
QMainWindow {
    background-color: #4a5942;
    border: 1px solid;
    border-color: #8c9284 #292c21 #292c21 #8c9284;
}

QWidget {
    color: #dedfd6;
    font-family: "ArialPixel", "Arial", sans-serif;
}

QLabel {
    color: #d8ded3;
    font-size: 16px;
}

QPushButton {
    background-color: #4a5942;
    color: #ffffff;
    border: 1px solid;
    border-color: #8c9284 #292c21 #292c21 #8c9284;
    padding: 4px 4px;
    font-size: 16px;
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
    font-size: 16px;
}

QLineEdit:focus {
    border-color: #c4b550;
}

QCheckBox {
    color: #dedfd6;
    spacing: 8px;
    font-size: 16px;
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
    font-size: 16px;
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
    font-size: 16px;
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
    border: 1px solid;
    border-color: #8c9284 #292c21 #292c21 #8c9284;
}

QTextEdit {
    background-color: #3e4637;
    color: #dedfd6;
    border: 1px solid;
    border-color: #292c21 #8c9284 #8c9284 #292c21;
    font-size: 16px;
    font-family: "ArialPixel", "Arial", sans-serif;
}

#TitleBar {
    background-color: #3a4535;
    border-bottom: 1px solid #292c21;
}

#TitleLabel {
    color: #c4b550;
    font-weight: bold;
    font-size: 16px;
    padding-left: 6px;
}

#TitleBarBtn {
    background-color: #4a5942;
    color: #ffffff;
    border: 1px solid;
    border-color: #8c9284 #292c21 #292c21 #8c9284;
    padding: 4px;
    font-size: 16px;
}

#TitleBarBtn:hover {
    color: #c4b550;
}

#TitleBarBtn:pressed {
    border-color: #292c21 #8c9284 #8c9284 #292c21;
}

#CloseBtn {
    background-color: #4a5942;
    color: #ffffff;
    border: 1px solid;
    border-color: #8c9284 #292c21 #292c21 #8c9284;
    padding: 4px;
    font-size: 16px;
}

#CloseBtn:hover {
    color: #ff6060;
}

#CloseBtn:pressed {
    border-color: #292c21 #8c9284 #8c9284 #292c21;
}

QToolTip {
    background-color: #958831;
    color: #000000;
    border: 1px solid #292c21;
    padding: 2px 2px 1px;
    font-size: 16px;
    font-family: "ArialPixel", "Arial", sans-serif;
}
"""

def dwm_sharp_corners(hwnd: int) -> None:
    #tell dwm to stfu and do sharp corners
    if sys.platform != "win32":
        return
    try:
        import ctypes
        DWMWA_WINDOW_CORNER_PREFERENCE = 33
        DWMWCP_DONOTROUND              = 1
        ctypes.windll.dwmapi.DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            ctypes.byref(ctypes.c_int(DWMWCP_DONOTROUND)),
            ctypes.sizeof(ctypes.c_int),
        )
    except Exception:
        pass


class _SharpCornersMixin:
    def showEvent(self, event):
        super().showEvent(event)
        dwm_sharp_corners(int(self.winId()))


def _load_pixel_font() -> None:
    font_path = get_resource_path("assets/arialpixel.ttf")
    if font_path.exists():
        font_id = QFontDatabase.addApplicationFont(str(font_path))
        if font_id == -1:
            logger.warning("arialpixel.ttf could not be loaded by Qt")
        else:
            families = QFontDatabase.applicationFontFamilies(font_id)
            logger.debug("loaded pixel font families: %s", families)
    else:
        logger.warning("arialpixel.ttf not found at %s", font_path)


class _InstantTooltipMixin:
    def _init_instant_tooltip(self):
        self.setMouseTracking(True)
        self._tooltip_shown = False

    def event(self, e):
        if e.type() == QEvent.Type.HoverEnter:
            QToolTip.showText(self.mapToGlobal(self.rect().center()), self.toolTip(), self)
            self._tooltip_shown = True
            return True
        if e.type() == QEvent.Type.HoverLeave:
            QToolTip.hideText()
            self._tooltip_shown = False
            return True
        if e.type() == QEvent.Type.ToolTip:
            return True
        return super().event(e)


class InstantTooltipCheckBox(_InstantTooltipMixin, QCheckBox):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._init_instant_tooltip()


class InstantTooltipLabel(_InstantTooltipMixin, QLabel):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._init_instant_tooltip()


class InstantTooltipLineEdit(_InstantTooltipMixin, QLineEdit):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._init_instant_tooltip()


class InstantTooltipPushButton(_InstantTooltipMixin, QPushButton):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._init_instant_tooltip()


class CS16DisabledButton(QPushButton):
    _SHADOW_COLOR   = QColor("#75806f")
    _DISABLED_COLOR = QColor("#292c21")

    def paintEvent(self, event) -> None:
        if not self.isEnabled():
            from PyQt6.QtGui import QPainter, QColor
            from PyQt6.QtCore import Qt
            painter = QPainter(self)
            painter.setRenderHint(QPainter.RenderHint.Antialiasing, False)

            opt = self._get_style_option()
            self.style().drawControl(
                self.style().ControlElement.CE_PushButtonBevel, opt, painter, self
            )

            rect = self.rect()
            font = self.font()
            painter.setFont(font)

            #shadow
            shadow_rect = rect.translated(1, 1)
            painter.setPen(self._SHADOW_COLOR)
            painter.drawText(shadow_rect, Qt.AlignmentFlag.AlignCenter, self.text())

            #text
            painter.setPen(self._DISABLED_COLOR)
            painter.drawText(rect, Qt.AlignmentFlag.AlignCenter, self.text())

            painter.end()
        else:
            super().paintEvent(event)

    def _get_style_option(self):
        from PyQt6.QtWidgets import QStyleOptionButton
        opt = QStyleOptionButton()
        self.initStyleOption(opt)
        return opt

class TitleBar(QWidget):
    def __init__(self, title: str, parent_window, minimizable: bool = True) -> None:
        super().__init__(parent_window)
        self.setObjectName("TitleBar")
        self.setFixedHeight(26)
        self._parent    = parent_window
        self._drag_pos: QPoint | None = None

        layout = QHBoxLayout(self)
        layout.setContentsMargins(4, 4, 4, 4)
        layout.setSpacing(2)

        lbl = QLabel(title)
        lbl.setObjectName("TitleLabel")
        lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        layout.addWidget(lbl)
        layout.addStretch()

        if minimizable:
            min_btn = QPushButton("-")
            min_btn.setObjectName("TitleBarBtn")
            min_btn.setFocusPolicy(Qt.FocusPolicy.NoFocus)
            min_btn.setFixedSize(18, 18)
            min_btn.clicked.connect(parent_window.showMinimized)
            layout.addWidget(min_btn)

        close_btn = QPushButton("X")
        close_btn.setObjectName("CloseBtn")
        close_btn.setFocusPolicy(Qt.FocusPolicy.NoFocus)
        close_btn.setFixedSize(18, 18)
        close_btn.clicked.connect(parent_window.close)
        layout.addWidget(close_btn)

    def mousePressEvent(self, event) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            #on wayland use native system mover
            if sys.platform != "win32":
                win_handle = self._parent.windowHandle()
                if win_handle is not None:
                    win_handle.startSystemMove()
                    event.accept()
                    return
            self._drag_pos = event.globalPosition().toPoint() - self._parent.frameGeometry().topLeft()
            event.accept()

    def mouseMoveEvent(self, event) -> None:
        if self._drag_pos is not None and event.buttons() == Qt.MouseButton.LeftButton:
            self._parent.move(event.globalPosition().toPoint() - self._drag_pos)
            event.accept()

    def mouseReleaseEvent(self, event) -> None:
        self._drag_pos = None


class UpdateChecker(QObject):
    update_available = pyqtSignal(str, str)   # latest_version, release_body
    check_done       = pyqtSignal()

    def check(self, dismissed: list) -> None:
        def _run():
            try:
                req = urllib.request.Request(GITHUB_API_URL, headers={"User-Agent": "input-overlay-ws"})
                with urllib.request.urlopen(req, timeout=5, context=_SSL_CTX) as resp:
                    data = json.loads(resp.read().decode())
                latest = data.get("tag_name", "").lstrip("v")
                body   = data.get("body", "").strip()
                if latest and latest != WS_SERVER_VERSION and latest not in dismissed:
                    self.update_available.emit(latest, body)
            except Exception as e:
                logger.debug("update check failed: %s", e)
            finally:
                self.check_done.emit()
        threading.Thread(target=_run, daemon=True).start()

class UpdateDialog(_SharpCornersMixin, QDialog):
    def __init__(self, latest_version: str, release_body: str = "", parent=None) -> None:
        super().__init__(parent)
        self.latest_version = latest_version
        self.release_body   = release_body
        self.dismissed      = False
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.Dialog)
        self.setFixedWidth(580)
        icon_path = get_resource_path("assets/icon.ico")
        if icon_path.exists():
            self.setWindowIcon(QIcon(str(icon_path)))
        self.setStyleSheet(CS16)
        self._build_ui()

    def _build_ui(self) -> None:
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        self._title_bar = TitleBar("UPDATE AVAILABLE", self, minimizable=False)
        layout.addWidget(self._title_bar)

        inner        = QWidget()
        inner_layout = QVBoxLayout(inner)
        inner_layout.setContentsMargins(20, 12, 20, 20)
        inner_layout.setSpacing(12)

        content_row = QHBoxLayout()
        content_row.setSpacing(12)

        text_col = QVBoxLayout()
        text_col.setSpacing(8)

        title = QLabel("A new version of Input Overlay WebSocket Server is available!")
        title.setStyleSheet("color: #c4b550; font-weight: bold; font-size: 16px;")
        title.setWordWrap(True)
        text_col.addWidget(title)

        body = QLabel(
            f"Current version: <b>{WS_SERVER_VERSION}</b><br>"
            f"Latest version:  <b>{self.latest_version}</b>"
        )
        body.setStyleSheet("color: #dedfd6; font-size: 16px;")
        body.setTextFormat(Qt.TextFormat.RichText)
        text_col.addWidget(body)
        text_col.addStretch()
        content_row.addLayout(text_col)

        gif_label = QLabel()
        gif_path  = get_resource_path("assets/update.gif")
        if gif_path.exists():
            self.movie = QMovie(str(gif_path))
            self.movie.setScaledSize(QSize(80, 80))
            gif_label.setMovie(self.movie)
            self.movie.start()
        gif_label.setAlignment(Qt.AlignmentFlag.AlignBottom | Qt.AlignmentFlag.AlignRight)
        content_row.addWidget(gif_label)

        inner_layout.addLayout(content_row)

        if self.release_body:
            notes_label = QLabel("PATCH NOTES")
            notes_label.setStyleSheet("color: #c4b550; font-weight: bold; font-size: 16px;")
            inner_layout.addWidget(notes_label)

            notes_box = QTextBrowser()
            notes_box.setReadOnly(True)
            notes_box.setOpenExternalLinks(True)
            notes_box.setHtml(_render_markdown(self.release_body))
            notes_box.setFixedHeight(160)
            notes_box.setStyleSheet(
                "QTextBrowser { background-color: #3e4637; border: 1px solid;"
                " border-color: #292c21 #8c9284 #8c9284 #292c21; }"
            )
            inner_layout.addWidget(notes_box)

        self._progress_bar = _SegmentedProgressBar()
        self._progress_bar.hide()
        inner_layout.addWidget(self._progress_bar)

        self._status_label = QLabel("")
        self._status_label.setStyleSheet("color: #a0aa95; font-size: 14px;")
        self._status_label.hide()
        inner_layout.addWidget(self._status_label)

        btn_row = QHBoxLayout()
        btn_row.setSpacing(4)

        self._download_btn = QPushButton("OPEN RELEASE PAGE" if sys.platform != "win32" else "DOWNLOAD & INSTALL")
        self._download_btn.setMinimumHeight(32)
        self._download_btn.clicked.connect(self._on_download)
        btn_row.addWidget(self._download_btn)

        dismiss_btn = QPushButton("DISMISS THIS VERSION")
        dismiss_btn.setMinimumHeight(32)
        dismiss_btn.clicked.connect(self._on_dismiss)
        btn_row.addWidget(dismiss_btn)

        later_btn = QPushButton("REMIND ON NEXT START")
        later_btn.setMinimumHeight(32)
        later_btn.clicked.connect(self._on_later)
        btn_row.addWidget(later_btn)

        inner_layout.addLayout(btn_row)
        layout.addWidget(inner)

    def _on_download(self) -> None:
        if sys.platform != "win32":
            QDesktopServices.openUrl(QUrl(GITHUB_RELEASES_URL))
            self.accept()
            return

        if not getattr(sys, "frozen", False):
            self._download_btn.setEnabled(False)
            self._status_label.show()
            self._status_label.setText("auto-update requires the compiled exe")
            return

        self._download_btn.setEnabled(False)
        self._download_btn.setText("DOWNLOADING...")
        self._progress_bar.show()
        self._status_label.show()
        self.setFixedHeight(self.sizeHint().height())

        self._updater = _AutoUpdater(self.latest_version)
        self._updater.progress.connect(self._on_progress, Qt.ConnectionType.QueuedConnection)
        self._updater.finished.connect(self._on_update_finished, Qt.ConnectionType.QueuedConnection)
        self._thread = threading.Thread(target=self._updater.run, daemon=True)
        self._thread.start()

    def _on_progress(self, pct: int, msg: str) -> None:
        self._progress_bar.setValue(pct)
        self._status_label.setText(msg)

    def _on_update_finished(self, success: bool, error: str) -> None:
        if success:
            self._status_label.setText("update ready - restarting...")
            self._progress_bar.setValue(100)
            from PyQt6.QtCore import QTimer
            QTimer.singleShot(1200, self.accept)
        elif error == "_open_browser":
            QDesktopServices.openUrl(QUrl(GITHUB_RELEASES_URL))
            self.accept()
        else:
            self._download_btn.setEnabled(True)
            self._download_btn.setText("DOWNLOAD & INSTALL")
            self._progress_bar.hide()
            self._status_label.setStyleSheet("color: #ff6060; font-size: 14px;")
            self._status_label.setText(f"update failed: {error}")
            logger.error("auto-update failed: %s", error)

    def _on_dismiss(self) -> None:
        self.dismissed = True
        self.accept()

    def _on_later(self) -> None:
        self.accept()



def check_for_updates_on_startup(config_path: str = "config.json",
                                  child_processes: list = None) -> None:
    try:
        with open(config_path, "r") as f:
            config = json.load(f)
        dismissed = config.get("dismissed_versions", [])
    except Exception:
        dismissed = []

    def _run() -> None:
        try:
            req = urllib.request.Request(GITHUB_API_URL, headers={"User-Agent": "input-overlay-ws"})
            with urllib.request.urlopen(req, timeout=5, context=_SSL_CTX) as resp:
                data = json.loads(resp.read().decode())
            latest = data.get("tag_name", "").lstrip("v")
            body   = data.get("body", "").strip()
            if latest and latest != WS_SERVER_VERSION and latest not in dismissed:
                env = os.environ.copy()
                env["IOV_UPDATE_BODY"] = body
                proc = spawn_subprocess("--update-popup", latest, config_path, env=env)
                if proc and child_processes is not None:
                    child_processes.append(proc)
        except Exception as e:
            logger.debug("startup update check failed: %s", e)

    threading.Thread(target=_run, daemon=True).start()

def _run_update_popup_process(latest_version: str, config_path: str,
                               release_body: str = "") -> None:
    app = QApplication(sys.argv)
    _load_pixel_font()
    dlg = UpdateDialog(latest_version, release_body)
    dlg.setStyleSheet(CS16)
    dlg.exec()
    if dlg.dismissed:
        try:
            with open(config_path, "r") as f:
                cfg = json.load(f)
            dismissed = cfg.get("dismissed_versions", [])
            if latest_version not in dismissed:
                dismissed.append(latest_version)
            cfg["dismissed_versions"] = dismissed
            with open(config_path, "w") as f:
                json.dump(cfg, f, indent=4)
        except Exception as e:
            logger.error("could not save dismissed version: %s", e)


def check_linux_permissions() -> tuple[bool, list[str]]:
    import grp
    import os
    import stat

    missing: list[str] = []
    try:
        input_gid = grp.getgrnam("input").gr_gid
        user_groups = os.getgroups()
        if input_gid not in user_groups:
            missing.append("input group")
    except KeyError:
        missing.append("input group (group does not exist)")

    import glob
    hidraw_devices = glob.glob("/dev/hidraw*")
    if hidraw_devices:
        try:
            input_gid = grp.getgrnam("input").gr_gid
        except KeyError:
            input_gid = -1
        any_accessible = False
        for dev in hidraw_devices:
            try:
                st = os.stat(dev)
                group_readable = bool(st.st_mode & stat.S_IRGRP)
                if st.st_gid == input_gid and group_readable:
                    any_accessible = True
                    break
            except OSError:
                pass
        if not any_accessible:
            missing.append("hidraw udev rule")

    ok = len(missing) == 0
    return ok, missing


class LinuxPermsDialog(_SharpCornersMixin, QDialog):
    def __init__(self, missing: list[str], parent=None) -> None:
        super().__init__(parent)
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint | Qt.WindowType.Dialog)
        self.setFixedWidth(480)
        icon_path = get_resource_path("assets/icon.ico")
        if icon_path.exists():
            self.setWindowIcon(QIcon(str(icon_path)))
        self.setStyleSheet(CS16)
        self._build_ui(missing)

    def _build_ui(self, missing: list[str]) -> None:
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        title_bar = TitleBar("PERMISSIONS REQUIRED", self, minimizable=False)
        layout.addWidget(title_bar)

        inner = QWidget()
        inner_layout = QVBoxLayout(inner)
        inner_layout.setContentsMargins(20, 16, 20, 20)
        inner_layout.setSpacing(12)

        heading = QLabel("Missing device permissions")
        heading.setStyleSheet("color: #c4b550; font-weight: bold; font-size: 16px;")
        inner_layout.addWidget(heading)

        missing_str = "".join(f"&nbsp;&nbsp;• {m}<br>" for m in missing)
        body = QLabel(
            f"Input Overlay needs access to input and HID devices, "
            f"but the following permissions are missing:<br><br>"
            f"{missing_str}<br>"
            f"Click <b>Instructions</b> to open the setup guide, then re-run the app "
            f"after following the steps."
        )
        body.setWordWrap(True)
        body.setTextFormat(Qt.TextFormat.RichText)
        body.setStyleSheet("color: #dedfd6; font-size: 15px;")
        inner_layout.addWidget(body)

        inner_layout.addSpacing(4)

        btn_row = QHBoxLayout()
        btn_row.setSpacing(6)

        instructions_btn = QPushButton("INSTRUCTIONS")
        instructions_btn.setMinimumHeight(32)
        instructions_btn.clicked.connect(self._on_instructions)
        btn_row.addWidget(instructions_btn)

        exit_btn = QPushButton("EXIT")
        exit_btn.setMinimumHeight(32)
        exit_btn.clicked.connect(self._on_exit)
        btn_row.addWidget(exit_btn)

        inner_layout.addLayout(btn_row)
        layout.addWidget(inner)

    def _on_instructions(self) -> None:
        QDesktopServices.openUrl(QUrl(GITHUB_RELEASES_URL))

    def _on_exit(self) -> None:
        self.accept()


def run_linux_perms_check_and_block() -> bool:
    if sys.platform == "win32":
        return True

    ok, missing = check_linux_permissions()
    if ok:
        return True

    logger.warning("linux perms check failed: %s", missing)

    app = QApplication.instance() or QApplication(sys.argv)
    _load_pixel_font()
    dlg = LinuxPermsDialog(missing)
    dlg.exec()
    return False