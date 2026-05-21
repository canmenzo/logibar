# logibar

Windows system tray battery monitor for Logitech wireless devices. Reads from G HUB's local database — no API hacks, no polling overhead.

Two icons in tray: one for mouse, one for headset. Each shows a battery bar (left→right) with percentage. Monochrome. Updates every 30 seconds.

## Setup

```powershell
python -m pip install -r requirements.txt
pythonw main.py
```

Requires Logitech G HUB running in background.

## Build as .exe (shows "logibar" in tray settings instead of "Python")

```powershell
.\build.ps1
# output: dist\logibar.exe
```

## Auto-start

Put a shortcut to `dist\logibar.exe` (or `run.vbs` for the script version) in `shell:startup`.
