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

WM_INPUT        = 0x00FF
WM_QUIT         = 0x0012
RIM_TYPEMOUSE   = 0
RIDEV_INPUTSINK = 0x00000100
RIDEV_REMOVE    = 0x00000001
RID_INPUT       = 0x10000003
WS_EX_TOOLWINDOW = 0x00000080
WS_EX_NOACTIVATE = 0x08000000
HWND_MESSAGE    = -3
THREAD_PRIORITY_IDLE          = -15
THREAD_PRIORITY_LOWEST        = -2
THREAD_PRIORITY_BELOW_NORMAL  = -1
THREAD_PRIORITY_NORMAL        =  0
THREAD_PRIORITY_ABOVE_NORMAL  =  1
THREAD_PRIORITY_HIGHEST       =  2
THREAD_PRIORITY_TIME_CRITICAL =  15
PM_REMOVE       = 0x0001
PM_NOREMOVE     = 0x0000


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
        ("hDevice", ctypes.c_uint64),
        ("wParam",  ctypes.c_uint64),
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

_kernel32.GetCurrentThread.restype  = wt.HANDLE
_kernel32.GetCurrentThread.argtypes = []

_kernel32.SetThreadPriority.restype  = wt.BOOL
_kernel32.SetThreadPriority.argtypes = [wt.HANDLE, ctypes.c_int]

_kernel32.SetThreadPriorityBoost.restype  = wt.BOOL
_kernel32.SetThreadPriorityBoost.argtypes = [wt.HANDLE, wt.BOOL]

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

_user32.WaitMessage.restype  = wt.BOOL
_user32.WaitMessage.argtypes = []

_user32.GetRawInputBuffer.restype  = wt.UINT
_user32.GetRawInputBuffer.argtypes = [
    ctypes.c_void_p,
    ctypes.POINTER(wt.UINT),
    wt.UINT,
]


class RawMouseThread(threading.Thread):
    FLUSH_HZ = RAW_MOUSE_FLUSH_HZ

    def __init__(self, callback: Callable[[int, int], None], min_delta: int = 0, daemon: bool = True):
        super().__init__(daemon=daemon, name="RawMouseThread")
        self._callback = callback
        self._min_delta = min_delta
        self._hwnd: int | None = None
        self._lock = threading.Lock()
        self._accum_dx = 0
        self._accum_dy = 0
        self._filtered_count = 0

    def stop(self):
        if self._hwnd:
            _user32.PostMessageW(self._hwnd, WM_QUIT, 0, 0)
        self.join(timeout=2.0)

    @staticmethod
    def _set_background_priority():
        h = _kernel32.GetCurrentThread()
        _kernel32.SetThreadPriority(h, THREAD_PRIORITY_BELOW_NORMAL)
        _kernel32.SetThreadPriorityBoost(h, True)

    def run(self):
        try:
            self._set_background_priority()
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
            self._unregister()
            if self._hwnd:
                _user32.DestroyWindow(self._hwnd)
                self._hwnd = None
            logger.info("raw_mouse: listener stopped")

    def _flush_loop(self):
        self._set_background_priority()
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
                self._callback(dx, dy)
            except Exception:
                logger.exception("raw_mouse: exception in flush callback")

    def _create_window(self) -> int | None:
        def _wnd_proc(hwnd, msg, wParam, lParam):
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
            HWND_MESSAGE, None, wc.hInstance, None,
        )
        if hwnd:
            logger.debug("raw_mouse: message window created successfully")
        else:
            logger.error("raw_mouse: CreateWindowExW returned NULL (error %d)", _kernel32.GetLastError())
        return hwnd or None

    def _register(self) -> bool:
        rid = RAWINPUTDEVICE()
        rid.usUsagePage = 0x01
        rid.usUsage     = 0x02
        rid.dwFlags     = RIDEV_INPUTSINK
        rid.hwndTarget  = self._hwnd
        result = bool(_user32.RegisterRawInputDevices(ctypes.byref(rid), 1, ctypes.sizeof(RAWINPUTDEVICE)))
        if result:
            logger.debug("raw_mouse: RegisterRawInputDevices succeeded (INPUTSINK on hwnd=0x%x)", self._hwnd)
        else:
            logger.error("raw_mouse: RegisterRawInputDevices failed (error %d)", _kernel32.GetLastError())
        return result

    def _unregister(self):
        rid = RAWINPUTDEVICE()
        rid.usUsagePage = 0x01
        rid.usUsage     = 0x02
        rid.dwFlags     = RIDEV_REMOVE
        rid.hwndTarget  = None
        result = bool(_user32.RegisterRawInputDevices(ctypes.byref(rid), 1, ctypes.sizeof(RAWINPUTDEVICE)))
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

        _user32.PeekMessageW.restype  = wt.BOOL
        _user32.PeekMessageW.argtypes = [
            ctypes.POINTER(MSG), wt.HWND, wt.UINT, wt.UINT, wt.UINT,
        ]

        logger.debug("raw_mouse: entering message pump")
        msg = MSG()
        msg_count = 0
        while True:
            while _user32.PeekMessageW(ctypes.byref(msg), None, WM_INPUT, WM_INPUT, PM_NOREMOVE):
                self._drain_raw_input_buffer()

            while _user32.PeekMessageW(ctypes.byref(msg), None, 0, WM_INPUT - 1, PM_REMOVE):
                msg_count += 1
                if msg_count <= 5:
                    logger.debug("raw_mouse: pump received message #%d (msg=0x%x)", msg_count, msg.message)
                if msg.message == WM_QUIT:
                    logger.debug("raw_mouse: message pump exited (total messages processed: %d)", msg_count)
                    return
                _user32.TranslateMessage(ctypes.byref(msg))
                _user32.DispatchMessageW(ctypes.byref(msg))

            _user32.WaitMessage()

    def _drain_raw_input_buffer(self):
        while True:
            buf_size = wt.UINT(0)
            if _user32.GetRawInputBuffer(None, ctypes.byref(buf_size), ctypes.sizeof(RAWINPUTHEADER)) != 0:
                break
            if buf_size.value == 0:
                break

            alloc = buf_size.value * 8
            buf = ctypes.create_string_buffer(alloc)
            buf_size.value = alloc
            count = _user32.GetRawInputBuffer(buf, ctypes.byref(buf_size), ctypes.sizeof(RAWINPUTHEADER))
            if count == 0 or count == 0xFFFFFFFF:
                break
            raw = bytes(buf)
            offset = 0
            for _ in range(count):
                ri = RAWINPUT.from_buffer_copy(raw[offset:])
                self._handle_rawinput(ri)
                offset = (offset + ri.header.dwSize + 7) & ~7

    def _handle_rawinput(self, ri: RAWINPUT):
        if ri.header.dwType != RIM_TYPEMOUSE:
            logger.debug("raw_mouse: ignoring non-mouse rawinput (dwType=%d)", ri.header.dwType)
            return
        m = ri.data.mouse
        if m.usFlags & 0x0001:
            logger.debug("raw_mouse: ignoring absolute mouse event (usFlags=0x%x)", m.usFlags)
            return
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