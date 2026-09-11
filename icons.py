from PIL import Image, ImageDraw

SS = 8  # supersampling factor; masks are drawn big and box-reduced for clean anti-aliasing
ACCENT = (0, 190, 255)
AMBER = (255, 185, 0)
RED = (255, 72, 72)
LOW, CRITICAL = 30, 15


def _mouse(d, x, y, s):
    w = s * 0.64
    l = x + (s - w) / 2
    d.rounded_rectangle((l, y, l + w, y + s), radius=w / 2, fill=255)
    ww = s * 0.14
    cx = x + s / 2
    d.rounded_rectangle((cx - ww / 2, y + s * 0.15, cx + ww / 2, y + s * 0.42), radius=ww / 2, fill=0)


def _headset(d, x, y, s):
    t = s * 0.12
    d.arc((x + s * 0.1, y, x + s * 0.9, y + s * 0.8), 180, 360, fill=255, width=round(t))
    d.rectangle((x + s * 0.1, y + s * 0.4, x + s * 0.1 + t, y + s * 0.6), fill=255)
    d.rectangle((x + s * 0.9 - t, y + s * 0.4, x + s * 0.9, y + s * 0.6), fill=255)
    for l in (x + s * 0.02, x + s * 0.7):
        d.rounded_rectangle((l, y + s * 0.5, l + s * 0.28, y + s), radius=s * 0.1, fill=255)


def _keyboard(d, x, y, s):
    top, bot = y + s * 0.18, y + s * 0.82
    d.rounded_rectangle((x, top, x + s, bot), radius=s * 0.14, fill=255)
    k = s * 0.13
    for col in range(3):
        kx = x + s * (0.2 + col * 0.235)
        d.rectangle((kx, top + s * 0.13, kx + k, top + s * 0.13 + k), fill=0)
    d.rounded_rectangle((x + s * 0.2, bot - s * 0.26, x + s * 0.8, bot - s * 0.13), radius=s * 0.05, fill=0)


GLYPHS = {"mouse": _mouse, "headset": _headset, "keyboard": _keyboard}


def level_color(pct, normal):
    if pct <= CRITICAL:
        return RED
    if pct <= LOW:
        return AMBER
    return normal


def _layer(mask, color, alpha=255):
    mask = mask.reduce(SS)
    if alpha < 255:
        mask = mask.point(lambda v: v * alpha // 255)
    im = Image.new("RGBA", mask.size, color + (0,))
    im.putalpha(mask)
    return im


def _compose(size, kind, pct, fg, fill, glyph_alpha=255):
    """Device glyph on top, battery level bar underneath. Bar geometry is pixel-snapped per size."""
    S = size * SS
    bar_h = max(2, round(size / 8))
    gap = max(1, round(size / 16))
    pad = max(1, round(size / 16))
    gs = (size - bar_h - gap) * SS

    glyph = Image.new("L", (S, S))
    GLYPHS[kind](ImageDraw.Draw(glyph), (S - gs) / 2, 0, gs)

    top, h = (size - bar_h) * SS, bar_h * SS
    track = Image.new("L", (S, S))
    ImageDraw.Draw(track).rounded_rectangle((pad * SS, top, S - pad * SS, S), radius=h / 2, fill=255)

    out = Image.alpha_composite(_layer(glyph, fg, glyph_alpha), _layer(track, fg, 70))
    if pct is not None and pct > 0:
        tw = size - 2 * pad
        fw = max(bar_h, round(tw * pct / 100)) * SS
        bar = Image.new("L", (S, S))
        ImageDraw.Draw(bar).rounded_rectangle((pad * SS, top, pad * SS + fw, S), radius=h / 2, fill=255)
        out = Image.alpha_composite(out, _layer(bar, fill))
    return out


def tray_icon(kind, pct, size, dark):
    fg = (255, 255, 255) if dark else (32, 32, 32)
    if pct is None:
        return _compose(size, kind, None, fg, fg, glyph_alpha=110)
    return _compose(size, kind, pct, fg, level_color(pct, fg))


def app_icon(size):
    S = size * SS
    box, radius = (0, 0, S - 1, S - 1), S * 0.24
    tile, edge = Image.new("L", (S, S)), Image.new("L", (S, S))
    ImageDraw.Draw(tile).rounded_rectangle(box, radius=radius, fill=255)
    ImageDraw.Draw(edge).rounded_rectangle(box, radius=radius, outline=255, width=max(SS, S // 96))
    # Top-lit dark tile, with a faint rim so it stays visible on dark backgrounds
    grad = Image.linear_gradient("L").resize((size, size)).point(lambda v: 16 - v * 16 // 255)
    bg = Image.merge("RGB", [grad.point(lambda v, b=b: b + v) for b in (20, 22, 28)])
    bg.putalpha(tile.reduce(SS))
    bg.alpha_composite(_layer(edge, (255, 255, 255), 36))

    inner = round(size * 0.6)
    mark = _compose(inner, "mouse", 70, (255, 255, 255), ACCENT)
    off = (size - inner) // 2
    bg.alpha_composite(mark, (off, off))
    return bg
