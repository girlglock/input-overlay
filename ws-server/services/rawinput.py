from __future__ import annotations

import sys

if sys.platform != 'win32':
    raise ImportError("rawinputbuffer is only supported on bimbows")

import ctypes
import ctypes.wintypes as wt
import logging
import threading
import time
from typing import Callable

from services.consts import RAW_MOUSE_FLUSH_HZ

logger = logging.getLogger(__name__)

WM_INPUT            = 0x00FF
WM_QUIT             = 0x0012
RIM_TYPEMOUSE       = 0
RIDEV_INPUTSINK     = 0x00000100
RIDEV_REMOVE        = 0x00000001
RID_INPUT           = 0x10000003
WS_EX_TOOLWINDOW    = 0x00000080
WS_EX_NOACTIVATE    = 0x08000000

SM_CXSCREEN = 0
SM_CYSCREEN = 1

MOUSEEVENTF_ABSOLUTE = 0x0001


class RAWINPUTDEVICE(ctypes.Structure):
    _fields_ = [
        ("usUsagePage", wt.USHORT),
        ("usUsage",     wt.USHORT),
        ("dwFlags",     wt.DWORD),
        ("hwndTarget",  wt.HWND),
    ]


class RAWMOUSE(ctypes.Structure):
    class _U(ctypes.Union):
        class _S(ctypes.Structure):
            _fields_ = [("usButtonFlags", wt.USHORT), ("usButtonData", wt.USHORT)]
        _fields_ = [("_s", _S), ("ulButtons", ctypes.c_ulong)]

    _fields_ = [
        ("usFlags",            wt.USHORT),
        ("_u",                 _U),
        ("ulRawButtons",       ctypes.c_ulong),
        ("lLastX",             ctypes.c_long),
        ("lLastY",             ctypes.c_long),
        ("ulExtraInformation", ctypes.c_ulong),
    ]


class RAWINPUTHEADER(ctypes.Structure):
    _fields_ = [
        ("dwType",  wt.DWORD),
        ("dwSize",  wt.DWORD),
        ("hDevice", wt.HANDLE),
        ("wParam",  wt.WPARAM),
    ]


class RAWINPUT(ctypes.Structure):
    class _DATA(ctypes.Union):
        _fields_ = [("mouse", RAWMOUSE)]

    _fields_ = [("header", RAWINPUTHEADER), ("data", _DATA)]


_user32   = ctypes.windll.user32
_kernel32 = ctypes.windll.kernel32

_LRESULT = ctypes.c_longlong
_WNDPROC = ctypes.WINFUNCTYPE(_LRESULT, wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM)

_kernel32.GetModuleHandleW.restype  = ctypes.c_void_p
_kernel32.GetModuleHandleW.argtypes = [wt.LPCWSTR]

_user32.CreateWindowExW.restype  = wt.HWND
_user32.CreateWindowExW.argtypes = [
    wt.DWORD,
    wt.LPCWSTR,
    wt.LPCWSTR,
    wt.DWORD,
    ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
    wt.HWND,
    wt.HANDLE,
    ctypes.c_void_p,
    ctypes.c_void_p,
]

_user32.DefWindowProcW.restype  = _LRESULT
_user32.DefWindowProcW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]

_user32.GetRawInputData.restype  = wt.UINT
_user32.GetRawInputData.argtypes = [
    wt.HANDLE,
    wt.UINT,
    ctypes.c_void_p,
    ctypes.POINTER(wt.UINT),
    wt.UINT,
]


def _get_raw_input(lParam: int) -> RAWINPUT | None:
    buf_size = wt.UINT(0)
    ret = _user32.GetRawInputData(lParam, RID_INPUT, None, ctypes.byref(buf_size), ctypes.sizeof(RAWINPUTHEADER))
    if ret != 0:
        logger.debug("raw_mouse: GetRawInputData (size query) returned %d, expected 0", ret)
        return None
    if buf_size.value == 0:
        logger.warning("raw_mouse: GetRawInputData reported 0 buffer size")
        return None
    buf = ctypes.create_string_buffer(buf_size.value)
    filled = _user32.GetRawInputData(lParam, RID_INPUT, buf, ctypes.byref(buf_size), ctypes.sizeof(RAWINPUTHEADER))
    if filled != buf_size.value:
        logger.warning("raw_mouse: GetRawInputData size mismatch (got %d, expected %d)", filled, buf_size.value)
        return None
    return RAWINPUT.from_buffer_copy(buf)


