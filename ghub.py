import json
import os
import shutil
import sqlite3
from datetime import datetime, timezone
from difflib import SequenceMatcher

DB_SRC = os.path.join(os.environ.get("LOCALAPPDATA", ""), "LGHUB", "settings.db")
TMP_BASE = os.path.join(os.environ.get("TEMP", "."), "logibar_tmp.db")
KEYWORDS = {"mouse": ("mouse",), "headset": ("headset", "headphone"), "keyboard": ("keyboard",)}
EPOCH = datetime(1970, 1, 1, tzinfo=timezone.utc)


def read_devices():
    """Newest battery reading per device kind from G HUB's settings.db:
    {"mouse": {"name": str, "pct": int, "time": datetime|None}, ...}.
    Returns None if the DB couldn't be read this time (G HUB mid-write)."""
    if not os.path.exists(DB_SRC):
        return {}
    try:
        # G HUB keeps the DB open, so read a snapshot. Clear old copies first: a stale
        # -wal next to a fresh .db would be replayed into it.
        cleanup_temp()
        for ext in ("", "-wal"):
            if os.path.exists(DB_SRC + ext):
                shutil.copyfile(DB_SRC + ext, TMP_BASE + ext)
        conn = sqlite3.connect(TMP_BASE)
        try:
            row = conn.execute("SELECT file FROM data ORDER BY _id DESC LIMIT 1").fetchone()
        finally:
            conn.close()
        return _parse(json.loads(row[0])) if row else {}
    except (OSError, sqlite3.Error, ValueError):
        return None


def cleanup_temp():
    for ext in ("", "-wal", "-shm", "-journal"):
        try:
            os.remove(TMP_BASE + ext)
        except OSError:
            pass


def _parse(data):
    # Battery keys are flat: "battery/<slug>/percentage" -> {"percentage": int, "time": iso8601}
    known = [((d.get("type") or "").lower(), d.get("baseModelId") or d.get("modelId") or "")
             for d in (data.get("devices/known") or {}).get("knownList", [])]
    readings = {}
    for key, val in data.items():
        parts = key.split("/")
        if len(parts) != 3 or parts[0] != "battery" or parts[2] != "percentage" or not isinstance(val, dict):
            continue
        pct = val.get("percentage")
        if not isinstance(pct, (int, float)):
            continue
        kind, name = _identify(parts[1].lower(), known)
        if kind:
            readings.setdefault(kind, []).append(
                {"name": name, "pct": max(0, min(100, int(pct))), "time": _time(val.get("time"))})
    return {k: max(rs, key=lambda r: r["time"] or EPOCH) for k, rs in readings.items()}


def _identify(slug, known):
    """Map a battery slug ("prox2superstrikewirelessmouse") to (kind, display name).
    Slugs don't always say "mouse" (e.g. G502 X), so fall back to the closest known device."""
    kind = next((k for k, words in KEYWORDS.items() if any(w in slug for w in words)), None)
    best, match_kind, model = 0.6, None, None
    for ktype, model_id in known:
        if kind and ktype != kind:
            continue
        ratio = SequenceMatcher(None, slug, model_id.replace("_", "")).ratio()
        if ratio > best:
            best, match_kind, model = ratio, ktype, model_id
    kind = kind or match_kind
    if kind not in KEYWORDS:
        return None, None
    if not model:
        return kind, kind.title()
    # "pro_x_2_superstrike_wireless_mouse" -> "PRO X 2 Superstrike Wireless Mouse", "g915_tkl" -> "G915 TKL"
    return kind, " ".join(w.upper() if len(w) <= 3 or any(c.isdigit() for c in w) else w.title()
                          for w in model.split("_"))


def _time(value):
    try:
        return datetime.fromisoformat(str(value).replace("Z", "+00:00")).astimezone(timezone.utc)
    except ValueError:
        return None
