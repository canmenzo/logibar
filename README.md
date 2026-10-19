# logibar

Native Rust battery indicators for Logitech wireless devices on Windows 10/11 x64. Reads Logitech G HUB's local SQLite settings database; no account, network requests, USB hooks, or background service.

Mouse and headset each have a tray icon. Keyboard support from v1.1 is retained and appears when a keyboard reading exists. The device silhouettes and battery bar are rendered at the Windows tray icon size, using the light/dark taskbar setting. Click either icon for device names, percentages, **Refresh**, **Start with Windows**, and **Quit**. Context menus use standard Windows styling rather than private theme APIs.

| Battery | Bar |
| --- | --- |
| Above 30% | White on dark taskbars; dark on light taskbars |
| 16–30% | Amber |
| 0–15% | Red |
| Missing or explicitly disconnected | Dimmed glyph, unfilled bar |

Defaults match v1.1: refresh every 30 seconds, low at 30%, critical at 15%. A low-battery notification appears once per device below the critical threshold and rearms after the battery rises above the low threshold. Stale readings over an hour old and explicitly disconnected devices do not trigger notifications. Windows may suppress notifications according to its own settings.

## Build and run

Install [Rust](https://rust-lang.org/tools/install/) and Visual Studio 2022 Build Tools with **Desktop development with C++** and a Windows SDK. The checked-in toolchain pins Rust 1.99.1; Cargo.lock pins dependencies. Builds download crates when they are not cached; the running app does not access the network.

```powershell
.\build.ps1
.\dist\logibar.exe
```

The build produces a normal Windows PE executable with embedded icons, version 2.0.0, and an asInvoker/DPI-aware manifest. SQLite and the C runtime are compiled into the binary; no separate Visual C++ redistributable installation is needed. Python, PyInstaller, runtime extraction, UPX, and other packers are absent. `dist` also contains the MIT license, dependency license texts, install/uninstall scripts, and `build-info.json` with the source revision, dirty-worktree flag, toolchain, lockfile hash, and executable SHA-256. Distribute the accompanying notices with the executable.

This branch has not published a Rust release; older GitHub release downloads may still be Python builds. Verify a binary against the provenance supplied with that specific build. Native compilation and smaller packaging do not guarantee antivirus acceptance. Builds here are unsigned; no antivirus or SmartScreen bypass is part of installation. See [packaging details](docs/packaging.md).

Requires G HUB to supply readings at `%LOCALAPPDATA%\LGHUB\settings.db`. If icons are hidden, open the taskbar overflow and drag them beside the clock. Only one default instance runs per Windows session. Quit older Python instances before switching to Rust.

## Settings

```powershell
.\dist\logibar.exe --interval 10 --critical 20 --low 50
.\dist\logibar.exe --db 'D:\G HUB data\settings.db'
```

Percentages must be 0–100 with critical <= low; interval is 1–86400 seconds. Enabling **Start with Windows** saves the executable path, database path, thresholds, and interval to the per-user `logibar` Run entry. These were the only persistent startup settings in v1.1; there is no separate settings file. Disable startup before moving a portable executable. Re-enabling startup records the current options.

The newest `data` row is parsed, with the newest timestamp selected per device kind. Names are matched against `devices/known`. G HUB's schema is not a public contract: some unsupported models may remain unclassified. G HUB often caches disconnected hardware without an explicit status; an age label is evidence of a cached reading, not proof of a live connection. If reading fails temporarily, the app keeps previous values, labels them as cached, and exposes the error in the menu. Missing or empty databases clear the readings. No hardware polling or control occurs.

## Install and remove

Quit any instance at the installed path, then run:

```powershell
.\install.ps1
# Later: Quit from the tray, then
.\uninstall.ps1
```

Installation copies the native build and notices to `%LOCALAPPDATA%\Programs\logibar`, registers its app identity, enables startup, and launches it. No elevation or Explorer restart. Use `-NoStartup -NoLaunch` to omit startup enablement and launch; existing startup settings are preserved. Both scripts support `-WhatIf`.

Uninstall removes only its known files and startup/identity entries that point to the installed executable. Unrelated files stay in place. The installed folder contains its own `uninstall.ps1`. Portable use requires no installer: untick **Start with Windows**, choose **Quit**, then remove the portable files yourself.

For a legacy Python installation, untick startup and Quit the old app first. The Rust installer can replace the old executable at the same installed path; it preserves matching Run-entry arguments and removes a legacy startup shortcut only when it targets that installed executable. Legacy app binaries in other folders are not stopped or deleted. The new uninstaller requires its own `install.json`; it does not recursively erase an unrecognized legacy folder or clear Windows tray caches.

## Verify safely

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --release --locked --target x86_64-pc-windows-msvc -- --nocapture
.\build.ps1
.\tests\install.ps1
```

The native tray integration tests need an interactive Windows desktop with Explorer running. They use synthetic SQLite fixtures, separate instance names, a bounded exit timer, and `--sandbox`, which disables startup changes and battery notifications. Packaging tests use a new temp subdirectory, preserve unrelated files, and verify that startup, the existing app identity, the legacy shortcut, and Explorer remain unchanged. They never run the real profile installer.

For a bounded manual smoke test:

```powershell
.\dist\logibar.exe --sandbox --instance smoke --db "$env:TEMP\missing-logibar.db" --exit-after 15 --report "$env:TEMP\logibar-smoke.json"
```

`--report` writes device names/readings locally only when explicitly requested. `--inspect --db PATH --report PATH` reads the database once without creating tray icons. Diagnostic reports do not upload anything. [Validation evidence](docs/validation.md) records the tested scope and remaining hardware/antivirus limitations. On non-Windows hosts, only the portable core tests are applicable (`cargo test --lib --test database`); there is no macOS tray implementation.

## License

[MIT](LICENSE), copyright 2026 Can Ozmen. Original history and the existing `assets/preview.png` are retained; the preview depicts v1.1. Device/app artwork is rendered from the Rust source. Third-party runtime license texts are collected from the locked crates into `dist/THIRD_PARTY_NOTICES.txt`; SQLite is public domain. Not affiliated with Logitech.
