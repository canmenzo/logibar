import atexit
import ctypes
import os
import sys
import threading
import winreg
from datetime import datetime, timezone

import pystray
from pystray._util import serialized_image, win32

from ghub import cleanup_temp, read_devices
from icons import CRITICAL, GLYPHS, LOW, tray_icon

APP = "logibar"
APP_ID = "menzo.logibar"
POLL_SECONDS = 30
FROZEN = getattr(sys, "frozen", False)
HKCU = winreg.HKEY_CURRENT_USER
RUN_KEY = r"Software\Microsoft\Windows\CurrentVersion\Run"
THEME_KEY = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"
SM_CXSMICON = 49
WM_SETTINGCHANGE = 0x001A


def _launch_cmd():
    if FROZEN:
        return f'"{sys.executable}"'
    pythonw = os.path.join(os.path.dirname(sys.executable), "pythonw.exe")
    return f'"{pythonw}" "{os.path.abspath(__file__)}"'


def startup_enabled(_=None):
    try:
        with winreg.OpenKey(HKCU, RUN_KEY) as k:
            return winreg.QueryValueEx(k, APP)[0] == _launch_cmd()
    except OSError:
        return False


def toggle_startup():
    with winreg.CreateKeyEx(HKCU, RUN_KEY, 0, winreg.KEY_SET_VALUE) as k:
        if startup_enabled():
            winreg.DeleteValue(k, APP)
        else:
            winreg.SetValueEx(k, APP, 0, winreg.REG_SZ, _launch_cmd())


def taskbar_is_dark():
    try:
        with winreg.OpenKey(HKCU, THEME_KEY) as k:
            return winreg.QueryValueEx(k, "SystemUsesLightTheme")[0] == 0
    except OSError:
        return True


def match_menu_theme(dark):
    uxtheme = ctypes.windll.uxtheme
    uxtheme[135](2 if dark else 3)  # SetPreferredAppMode: ForceDark / ForceLight
    uxtheme[136]()  # FlushMenuThemes


def register_app_id():
    # Gives notifications and Settings > Other system tray icons a proper name and icon.
    ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(APP_ID)
    with winreg.CreateKeyEx(HKCU, rf"Software\Classes\AppUserModelId\{APP_ID}", 0, winreg.KEY_SET_VALUE) as k:
        winreg.SetValueEx(k, "DisplayName", 0, winreg.REG_SZ, APP)
        winreg.SetValueEx(k, "IconResource", 0, winreg.REG_SZ, f"{sys.executable},0")
        winreg.SetValueEx(k, "IconUri", 0, winreg.REG_SZ, f"{sys.executable},0")


def _ago(t):
    mins = int((datetime.now(timezone.utc) - t).total_seconds() // 60)
    if mins < 60:
        return None
    if mins < 60 * 24:
        return f"{mins // 60} h ago"
    return f"{mins // (60 * 24)} d ago"


class TrayIcon(pystray.Icon):
    on_settings_change = None

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._message_handlers[WM_SETTINGCHANGE] = lambda w, l: self.on_settings_change and self.on_settings_change()

    def _assert_icon_handle(self):
        # pystray loads at the large-icon size and lets the shell rescale it (blurry);
        # load the frame we rendered at the exact tray size instead.
        if not self._icon_handle:
            with serialized_image(self.icon, "ICO") as path:
                self._icon_handle = win32.LoadImage(
                    None, path, win32.IMAGE_ICON, self.icon.width, self.icon.height, win32.LR_LOADFROMFILE)

    def _on_notify(self, wparam, lparam):
        # Any click opens the menu, rebuilt on open so it's current and never swapped out while shown.
        if lparam in (win32.WM_LBUTTONUP, win32.WM_RBUTTONUP):
            self.update_menu()
            lparam = win32.WM_RBUTTONUP
        super()._on_notify(wparam, lparam)


class App:
    def __init__(self):
        self.devices = read_devices() or {}
        self.warned = set()
        self.lock = threading.RLock()
        self.stopped = threading.Event()
        menu = pystray.Menu(self._menu_items)
        self.icons = {kind: TrayIcon(f"{APP}-{kind}", menu=menu) for kind in GLYPHS}

    def _menu_items(self):
        for kind in GLYPHS:
            d = self.devices.get(kind)
            if d:
                yield pystray.MenuItem(f"{d['name']}\t{d['pct']}%", None)
        if not self.devices:
            yield pystray.MenuItem("No devices found", None, enabled=False)
        yield pystray.Menu.SEPARATOR
        yield pystray.MenuItem("Refresh", self.refresh)
        yield pystray.MenuItem("Start with Windows", toggle_startup, checked=startup_enabled)
        yield pystray.Menu.SEPARATOR
        yield pystray.MenuItem("Quit", self.quit)

    def _tooltip(self, d):
        if not d:
            return "logibar\nNo devices found. Is G HUB running?"
        ago = d["time"] and _ago(d["time"])
        return f"{d['name']}\n{d['pct']}%" + (f" (last seen {ago})" if ago else "")

    def _render(self, kind, dark, size):
        with self.lock:
            icon, d = self.icons[kind], self.devices.get(kind)
            icon.icon = tray_icon(kind, d and d["pct"], size, dark)
            icon.title = self._tooltip(d)
            # With nothing detected, keep one icon up so the app can still be reached.
            icon.visible = bool(d) or (kind == "mouse" and not self.devices)

    def redraw(self, kinds=GLYPHS):
        dark = taskbar_is_dark()
        match_menu_theme(dark)
        size = ctypes.windll.user32.GetSystemMetrics(SM_CXSMICON)
        for kind in kinds:
            self._render(kind, dark, size)

    def refresh(self):
        with self.lock:
            devices = read_devices()
            if devices is not None:
                self.devices = devices
            self.redraw()
            for kind, d in self.devices.items():
                if d["pct"] <= CRITICAL and kind not in self.warned:
                    self.warned.add(kind)
                    self.icons[kind].notify(f"{d['name']} is at {d['pct']}%. Time to charge.", "Low battery")
                elif d["pct"] > LOW:
                    self.warned.discard(kind)

    def _poll(self):
        while not self.stopped.wait(POLL_SECONDS):
            self.refresh()

    def quit(self):
        self.stopped.set()
        for icon in self.icons.values():
            icon.stop()

    def run(self):
        # Each icon draws itself once its own window exists; the poll thread takes over after that.
        first, *rest = GLYPHS
        self.icons[first].on_settings_change = lambda: threading.Thread(target=self.redraw).start()
        for kind in rest:
            self.icons[kind].run_detached(setup=lambda _, k=kind: self.redraw([k]))
        threading.Thread(target=self._poll, daemon=True).start()
        self.icons[first].run(setup=lambda _: self.redraw([first]))


def main():
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.CreateMutexW(None, False, f"Local\\{APP_ID}")
    if ctypes.get_last_error() == 183:  # ERROR_ALREADY_EXISTS: already running
        return
    ctypes.windll.user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))  # per-monitor v2: crisp icons + menus
    if FROZEN:
        register_app_id()
    atexit.register(cleanup_temp)
    App().run()


if __name__ == "__main__":
    main()
