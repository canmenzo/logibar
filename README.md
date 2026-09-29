# 🔋 logibar

[![release](https://img.shields.io/github/v/release/canmenzo/logibar)](https://github.com/canmenzo/logibar/releases/latest) ![platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows&logoColor=white) ![requires](https://img.shields.io/badge/requires-Logitech%20G%20HUB-00B8FC) [![license](https://img.shields.io/github/license/canmenzo/logibar)](LICENSE)

Your Logitech mouse, headset and keyboard battery, right in the Windows tray.

![logibar](assets/preview.png)

## ✨ Features

- 🖱️ One tray icon per device (mouse, headset, keyboard), with a battery bar under it
- 📋 Click an icon for device names and levels; the tooltip flags readings older than an hour
- 🟡 Bar turns yellow at 30%, 🔴 red at 15% (plus a "time to charge" alert)
- 🌗 Matches your light or dark taskbar
- 🪶 Tiny. No account, no internet, it just reads what G HUB already knows

## 🚀 Install

1. Have **[Logitech G HUB](https://www.logitechg.com/innovation/g-hub.html)** installed
2. Download **[logibar.exe](https://github.com/canmenzo/logibar/releases/latest/download/logibar.exe)**
3. Double-click it. Done.

💡 **"Windows protected your PC"?** Click **More info → Run anyway**.<br>
💡 **Don't see the icons?** Click **^** next to the clock and drag them onto the taskbar.<br>
💡 **Start on boot:** right-click an icon → **Start with Windows**.

## 🗑️ Uninstall

Right-click an icon → untick **Start with Windows** → **Quit**. Then delete `logibar.exe`.

Installed with `install.ps1`? Run `powershell -ExecutionPolicy Bypass -File uninstall.ps1` instead.

## 🛠️ Build from source

Needs [Python 3.8+](https://www.python.org/downloads/) (the floor set by `Pillow>=10`; developed on 3.11).

```powershell
git clone https://github.com/canmenzo/logibar
cd logibar
powershell -ExecutionPolicy Bypass -File install.ps1
```

`install.ps1` builds `dist\logibar.exe` with PyInstaller (via `build.ps1`), copies it to `%LOCALAPPDATA%\Programs\logibar`, turns on start with Windows and launches it. Run `build.ps1` alone if you only want the exe.

## 📄 License

[MIT](LICENSE)

<sub>Not affiliated with Logitech.</sub>
