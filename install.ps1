# Installs logibar to %LOCALAPPDATA%\Programs\logibar, starts it with Windows, and launches it.

$root = $PSScriptRoot
$src  = Join-Path $root "dist\logibar.exe"

if (-not (Test-Path $src)) {
    Write-Host "logibar.exe not found. Building first..." -ForegroundColor Yellow
    & (Join-Path $root "build.ps1")
    if ($LASTEXITCODE -ne 0) { Write-Host "Build failed." -ForegroundColor Red; exit 1 }
}

Get-Process -Name logibar -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

$installDir = Join-Path $env:LOCALAPPDATA "Programs\logibar"
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
$exe = Join-Path $installDir "logibar.exe"
Copy-Item $src $exe -Force

# Same Run entry the tray menu's "Start with Windows" toggle manages
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "logibar" -Value "`"$exe`""

# Older versions used a Startup-folder shortcut
Remove-Item (Join-Path ([Environment]::GetFolderPath("Startup")) "logibar.lnk") -ErrorAction SilentlyContinue

Start-Process $exe

Write-Host "`nlogibar is installed and running." -ForegroundColor Green
Write-Host "Icons hidden? Click ^ in the taskbar and drag them next to the clock." -ForegroundColor DarkGray
