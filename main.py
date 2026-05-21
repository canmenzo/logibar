import ctypes
import os
import sys
import threading
import time
from PIL import Image, ImageDraw, ImageFont
import pystray
from ghub import get_battery

POLL_INTERVAL = 30

# Show "logibar" in Windows app settings instead of "Python"
try:
    ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("logibar")
except Exception:
    pass

_FONT_PATHS = [
    r"C:\Windows\Fonts\arialbd.ttf",
    r"C:\Windows\Fonts\arial.ttf",
    r"C:\Windows\Fonts\calibrib.ttf",
    r"C:\Windows\Fonts\segoeui.ttf",
]


def _font(size):
    for path in _FONT_PATHS:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                pass
    return ImageFont.load_default()


def make_icon(pct, label):
    SIZE = 64
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Dark background
    draw.rounded_rectangle([0, 0, SIZE - 1, SIZE - 1], radius=10, fill=(22, 22, 22, 245))

    # Label at top
    draw.text((SIZE // 2, 13), label, fill=(110, 110, 110, 255), anchor="mm", font=_font(10))

    # Big percentage number
    text = f"{pct}%" if pct is not None else "--"
    draw.text((SIZE // 2, 36), text, fill=(235, 235, 235, 255), anchor="mm", font=_font(22))

    # Progress bar at bottom — left to right fill
    BX1, BY1, BX2, BY2 = 6, 51, 58, 58
    draw.rounded_rectangle([BX1, BY1, BX2, BY2], radius=3, fill=(50, 50, 50, 255))
    if pct is not None and pct > 0:
        fw = max(4, int((BX2 - BX1) * pct / 100))
        draw.rounded_rectangle([BX1, BY1, BX1 + fw, BY2], radius=3, fill=(190, 190, 190, 255))

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
            self.mouse_icon.icon = make_icon(self.mouse, "MOUSE")
            self.mouse_icon.title = f"Mouse: {self.mouse}%" if self.mouse is not None else "Mouse: N/A"
        if self.headset_icon:
            self.headset_icon.icon = make_icon(self.headset, "HEADSET")
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
            make_icon(self.mouse, "MOUSE"),
            f"Mouse: {self.mouse}%" if self.mouse is not None else "Mouse: N/A",
            menu=menu,
        )
        self.headset_icon = pystray.Icon(
            "logibar-headset",
            make_icon(self.headset, "HEADSET"),
            f"Headset: {self.headset}%" if self.headset is not None else "Headset: N/A",
            menu=menu,
        )

        self.mouse_icon.run_detached()
        self.headset_icon.run()


if __name__ == "__main__":
    App().run()
