import ctypes
import os
import threading
import time
from PIL import Image, ImageDraw
import pystray
from ghub import get_battery

POLL_INTERVAL = 30

try:
    ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("logibar")
except Exception:
    pass

def _color(pct):
    if pct is None:
        return (130, 130, 130, 255)
    if pct <= 20:
        return (230, 60, 60, 255)
    if pct <= 50:
        return (235, 185, 0, 255)
    return (55, 200, 70, 255)

def _darker(c, amount=70):
    return tuple(max(0, x - amount) for x in c[:3]) + (255,)

def make_mouse_icon(pct):
    SIZE = 64
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    color = _color(pct)
    dark = _darker(color)

    cx = SIZE // 2
    mw, mh = 38, 52
    x1, y1, x2, y2 = cx - mw//2, 6, cx + mw//2, 6 + mh
    d.rounded_rectangle([x1, y1, x2, y2], radius=mw//2, fill=color)
    d.line([cx, y1 + 4, cx, y1 + mh//2 - 2], fill=dark, width=2)
    d.rounded_rectangle([cx-3, y1 + 9, cx+3, y1 + 20], radius=3, fill=dark)
    return img

def make_headset_icon(pct):
    SIZE = 64
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    color = _color(pct)

    cx, cy = SIZE//2, 32
    r = 22
    d.arc([cx-r, cy-r, cx+r, cy+r], start=180, end=360, fill=color, width=7)
    ew, eh = 14, 20
    for ex in (cx - r, cx + r):
        d.rounded_rectangle([ex - ew//2, cy - 4, ex + ew//2, cy - 4 + eh], radius=5, fill=color)
    return img


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
        except Exception:
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
