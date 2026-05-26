"""Convert assets/png-transparent-logitech-logo-icon.png -> assets/app.ico (multi-res)."""
import os
from PIL import Image

SRC = os.path.join(os.path.dirname(__file__), "assets", "png-transparent-logitech-logo-icon.png")
DST = os.path.join(os.path.dirname(__file__), "assets", "app.ico")
SIZES = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]


def build():
    src = Image.open(SRC).convert("RGBA")
    # Pad to square if needed (some PNGs come non-square)
    w, h = src.size
    side = max(w, h)
    sq = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    sq.paste(src, ((side - w) // 2, (side - h) // 2))
    sq.save(DST, format="ICO", sizes=SIZES)
    print(f"wrote {DST}")


if __name__ == "__main__":
    build()
