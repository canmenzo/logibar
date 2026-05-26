# Installs logibar to %LOCALAPPDATA%\Programs\logibar and registers it for startup.
# Also registers AppUserModelID so Settings > Other tray icons shows "logibar" + Logitech-G icon.

$APP_ID = "menzo.logibar"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$src  = Join-Path $root "dist\logibar.exe"

if (-not (Test-Path $src)) {
    Write-Host "logibar.exe not found. Building first..." -ForegroundColor Yellow
    & (Join-Path $root "build.ps1")
    if ($LASTEXITCODE -ne 0) { Write-Host "Build failed." -ForegroundColor Red; exit 1 }
}

# Stop running instance before overwriting
Get-Process -Name logibar -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# Install to a stable per-user location
$installDir = Join-Path $env:LOCALAPPDATA "Programs\logibar"
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
$exe = Join-Path $installDir "logibar.exe"
Copy-Item $src $exe -Force

# Register AppUserModelID so Windows Settings shows "logibar" + exe icon (Logitech-G)
$aumiKey = "HKCU:\Software\Classes\AppUserModelId\$APP_ID"
New-Item -Path $aumiKey -Force | Out-Null
Set-ItemProperty -Path $aumiKey -Name "DisplayName"    -Value "logibar"
Set-ItemProperty -Path $aumiKey -Name "IconResource"   -Value "$exe,0"
Set-ItemProperty -Path $aumiKey -Name "IconUri"        -Value "$exe,0"

# Clear stale NotifyIconSettings entries (left over from older icon iterations)
$niPath = "HKCU:\Control Panel\NotifyIconSettings"
if (Test-Path $niPath) {
    Get-ChildItem $niPath | ForEach-Object {
        $p = (Get-ItemProperty $_.PSPath -Name ExecutablePath -ErrorAction SilentlyContinue).ExecutablePath
        if ($p -and $p -like "*logibar.exe") {
            Remove-Item $_.PSPath -Recurse -Force
            Write-Host "Cleared stale tray cache entry: $($_.PSChildName)" -ForegroundColor DarkGray
        }
    }
}

# Startup shortcut -> installed exe
$startup = [Environment]::GetFolderPath("Startup")
$link    = Join-Path $startup "logibar.lnk"
$ws = New-Object -ComObject WScript.Shell
$sc = $ws.CreateShortcut($link)
$sc.TargetPath       = $exe
$sc.WorkingDirectory = $installDir
$sc.IconLocation     = "$exe,0"
$sc.Description      = "logibar - Logitech battery tray monitor"
$sc.Save()

# Restart Explorer to refresh tray + Settings cache (intrusive but necessary)
Write-Host "Restarting Explorer to refresh tray cache..." -ForegroundColor Yellow
Stop-Process -Name explorer -Force
Start-Sleep -Seconds 2
if (-not (Get-Process -Name explorer -ErrorAction SilentlyContinue)) {
    Start-Process explorer
}

Start-Sleep -Seconds 1
Start-Process $exe

Write-Host "`nInstalled:        $exe" -ForegroundColor Green
Write-Host "Startup shortcut: $link" -ForegroundColor Green
Write-Host "AppUserModelID:   $APP_ID" -ForegroundColor Green
