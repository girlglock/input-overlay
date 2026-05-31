from __future__ import annotations

import sys

if sys.platform != 'win32':
    raise ImportError("input_rawinputbuffer is only supported on binbows")

import ctypes
import ctypes.wintypes as wt
import logging
import threading
import time
from typing import Callable

from services.consts import RAW_MOUSE_FLUSH_HZ

logger = logging.getLogger(__name__)

WM_INPUT          = 0x00FF
WM_QUIT           = 0x0012
RIM_TYPEMOUSE     = 0
RIM_TYPEKEYBOARD  = 1
RIDEV_INPUTSINK   = 0x00000100
RIDEV_REMOVE      = 0x00000001
WS_EX_TOOLWINDOW  = 0x00000080
WS_EX_NOACTIVATE  = 0x08000000
HWND_MESSAGE      = -3
PM_REMOVE         = 0x0001
PM_NOREMOVE       = 0x0000

THREAD_PRIORITY_BELOW_NORMAL = -1

RI_KEY_BREAK = 0x01
RI_KEY_E0    = 0x02
RI_KEY_E1    = 0x04

RI_MOUSE_LEFT_BUTTON_DOWN   = 0x0001
RI_MOUSE_LEFT_BUTTON_UP     = 0x0002
RI_MOUSE_RIGHT_BUTTON_DOWN  = 0x0004
RI_MOUSE_RIGHT_BUTTON_UP    = 0x0008
RI_MOUSE_MIDDLE_BUTTON_DOWN = 0x0010
RI_MOUSE_MIDDLE_BUTTON_UP   = 0x0020
RI_MOUSE_BUTTON_4_DOWN      = 0x0040
RI_MOUSE_BUTTON_4_UP        = 0x0080
RI_MOUSE_BUTTON_5_DOWN      = 0x0100
RI_MOUSE_BUTTON_5_UP        = 0x0200
RI_MOUSE_WHEEL              = 0x0400

MAPVK_VSC_TO_VK_EX = 3

_BTN_MAP = (
    (RI_MOUSE_LEFT_BUTTON_DOWN,   RI_MOUSE_LEFT_BUTTON_UP,   1),
    (RI_MOUSE_RIGHT_BUTTON_DOWN,  RI_MOUSE_RIGHT_BUTTON_UP,  2),
    (RI_MOUSE_MIDDLE_BUTTON_DOWN, RI_MOUSE_MIDDLE_BUTTON_UP, 3),
    (RI_MOUSE_BUTTON_4_DOWN,      RI_MOUSE_BUTTON_4_UP,      4),
    (RI_MOUSE_BUTTON_5_DOWN,      RI_MOUSE_BUTTON_5_UP,      5),
)


class RAWINPUTDEVICE(ctypes.Structure):
    _fields_ = [
        ("usUsagePage", wt.USHORT),
        ("usUsage",     wt.USHORT),
        ("dwFlags",     wt.DWORD),
        ("hwndTarget",  wt.HWND),
    ]


class RAWKEYBOARD(ctypes.Structure):
    _fields_ = [
        ("MakeCode",         wt.USHORT),
        ("Flags",            wt.USHORT),
        ("Reserved",         wt.USHORT),
        ("VKey",             wt.USHORT),
        ("Message",          wt.UINT),
        ("ExtraInformation", ctypes.c_ulong),
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
        _fields_ = [("mouse", RAWMOUSE), ("keyboard", RAWKEYBOARD)]

    _fields_ = [("header", RAWINPUTHEADER), ("data", _DATA)]


class _MSG(ctypes.Structure):
    _fields_ = [
        ("hwnd",    wt.HWND),   ("message", wt.UINT),
        ("wParam",  wt.WPARAM), ("lParam",  wt.LPARAM),
        ("time",    wt.DWORD),  ("pt",      wt.POINT),
    ]


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
    wt.DWORD, wt.LPCWSTR, wt.LPCWSTR, wt.DWORD,
    ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
    wt.HWND, wt.HANDLE, ctypes.c_void_p, ctypes.c_void_p,
]

_user32.DefWindowProcW.restype  = _LRESULT
_user32.DefWindowProcW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]

_user32.WaitMessage.restype  = wt.BOOL
_user32.WaitMessage.argtypes = []

_user32.PeekMessageW.restype = wt.BOOL

