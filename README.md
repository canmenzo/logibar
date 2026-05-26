# logibar

Windows tray battery monitor for Logitech wireless devices. Reads from G HUB's local SQLite DB — no API hacks.

Two tray icons (mouse + headset), tinted by battery level: green > 50%, yellow ≤ 50%, red ≤ 20%, gray = unknown. Refreshes every 30 seconds.

## Run from source

```powershell
python -m pip install -r requirements.txt
pythonw main.py
```

Requires Logitech G HUB running in the background.

## Build .exe

```powershell
.\build.ps1   # output: dist\logibar.exe (embedded Logitech-G icon + "logibar" identity)
```

## Install to startup

```powershell
.\install.ps1   # copies exe to %LOCALAPPDATA%\Programs\logibar, adds startup shortcut, launches
.\uninstall.ps1 # removes everything: shortcut, install dir, registry entries, tray cache
```

After install:
- Task Manager > Startup → **logibar** with Logitech-G icon
- Settings > Personalization > Taskbar > Other system tray icons → **logibar** with Logitech-G icon
- Process name: `logibar.exe`

`install.ps1` restarts Explorer to flush the tray-icon cache.

## Layout

```
main.py             tray app (loads PNGs, tints by battery level)
ghub.py             reads G HUB settings.db
make_ico.py         generates assets/app.ico from the Logitech-G PNG
version_info.txt    sets FileDescription = "logibar" on the .exe
build.ps1           PyInstaller bundle (--onefile, --windowed, --icon, --version-file)
install.ps1         per-user install + startup + AppUserModelID + tray cache reset
uninstall.ps1       full removal
assets/             mouseicon.png, headseticon.png, source logo for app.ico
```
