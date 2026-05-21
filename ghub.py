import json
import os
import shutil
import sqlite3

DB_SRC = os.path.join(os.environ["LOCALAPPDATA"], "LGHUB", "settings.db")
TMP_BASE = os.path.join(os.environ.get("TEMP", "."), "logibar_tmp")


def get_battery():
    """Returns {"mouse": int|None, "headset": int|None} from G HUB settings.db."""
    for ext in ("", "-shm", "-wal"):
        src = DB_SRC + ext
        if os.path.exists(src):
            shutil.copy2(src, TMP_BASE + ".db" + ext)

    conn = sqlite3.connect(TMP_BASE + ".db")
    try:
        row = conn.execute("SELECT file FROM data LIMIT 1").fetchone()
        data = json.loads(row[0])
    finally:
        conn.close()

    return _parse(data)


def _parse(data):
    # Keys are flat: "battery/<device>/percentage" -> {"percentage": int, "time": ...}
    result = {"mouse": None, "headset": None}
    for key, val in data.items():
        parts = key.split("/")
        if len(parts) != 3 or parts[0] != "battery" or parts[2] != "percentage":
            continue
        pct = val.get("percentage") if isinstance(val, dict) else None
        if pct is None:
            continue
        device = parts[1].lower()
        if "mouse" in device and result["mouse"] is None:
            result["mouse"] = int(pct)
        elif ("headset" in device or "headphone" in device) and result["headset"] is None:
            result["headset"] = int(pct)
    return result
