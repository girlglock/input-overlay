from __future__ import annotations

import atexit
import datetime as _dt
import logging
import os
import re
import signal
import sys
import traceback as _traceback
from pathlib import Path

_SECRET_PATTERNS = [ #for logging
    (re.compile(r"(?i)(sec-websocket-key\s*:\s*)(\S{4})\S+"),    r"\g<1>\g<2>****"),
    (re.compile(r"(?i)(sec-websocket-accept\s*:\s*)(\S{4})\S+"), r"\g<1>\g<2>****"),
    (re.compile(r"(?i)(authorization\s*:\s*)(\S{4})\S+"),        r"\g<1>\g<2>****"),
    (re.compile(r"(?i)(auth[_\-]?token\s*[=:]\s*)(\S{4})\S+"),  r"\g<1>\g<2>****"),
    (re.compile(r"(?i)(wsauth=)(\S{4})\S+"),                     r"\g<1>\g<2>****"),
    (re.compile(r'"token"\s*:\s*"([^"]{4})[^"]*"'),              r'"token": "\1****"'),
    (re.compile(r"'token'\s*:\s*'([^']{4})[^']*'"),              r"'token': '\1****'"),
]


def _redact(text: str) -> str:
    for pattern, repl in _SECRET_PATTERNS:
        text = pattern.sub(repl, text)
    return text


class _RedactingHandler(logging.Handler):
    def __init__(self, inner: logging.Handler) -> None:
        super().__init__()
        self.inner = inner

    def setFormatter(self, fmt: logging.Formatter) -> None:
        self.inner.setFormatter(fmt)

    def emit(self, record: logging.LogRecord) -> None:
        try:
            msg = self.inner.format(record)
            msg = _redact(msg)
            if not msg.strip():
                return
            stream = getattr(self.inner, "stream", None)
            if stream is not None:
                stream.write(msg + self.inner.terminator)
                stream.flush()
            else:
                self.inner.emit(record)
        except Exception:
            self.handleError(record)

    def flush(self) -> None:
        self.inner.flush()

    def close(self) -> None:
        self.inner.close()
        super().close()

def _resolve_logs_dir() -> Path:
    if getattr(sys, "frozen", False):
        base = Path(sys.executable).parent
    else:
        base = Path(__file__).resolve().parent.parent
    logs_dir = base / "logs"
    logs_dir.mkdir(exist_ok=True)
    return logs_dir


_logs_dir = _resolve_logs_dir()

def _process_label() -> str:
    argv = sys.argv[1:] if len(sys.argv) > 1 else []
    if argv:
        flag = argv[0].lstrip("-")
        if flag in ("settings", "update-popup", "port-error", "rebind-failed"):
            return flag.replace("-", "_")
    return "server"

_LOG_FILE = _logs_dir / f"{_dt.datetime.now().strftime('%Y-%m-%d_%H-%M-%S')}_{_process_label()}_{os.getpid()}.log"
_fmt = logging.Formatter("%(asctime)s - %(levelname)s - %(message)s")

_file_handler = _RedactingHandler(
    logging.FileHandler(_LOG_FILE, mode="w", encoding="utf-8", delay=False)
)
_file_handler.inner.setFormatter(_fmt)

_console_handler = _RedactingHandler(logging.StreamHandler(sys.stdout))
_console_handler.inner.setFormatter(_fmt)

_root = logging.getLogger()
_root.setLevel(logging.DEBUG)
_root.addHandler(_file_handler)
_root.addHandler(_console_handler)
logging.getLogger("websockets").setLevel(logging.WARNING)
logging.getLogger("PIL").setLevel(logging.WARNING)

def flush_log() -> None:
    try:
        _file_handler.flush()
        _file_handler.close()
    except Exception:
        pass


atexit.register(flush_log)


def setup_crash_handler() -> None:
    def _crash_handler(exc_type, exc_value, exc_tb):
        logging.getLogger(__name__).critical(
            "unhandled exception:\n%s",
            "".join(_traceback.format_exception(exc_type, exc_value, exc_tb)),
        )
        flush_log()
        sys.__excepthook__(exc_type, exc_value, exc_tb)

    sys.excepthook = _crash_handler


def setup_signal_handlers() -> None:
    def _signal_handler(sig, frame):
        logging.getLogger(__name__).info("received signal %s, shutting down", sig)
        flush_log()
        sys.exit(0)

    for _sig in (signal.SIGTERM, signal.SIGINT):
        try:
            signal.signal(_sig, _signal_handler)
        except (OSError, ValueError):
            pass