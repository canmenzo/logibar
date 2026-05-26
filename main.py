import atexit
import ctypes
import os
import sys
import threading
import time
from PIL import Image
import pystray
from ghub import get_battery, cleanup_temp

POLL_INTERVAL = 30
APP_ID = "menzo.logibar"

try:
    ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(APP_ID)
except (AttributeError, OSError):
    pass

atexit.register(cleanup_temp)


def _resource(rel):
    base = getattr(sys, "_MEIPASS", os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base, rel)


def _color(pct):
    if pct is None: return (130, 130, 130)
    if pct <= 20:   return (230, 60, 60)
    if pct <= 50:   return (235, 185, 0)
    return (55, 200, 70)


MOUSE_SRC = Image.open(_resource("assets/mouseicon.png")).convert("RGBA")
HEADSET_SRC = Image.open(_resource("assets/headseticon.png")).convert("RGBA")


def _tint(src, color):
    # Preserve alpha (silhouette + anti-aliased edges), replace RGB with target color.
    alpha = src.split()[3]
    solid = Image.new("RGBA", src.size, color + (255,))
    solid.putalpha(alpha)
    return solid


def make_mouse_icon(pct):
    return _tint(MOUSE_SRC, _color(pct))


def make_headset_icon(pct):
    return _tint(HEADSET_SRC, _color(pct))


class App:
    def __init__(self):
        self.mouse = None
        self.headset = None
        self.mouse_icon = None
        self.headset_icon = None

    def refresh(self, _=None):
        try:
            data = get_battery()
            self.mouse = data["mouse"]
            self.headset = data["headset"]
        except (OSError, ValueError, KeyError):
            pass
        if self.mouse_icon:
            self.mouse_icon.icon = make_mouse_icon(self.mouse)
            self.mouse_icon.title = f"Mouse: {self.mouse}%" if self.mouse is not None else "Mouse: N/A"
        if self.headset_icon:
            self.headset_icon.icon = make_headset_icon(self.headset)
            self.headset_icon.title = f"Headset: {self.headset}%" if self.headset is not None else "Headset: N/A"

    def _poll(self):
        while True:
            time.sleep(POLL_INTERVAL)
            self.refresh()

    def _quit(self, _=None):
        if self.mouse_icon:
            self.mouse_icon.stop()
        if self.headset_icon:
            self.headset_icon.stop()

    def run(self):
        self.refresh()
        threading.Thread(target=self._poll, daemon=True).start()

        menu = pystray.Menu(
            pystray.MenuItem("Refresh", self.refresh),
            pystray.MenuItem("Quit", self._quit),
        )

        self.mouse_icon = pystray.Icon(
            "logibar-mouse",
            make_mouse_icon(self.mouse),
            f"Mouse: {self.mouse}%" if self.mouse is not None else "Mouse: N/A",
            menu=menu,
        )
        self.headset_icon = pystray.Icon(
            "logibar-headset",
            make_headset_icon(self.headset),
            f"Headset: {self.headset}%" if self.headset is not None else "Headset: N/A",
            menu=menu,
        )

        self.mouse_icon.run_detached()
        self.headset_icon.run()


if __name__ == "__main__":
    App().run()
