# Removes logibar: startup shortcut, install dir, AppUserModelID, tray cache.

$APP_ID = "menzo.logibar"

Get-Process -Name logibar -ErrorAction SilentlyContinue | ForEach-Object {
    $_ | Stop-Process -Force
    Write-Host "Stopped logibar (PID $($_.Id))" -ForegroundColor Cyan
}

$startup = [Environment]::GetFolderPath("Startup")
$link    = Join-Path $startup "logibar.lnk"
if (Test-Path $link) {
    Remove-Item $link -Force
    Write-Host "Removed startup shortcut: $link" -ForegroundColor Green
}

$installDir = Join-Path $env:LOCALAPPDATA "Programs\logibar"
if (Test-Path $installDir) {
    Remove-Item $installDir -Recurse -Force
    Write-Host "Removed install dir:      $installDir" -ForegroundColor Green
}

$aumiKey = "HKCU:\Software\Classes\AppUserModelId\$APP_ID"
if (Test-Path $aumiKey) {
    Remove-Item $aumiKey -Recurse -Force
    Write-Host "Removed AppUserModelID:   $APP_ID" -ForegroundColor Green
}

$niPath = "HKCU:\Control Panel\NotifyIconSettings"
if (Test-Path $niPath) {
    Get-ChildItem $niPath | ForEach-Object {
        $p = (Get-ItemProperty $_.PSPath -Name ExecutablePath -ErrorAction SilentlyContinue).ExecutablePath
        if ($p -and $p -like "*logibar.exe") {
            Remove-Item $_.PSPath -Recurse -Force
            Write-Host "Removed tray cache entry: $($_.PSChildName)" -ForegroundColor DarkGray
        }
    }
}

Write-Host "`nDone. (Sign out / in to fully refresh tray Settings.)" -ForegroundColor Green
