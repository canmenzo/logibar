# Builds dist\logibar.exe with embedded icon + version info.

Push-Location $PSScriptRoot
try {
    python -m pip install -r requirements.txt pyinstaller --quiet --disable-pip-version-check
    if ($LASTEXITCODE -ne 0) { Write-Host "pip install failed" -ForegroundColor Red; exit 1 }

    python make_ico.py
    if ($LASTEXITCODE -ne 0) { Write-Host "make_ico failed" -ForegroundColor Red; exit 1 }

    python -m PyInstaller `
        --onefile `
        --windowed `
        --name logibar `
        --icon assets/app.ico `
        --version-file version_info.txt `
        --exclude-module numpy `
        --exclude-module tkinter `
        --exclude-module PIL._avif `
        --noconfirm `
        main.py
    if ($LASTEXITCODE -ne 0) { Write-Host "PyInstaller failed (exit $LASTEXITCODE)" -ForegroundColor Red; exit 1 }

    Write-Host "`nDone. Executable: dist\logibar.exe" -ForegroundColor Green
} finally {
    Pop-Location
}
