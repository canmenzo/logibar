import ctypes
import os
import threading
import time
from PIL import Image, ImageDraw, ImageFont
import pystray
from ghub import get_battery

POLL_INTERVAL = 30

try:
    ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("logibar")
except Exception:
    pass

_FONTS = [r"C:\Windows\Fonts\arialbd.ttf", r"C:\Windows\Fonts\arial.ttf"]

def _font(size):
    for p in _FONTS:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                pass
    return ImageFont.load_default()

WHITE = (240, 240, 240, 255)

def make_mouse_icon(pct):
    W, H = 96, 32
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    # Mouse silhouette on left
    mx, my = 14, 16
    mw, mh = 16, 24
    x1, y1, x2, y2 = mx-mw//2, my-mh//2, mx+mw//2, my+mh//2
    d.rounded_rectangle([x1, y1, x2, y2], radius=mw//2, outline=WHITE, width=2)
    d.line([mx, y1+2, mx, y1+mh//2], fill=WHITE, width=2)
    d.rounded_rectangle([mx-2, y1+5, mx+2, y1+10], radius=2, fill=WHITE)
    # Percentage on right
    d.text((58, 16), f"{pct}%" if pct is not None else "--", fill=WHITE, anchor="mm", font=_font(20))
    return img

def make_headset_icon(pct):
    W, H = 96, 32
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    # Headset silhouette on left
    cx, cy = 16, 16
    r = 11
    d.arc([cx-r, cy-r, cx+r, cy+r], start=180, end=360, fill=WHITE, width=2)
    ew, eh = 7, 11
    for ex in (cx-r, cx+r):
        d.ellipse([ex-ew//2, cy-eh//2, ex+ew//2, cy+eh//2], outline=WHITE, width=2)
    # Percentage on right
    d.text((60, 16), f"{pct}%" if pct is not None else "--", fill=WHITE, anchor="mm", font=_font(20))
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
