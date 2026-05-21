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

WHITE = (215, 215, 215, 255)
BG    = (22, 22, 22, 245)
TRACK = (50, 50, 50, 255)
BAR   = (190, 190, 190, 255)

def _progress(draw, pct):
    x1, y1, x2, y2 = 6, 53, 58, 59
    draw.rounded_rectangle([x1, y1, x2, y2], radius=3, fill=TRACK)
    if pct and pct > 0:
        fw = max(4, int((x2 - x1) * pct / 100))
        draw.rounded_rectangle([x1, y1, x1 + fw, y2], radius=3, fill=BAR)

def make_mouse_icon(pct):
    SIZE = 64
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, SIZE-1, SIZE-1], radius=10, fill=BG)

    cx = SIZE // 2
    mw, mh = 18, 24
    x1, y1, x2, y2 = cx-mw//2, 5, cx+mw//2, 5+mh
    d.rounded_rectangle([x1, y1, x2, y2], radius=mw//2, outline=WHITE, width=2)
    d.line([cx, y1+2, cx, y1+mh//2-1], fill=WHITE, width=1)
    d.rounded_rectangle([cx-2, y1+5, cx+2, y1+11], radius=2, fill=WHITE)

    d.text((cx, 42), f"{pct}%" if pct is not None else "--", fill=(235, 235, 235), anchor="mm", font=_font(18))
    _progress(d, pct)
    return img

def make_headset_icon(pct):
    SIZE = 64
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, SIZE-1, SIZE-1], radius=10, fill=BG)

    cx = SIZE // 2
    r, cy = 15, 22
    d.arc([cx-r, cy-r, cx+r, cy+r], start=180, end=360, fill=WHITE, width=2)

    ew, eh = 8, 12
    for ex in (cx - r, cx + r):
        d.ellipse([ex-ew//2, cy-eh//2, ex+ew//2, cy+eh//2], outline=WHITE, width=2)

    d.text((cx, 42), f"{pct}%" if pct is not None else "--", fill=(235, 235, 235), anchor="mm", font=_font(18))
    _progress(d, pct)
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
