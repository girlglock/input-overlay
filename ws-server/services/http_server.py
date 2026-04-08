# HTTP server for serving overlay as a static file locally without network connection
from __future__ import annotations

import http.server
import logging
import threading
from functools import partial
from pathlib import Path

logger = logging.getLogger(__name__)


class _SilentHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        logger.debug("HTTP %s", format % args)

    def end_headers(self):
        self.send_header("Cache-Control", "no-cache")
        super().end_headers()


class LocalHTTPServer:
    def __init__(self, host: str, port: int, web_root: Path) -> None:
        self.host = host
        self.port = port
        self.web_root = web_root
        self._server: http.server.HTTPServer | None = None
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        if not self.web_root.exists():
            logger.error("web root does not exist: %s", self.web_root)
            return

        handler = partial(_SilentHandler, directory=str(self.web_root))
        try:
            self._server = http.server.HTTPServer((self.host, self.port), handler)
        except OSError as e:
            logger.error("failed to start HTTP server on %s:%d: %s", self.host, self.port, e)
            return

        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()
        logger.info("HTTP server started on http://%s:%d (serving %s)", self.host, self.port, self.web_root)

    def stop(self) -> None:
        if self._server:
            self._server.shutdown()
            self._server = None
            self._thread = None
            logger.info("HTTP server stopped")
