# Validation of the native Rust rewrite

Tested on 2026-10-19 on an interactive Windows 11 Pro x64 desktop, OS version 10.0.26200, with Explorer running. Rust 1.99.1 (58cf8f9c3), Cargo 1.99.1, MSVC 14.44.35228, and the installed Windows SDK were used. The release target is x86_64-pc-windows-msvc with a statically linked C runtime. Tests use synthetic device data, not a user's G HUB database.

## Local checks

| Check | Result |
| --- | --- |
| cargo fmt --check | Passed |
| cargo clippy --locked --all-targets -- -D warnings | Passed |
| cargo test --release --locked --target x86_64-pc-windows-msvc -- --nocapture | 14 passed: 3 unit, 8 database/core integration, 3 Windows runtime integration |
| build.ps1 | Native release executable built; embedded file description logibar, file version 2.0.0.0 |
| tests/install.ps1 | 9 packaging checks passed in a new Windows temp sandbox |
| dumpbin /dependents | Only Windows system DLLs; no Python, SQLite DLL, or Visual C++ redistributable DLL dependency |
| Authenticode inspection | Unsigned; no certificate or signing assertion |

The database integration tests create real SQLite files with the representative JSON from tests/fixtures/settings.json, stored as both TEXT and BLOB. They verify newest database row selection, newest per-kind timestamps including UTC offsets, model matching without kind words (G502 X / G915), percentage clamping, malformed values, missing/empty/corrupt databases, explicit disconnection, and stale labels. WAL tests hold a writer open, confirm committed readings remain visible while an uncommitted row is ignored, and check database/WAL bytes remain unchanged.

Threshold tests cover 0, 15, 16, 30, 31, and 100 percent plus custom 20/50 thresholds. Notification logic is tested for one alert per low-battery episode, rearming above the low threshold, and suppression for disconnected/stale readings. Raster checks cover transparent unknown icons, light-taskbar contrast, critical bar color, and valid native ICO frame offsets.

## Windows runtime evidence

The native release integration test observes the launched process ID and independently calls Shell_NotifyIconGetRect for IDs 1, 2, and 3. All three return valid shell rectangles for the mouse, headset, and keyboard icons. This verifies registration even when Windows places icons in its overflow area; returned rectangles can refer to the overflow affordance. A duplicate instance exits successfully while the first stays running.

The fixture then supplies malformed JSON: the app retains the previous 70% mouse reading and reports the read error. A subsequent empty snapshot clears readings and removes the optional keyboard icon. WM_CLOSE triggers graceful shutdown, exit code 0, three successful icon removals, and no remaining shell rectangles. A separate missing-database test shows exactly two unknown icons and no created database. Its bounded timer exits successfully even while the native context menu is open. A third native test verifies one-shot --inspect output without a tray.

A separate release smoke run showed the native menu on the Windows desktop. Its screenshot was inspected locally: mouse 70%, headset 30%, disconnected keyboard 15%, age labels, Refresh, disabled sandbox startup control, and Quit. The screenshot and synthetic smoke reports remain local; they are not embedded in this public repository.

The nine packaging checks cover install WhatIf, rejection of a non-temp sandbox, exact installed binary hash and accompanying notices, uninstall WhatIf, refusal to overwrite the running installed executable, installed-binary tray lifecycle, preservation of an unrelated file, complete removal of owned files when no unrelated file remains, and unchanged user Run entry/app identity/startup shortcut/Explorer PID. Installation and removal were executed only inside a temp sandbox, never against the user's real profile install. Existing unrelated logibar processes were not stopped.

## Scope and limitations

G HUB hardware was unavailable on the test host. These tests verify the database layout inherited from the previous app using representative fixtures, not live discharge/reconnection telemetry from physical Logitech devices. G HUB's private schema may change. Notifications were suppressed during runtime smoke tests; their threshold/hysteresis logic is covered by tests, while actual notification delivery and user startup registration remain unexercised on the real profile.

The Mac mini was reached over SSH and identified as Darwin. It had no Rust toolchain; no toolchain or service was installed there. Windows tray registration, Win32 messages, PE resources, per-user registry integration, and PowerShell install/uninstall cannot be validated by running a macOS executable. Portable core tests can run on macOS if Rust is installed separately; all results above came from Windows.

No antivirus multi-engine scan, SmartScreen reputation test, vendor submission, binary upload, code-signing operation, production install, or deployment was performed. Native compilation is not evidence of zero detections. GitHub CI status is separate from these local results; account billing can prevent Actions from starting, and a blocked run must not be described as passed. Hosted CI builds and checks the portable core; interactive tray and packaging checks are deliberately local.

For the exact local artifact's revision, clean/dirty source state, executable SHA-256, and lockfile hash, inspect dist/build-info.json emitted by build.ps1. Binaries and local diagnostic data are ignored by Git. The review PR contains source only.
