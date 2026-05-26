# Builds dist\logibar.exe with embedded icon + version info.
# Result shows "logibar" with the Logitech-G icon in Task Manager, Startup, and Tray settings.

python -m pip install pyinstaller --quiet *> $null
python make_ico.py
if ($LASTEXITCODE -ne 0) { Write-Host "make_ico failed" -ForegroundColor Red; exit 1 }

python -m PyInstaller `
    --onefile `
    --windowed `
    --name logibar `
    --icon assets/app.ico `
    --version-file version_info.txt `
    --add-data "ghub.py;." `
    --add-data "assets;assets" `
    --noconfirm `
    main.py

if ($LASTEXITCODE -ne 0) { Write-Host "PyInstaller failed (exit $LASTEXITCODE)" -ForegroundColor Red; exit 1 }
Write-Host "`nDone. Executable: dist\logibar.exe" -ForegroundColor Green