class RawMouseThread(threading.Thread):
    FLUSH_HZ = RAW_MOUSE_FLUSH_HZ

    def __init__(self, callback: Callable[[int, int], None] | None = None, absolute_callback: Callable[[int, int, bool], None] | None = None, min_delta: int = 0, daemon: bool = True):
        super().__init__(daemon=daemon, name="RawMouseThread")
        self._callback = callback
        self._absolute_callback = absolute_callback
        self._min_delta = min_delta
        self._hwnd: int | None = None
        self._lock = threading.Lock()
        self._accum_dx = 0
        self._accum_dy = 0
        self._filtered_count = 0

        self._last_abs_x: int | None = None
        self._last_abs_y: int | None = None
        self._is_pen_down = False
        self._abs_timeout_thread: threading.Thread | None = None
        self._abs_timeout_running = threading.Event()
        self._abs_timeout_lock = threading.Lock()
        self._last_abs_event_time: float = 0

    def stop(self):
        if self._hwnd:
            _user32.PostMessageW(self._hwnd, WM_QUIT, 0, 0)
        self._abs_timeout_running.clear()
        if self._abs_timeout_thread:
            self._abs_timeout_thread.join(timeout=0.5)
            self._abs_timeout_thread = None
        self.join(timeout=2.0)

    def run(self):
        try:
            self._abs_timeout_running.set()
            self._abs_timeout_thread = threading.Thread(target=self._abs_timeout_loop, daemon=True, name="AbsTimeout")
            self._abs_timeout_thread.start()

            logger.debug("raw_mouse: thread starting, registering window class")
            self._hwnd = self._create_window()
            if not self._hwnd:
                logger.error("raw_mouse: CreateWindowEx failed (error %d)", _kernel32.GetLastError())
                return
            logger.debug("raw_mouse: window created (hwnd=0x%x)", self._hwnd)

            if not self._register():
                logger.error("raw_mouse: RegisterRawInputDevices failed (error %d)", _kernel32.GetLastError())
                _user32.DestroyWindow(self._hwnd)
                return
            logger.info("raw_mouse: listener started (hwnd=0x%x, min_delta=%d)", self._hwnd, self._min_delta)

            flush_thread = threading.Thread(target=self._flush_loop, daemon=True, name="RawMouseFlush")
            flush_thread.start()
            logger.debug("raw_mouse: flush loop thread started (tid=%d, hz=%d)", flush_thread.ident, self.FLUSH_HZ)

            self._pump()
        except Exception:
            logger.exception("raw_mouse: unhandled error in run()")
        finally:
            logger.debug("raw_mouse: cleaning up")
            self._abs_timeout_running.clear()
            self._unregister()
            if self._hwnd:
                _user32.DestroyWindow(self._hwnd)
                self._hwnd = None
            logger.info("raw_mouse: listener stopped")

    def _abs_timeout_loop(self):
        while self._abs_timeout_running.is_set():
            time.sleep(0.1)
            with self._abs_timeout_lock:
                if self._is_pen_down:
                    now = time.monotonic()
                    if now - self._last_abs_event_time >= 0.2:
                        self._is_pen_down = False
                        if (self._absolute_callback and self._last_abs_x is not None and self._last_abs_y is not None):
                            try:
                                self._absolute_callback(self._last_abs_x, self._last_abs_y, False)
                            except Exception:
                                logger.exception("raw_mouse: exception in absolute_callback (pen up)")
                            self._last_abs_x = None
                            self._last_abs_y = None

    def _flush_loop(self):
        interval = 1.0 / self.FLUSH_HZ
        logger.debug("raw_mouse: flush loop running (interval=%.4fs)", interval)
        while True:
            time.sleep(interval)
            with self._lock:
                dx, dy = self._accum_dx, self._accum_dy
                self._accum_dx = 0
                self._accum_dy = 0
            if dx == 0 and dy == 0:
                continue
            try:
                if self._callback:
                    self._callback(dx, dy)
            except Exception:
                logger.exception("raw_mouse: exception in flush callback")

    def _create_window(self) -> int | None:
        def _wnd_proc(hwnd, msg, wParam, lParam):
            if msg == WM_INPUT:
                self._on_wm_input(lParam)
            return _user32.DefWindowProcW(hwnd, msg, wParam, lParam)

        self._wnd_proc_ref = _WNDPROC(_wnd_proc)

        class WNDCLASSEX(ctypes.Structure):
            _fields_ = [
                ("cbSize",        wt.UINT),    ("style",         wt.UINT),
                ("lpfnWndProc",   _WNDPROC),   ("cbClsExtra",    ctypes.c_int),
                ("cbWndExtra",    ctypes.c_int),("hInstance",     ctypes.c_void_p),
                ("hIcon",         wt.HANDLE),  ("hCursor",       wt.HANDLE),
                ("hbrBackground", wt.HANDLE),  ("lpszMenuName",  wt.LPCWSTR),
                ("lpszClassName", wt.LPCWSTR), ("hIconSm",       wt.HANDLE),
            ]

        class_name = "IOvRawMouse"
        wc = WNDCLASSEX()
        wc.cbSize        = ctypes.sizeof(WNDCLASSEX)
        wc.lpfnWndProc   = self._wnd_proc_ref
        wc.hInstance     = _kernel32.GetModuleHandleW(None)
        wc.lpszClassName = class_name

        atom = _user32.RegisterClassExW(ctypes.byref(wc))
        if atom == 0:
            logger.warning("raw_mouse: RegisterClassExW returned 0 (error %d) - class may already exist", _kernel32.GetLastError())
        else:
            logger.debug("raw_mouse: window class registered (atom=0x%x)", atom)

        hwnd = _user32.CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name, None, 0,
            0, 0, 0, 0,
            None, None, wc.hInstance, None,
        )
        if hwnd:
            logger.debug("raw_mouse: message window created successfully")
        else:
            logger.error("raw_mouse: CreateWindowExW returned NULL (error %d)", _kernel32.GetLastError())
        return hwnd or None

    def _register(self) -> bool:
        devices = (RAWINPUTDEVICE * 2)()
        devices[0].usUsagePage  = 0x01
        devices[0].usUsage      = 0x02
        devices[0].dwFlags      = RIDEV_INPUTSINK
        devices[0].hwndTarget   = self._hwnd
        devices[1].usUsagePage  = 0x01
        devices[1].usUsage      = 0x01
        devices[1].dwFlags      = RIDEV_INPUTSINK
        devices[1].hwndTarget   = self._hwnd
        result = bool(_user32.RegisterRawInputDevices(devices, 2, ctypes.sizeof(RAWINPUTDEVICE)))
        if result:
            logger.debug("raw_mouse: RegisterRawInputDevices succeeded (INPUTSINK on hwnd=0x%x)", self._hwnd)
        else:
            logger.error("raw_mouse: RegisterRawInputDevices failed (error %d)", _kernel32.GetLastError())
        return result

    def _unregister(self):
        devices = (RAWINPUTDEVICE * 2)()
        devices[0].usUsagePage  = 0x01
        devices[0].usUsage      = 0x02
        devices[0].dwFlags      = RIDEV_REMOVE
        devices[0].hwndTarget   = None
        devices[1].usUsagePage  = 0x01
        devices[1].usUsage      = 0x01
        devices[1].dwFlags      = RIDEV_REMOVE
        devices[1].hwndTarget   = None
        result = bool(_user32.RegisterRawInputDevices(devices, 2, ctypes.sizeof(RAWINPUTDEVICE)))
        if result:
            logger.debug("raw_mouse: unregistered raw input device")
        else:
            logger.warning("raw_mouse: unregister failed (error %d)", _kernel32.GetLastError())

    def _pump(self):
        class MSG(ctypes.Structure):
            _fields_ = [
                ("hwnd",    wt.HWND),   ("message", wt.UINT),
                ("wParam",  wt.WPARAM), ("lParam",  wt.LPARAM),
                ("time",    wt.DWORD),  ("pt",      wt.POINT),
            ]

        logger.debug("raw_mouse: entering message pump")
        msg = MSG()
        msg_count = 0
        while _user32.GetMessageW(ctypes.byref(msg), None, 0, 0) > 0:
            msg_count += 1
            if msg_count <= 5:
                logger.debug("raw_mouse: pump received message #%d (msg=0x%x)", msg_count, msg.message)
            _user32.TranslateMessage(ctypes.byref(msg))
            _user32.DispatchMessageW(ctypes.byref(msg))
        logger.debug("raw_mouse: message pump exited (total messages processed: %d)", msg_count)

    def _on_wm_input(self, lParam: int):
        ri = _get_raw_input(lParam)
        if ri is None:
            logger.debug("raw_mouse: _get_raw_input returned None")
            return
        if ri.header.dwType != RIM_TYPEMOUSE:
            logger.debug("raw_mouse: ignoring non-mouse rawinput (dwType=%d)", ri.header.dwType)
            return
        m = ri.data.mouse
        is_absolute = bool(m.usFlags & MOUSEEVENTF_ABSOLUTE)
        if is_absolute:
            if not self._absolute_callback:
                return
            self._handle_absolute_event(m)
        else:
            if not self._callback:
                return
            self._handle_relative_event(m)

    def _handle_absolute_event(self, m: RAWMOUSE):
        abs_x = m.lLastX
        abs_y = m.lLastY
        screen_w = _user32.GetSystemMetrics(SM_CXSCREEN)
        screen_h = _user32.GetSystemMetrics(SM_CYSCREEN)

        if screen_w > 0 and screen_h > 0:
            norm_x = abs_x / 65535.0
            norm_y = abs_y / 65535.0

        with self._abs_timeout_lock:
            self._last_abs_x = norm_x
            self._last_abs_y = norm_y
            was_down = self._is_pen_down
            self._is_pen_down = True
            self._last_abs_event_time = time.monotonic()

        if not was_down:
            if self._absolute_callback:
                try:
                    self._absolute_callback(norm_x, norm_y, True)
                except Exception:
                    logger.exception("raw_mouse: exception in absolute_callback (pen down)")
        else:
            if self._absolute_callback:
                try:
                    self._absolute_callback(norm_x, norm_y, True)
                except Exception:
                    logger.exception("raw_mouse: exception in absolute_callback")

    def _handle_relative_event(self, m: RAWMOUSE):
        dx, dy = m.lLastX, m.lLastY
        if dx == 0 and dy == 0:
            return
        if abs(dx) + abs(dy) < self._min_delta:
            self._filtered_count += 1
            if self._filtered_count % 500 == 1:
                logger.debug("raw_mouse: %d events filtered by min_delta=%d (latest dx=%d dy=%d)",
                             self._filtered_count, self._min_delta, dx, dy)
            return
        with self._lock:
            self._accum_dx += dx
            self._accum_dy += dy