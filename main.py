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

    draw.rounded_rectangle([0, 0, SIZE - 1, SIZE - 1], radius=10, fill=(28, 28, 28, 240))

    # Small label at top
    draw.text((SIZE // 2, 11), label, fill=(150, 150, 150, 255), anchor="mm", font=_font(10))

    # Battery body
    BX1, BY1, BX2, BY2 = 3, 18, 54, 46
    NX1, NY1, NX2, NY2 = 54, 26, 61, 38
    BORDER = (210, 210, 210, 255)

    draw.rectangle([BX1, BY1, BX2, BY2], outline=BORDER, width=2)
    draw.rectangle([NX1, NY1, NX2, NY2], fill=BORDER)

    # Fill bar — left to right
    if pct is not None and pct > 0:
        inner_w = BX2 - BX1 - 4
        fill_w = max(1, int(inner_w * pct / 100))
        draw.rectangle([BX1 + 2, BY1 + 2, BX1 + 2 + fill_w, BY2 - 2], fill=(195, 195, 195, 240))

    # Percent number centered in battery
    text = f"{pct}%" if pct is not None else "?"
    cx = (BX1 + BX2) // 2
    cy = (BY1 + BY2) // 2
    draw.text((cx, cy), text, fill=(255, 255, 255, 255), anchor="mm", font=_font(14))

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
