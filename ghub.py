import json
import os
import shutil
import sqlite3

DB_SRC = os.path.join(os.environ.get("LOCALAPPDATA", ""), "LGHUB", "settings.db")
TMP_BASE = os.path.join(os.environ.get("TEMP", "."), "logibar_tmp.db")


def get_battery():
    """Returns {"mouse": int|None, "headset": int|None} from G HUB settings.db."""
    if not os.path.exists(DB_SRC):
        return {"mouse": None, "headset": None}

    for ext in ("", "-shm", "-wal"):
        src = DB_SRC + ext
        if os.path.exists(src):
            shutil.copy2(src, TMP_BASE + ext)

    conn = sqlite3.connect(TMP_BASE)
    try:
        row = conn.execute("SELECT file FROM data LIMIT 1").fetchone()
        if not row:
            return {"mouse": None, "headset": None}
        data = json.loads(row[0])
    finally:
        conn.close()

    return _parse(data)


def cleanup_temp():
    """Remove copied G HUB DB snapshots from %TEMP%. Called on app exit."""
    for ext in ("", "-shm", "-wal"):
        try:
            os.remove(TMP_BASE + ext)
        except OSError:
            pass


def _parse(data):
    # Keys are flat: "battery/<device>/percentage" -> {"percentage": int, "time": ...}
    result = {"mouse": None, "headset": None}
    for key, val in data.items():
        parts = key.split("/")
        if len(parts) != 3 or parts[0] != "battery" or parts[2] != "percentage":
            continue
        if not isinstance(val, dict):
            continue
        pct = val.get("percentage")
        if not isinstance(pct, (int, float)):
            continue
        pct = max(0, min(100, int(pct)))
        device = parts[1].lower()
        if "mouse" in device and result["mouse"] is None:
            result["mouse"] = pct
        elif ("headset" in device or "headphone" in device) and result["headset"] is None:
            result["headset"] = pct
    return result