_user32.GetRawInputBuffer.restype  = wt.UINT
_user32.GetRawInputBuffer.argtypes = [
    ctypes.c_void_p,
    ctypes.POINTER(wt.UINT),
    wt.UINT,
]

_user32.RegisterRawInputDevices.restype  = wt.BOOL
_user32.RegisterRawInputDevices.argtypes = [ctypes.c_void_p, wt.UINT, wt.UINT]

_user32.LoadKeyboardLayoutW.restype  = ctypes.c_void_p
_user32.LoadKeyboardLayoutW.argtypes = [wt.LPCWSTR, wt.UINT]

_user32.MapVirtualKeyExW.restype  = wt.UINT
_user32.MapVirtualKeyExW.argtypes = [wt.UINT, wt.UINT, ctypes.c_void_p]

_US_LAYOUT = _user32.LoadKeyboardLayoutW("00000409", 0)


class RawInputBuffer(threading.Thread):
    FLUSH_HZ = RAW_MOUSE_FLUSH_HZ

    def __init__(
        self,
        on_key_press:    Callable[[int], None],
        on_key_release:  Callable[[int], None],
        on_mouse_click:  Callable[[int, bool], None] | None = None,
        on_mouse_scroll: Callable[[int], None] | None = None,
        on_mouse_move:   Callable[[int, int], None] | None = None,
        min_delta:       int = 0,
        daemon:          bool = True,
    ) -> None:
        super().__init__(daemon=daemon, name="RawInputBuffer")
        self._on_key_press    = on_key_press
        self._on_key_release  = on_key_release
        self._on_mouse_click  = on_mouse_click
        self._on_mouse_scroll = on_mouse_scroll
        self._on_mouse_move   = on_mouse_move
        self._min_delta       = min_delta
        self._hwnd: int | None = None
        self._lock     = threading.Lock()
        self._accum_dx = 0
        self._accum_dy = 0

    def stop(self) -> None:
        if self._hwnd:
            _user32.PostMessageW(self._hwnd, WM_QUIT, 0, 0)
        self.join(timeout=2.0)

    @staticmethod
    def _set_background_priority() -> None:
        h = _kernel32.GetCurrentThread()
        _kernel32.SetThreadPriority(h, THREAD_PRIORITY_BELOW_NORMAL)
        _kernel32.SetThreadPriorityBoost(h, True)

    def run(self) -> None:
        try:
            self._set_background_priority()
            self._hwnd = self._create_window()
            if not self._hwnd:
                logger.error("rawinputbuffer: CreateWindowEx failed (error %d)", _kernel32.GetLastError())
                return
            if not self._register():
                logger.error("rawinputbuffer: RegisterRawInputDevices failed (error %d)", _kernel32.GetLastError())
                _user32.DestroyWindow(self._hwnd)
                return
            logger.info("rawinputbuffer: listener started (hwnd=0x%x, min_delta=%d)", self._hwnd, self._min_delta)

            if self._on_mouse_move is not None:
                flush = threading.Thread(target=self._flush_loop, daemon=True, name="RawInputBufferFlush")
                flush.start()

            self._pump()
        except Exception:
            logger.exception("rawinputbuffer: unhandled error in run()")
        finally:
            self._unregister()
            if self._hwnd:
                _user32.DestroyWindow(self._hwnd)
                self._hwnd = None
            logger.info("rawinputbuffer: listener stopped")

    def _flush_loop(self) -> None:
        self._set_background_priority()
        interval = 1.0 / self.FLUSH_HZ
        while True:
            time.sleep(interval)
            with self._lock:
                dx, dy = self._accum_dx, self._accum_dy
                self._accum_dx = 0
                self._accum_dy = 0
            if dx == 0 and dy == 0:
                continue
            cb = self._on_mouse_move
            if not cb:
                continue
            try:
                cb(dx, dy)
            except Exception:
                logger.exception("rawinputbuffer: exception in mouse move callback")

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

        class_name = "IOvRawInputBuffer"
        wc = WNDCLASSEX()
        wc.cbSize        = ctypes.sizeof(WNDCLASSEX)
        wc.lpfnWndProc   = self._wnd_proc_ref
        wc.hInstance     = _kernel32.GetModuleHandleW(None)
        wc.lpszClassName = class_name

        atom = _user32.RegisterClassExW(ctypes.byref(wc))
        if atom == 0:
            logger.warning(
                "rawinputbuffer: RegisterClassExW returned 0 (error %d) - class may already exist",
                _kernel32.GetLastError(),
            )

        hwnd = _user32.CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name, None, 0,
            0, 0, 0, 0,
            HWND_MESSAGE, None, wc.hInstance, None,
        )
        return hwnd or None

    def _register(self) -> bool:
        devices = (RAWINPUTDEVICE * 2)()
        devices[0].usUsagePage = 0x01
        devices[0].usUsage     = 0x06
        devices[0].dwFlags     = RIDEV_INPUTSINK
        devices[0].hwndTarget  = self._hwnd
        devices[1].usUsagePage = 0x01
        devices[1].usUsage     = 0x02
        devices[1].dwFlags     = RIDEV_INPUTSINK
        devices[1].hwndTarget  = self._hwnd
        result = bool(
            _user32.RegisterRawInputDevices(
                ctypes.byref(devices[0]),
                2,
                ctypes.sizeof(RAWINPUTDEVICE),
            )
        )
        if result:
            logger.debug("rawinputbuffer: registered keyboard+mouse (INPUTSINK, hwnd=0x%x)", self._hwnd)
        else:
            logger.error("rawinputbuffer: RegisterRawInputDevices failed (error %d)", _kernel32.GetLastError())
        return result

    def _unregister(self) -> None:
        devices = (RAWINPUTDEVICE * 2)()
        for i, usage in enumerate((0x06, 0x02)):
            devices[i].usUsagePage = 0x01
            devices[i].usUsage     = usage
            devices[i].dwFlags     = RIDEV_REMOVE
            devices[i].hwndTarget  = None
        _user32.RegisterRawInputDevices(
            ctypes.byref(devices[0]),
            2,
            ctypes.sizeof(RAWINPUTDEVICE),
        )

    def _pump(self) -> None:
        interval = 1.0 / self.FLUSH_HZ
        msg = _MSG()
        while True:
            time.sleep(interval)
            self._drain_buffer()
            while _user32.PeekMessageW(ctypes.byref(msg), None, 0, WM_INPUT - 1, PM_REMOVE):
                if msg.message == WM_QUIT:
                    return
                _user32.TranslateMessage(ctypes.byref(msg))
                _user32.DispatchMessageW(ctypes.byref(msg))

    def _drain_buffer(self) -> None:
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

    def _handle_rawinput(self, ri: RAWINPUT) -> None:
        if ri.header.dwType == RIM_TYPEKEYBOARD:
            self._handle_keyboard(ri.data.keyboard)
        elif ri.header.dwType == RIM_TYPEMOUSE:
            self._handle_mouse(ri.data.mouse)

    def _handle_keyboard(self, kb: RAWKEYBOARD) -> None:
        if kb.VKey == 0xFF:
            return
        is_release = bool(kb.Flags & RI_KEY_BREAK)
        if kb.Flags & RI_KEY_E1:
            rawcode = kb.VKey
        else:
            extended_scan = kb.MakeCode | (0x100 if (kb.Flags & RI_KEY_E0) else 0)
            us_vk = _user32.MapVirtualKeyExW(extended_scan, MAPVK_VSC_TO_VK_EX, _US_LAYOUT)
            rawcode = us_vk if us_vk else kb.VKey
        if not rawcode:
            return
        if is_release:
            self._on_key_release(rawcode)
        else:
            self._on_key_press(rawcode)

    def _handle_mouse(self, m: RAWMOUSE) -> None:
        flags = m._u._s.usButtonFlags
        if flags:
            if self._on_mouse_click:
                for down_flag, up_flag, btn_code in _BTN_MAP:
                    if flags & down_flag:
                        self._on_mouse_click(btn_code, True)
                    if flags & up_flag:
                        self._on_mouse_click(btn_code, False)
            if flags & RI_MOUSE_WHEEL and self._on_mouse_scroll:
                delta = ctypes.c_short(m._u._s.usButtonData).value
                rotation = -1 if delta > 0 else 1 if delta < 0 else 0
                if rotation:
                    self._on_mouse_scroll(rotation)

        if m.usFlags & 0x0001:
            return
        dx, dy = m.lLastX, m.lLastY
        if dx == 0 and dy == 0:
            return
        if self._on_mouse_move is None:
            return
        if self._min_delta and abs(dx) + abs(dy) < self._min_delta:
            return
        with self._lock:
            self._accum_dx += dx
            self._accum_dy += dy
