"""Render assets/app.ico (multi-res, each size drawn natively) from icons.app_icon."""
import os
from icons import app_icon

DST = os.path.join(os.path.dirname(os.path.abspath(__file__)), "assets", "app.ico")
SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256]


def build():
    frames = [app_icon(s) for s in SIZES]
    frames[-1].save(DST, format="ICO", sizes=[(s, s) for s in SIZES], append_images=frames[:-1])
    print(f"wrote {DST}")


if __name__ == "__main__":
    build()
