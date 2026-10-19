[CmdletBinding(SupportsShouldProcess)]
param([string]$SandboxRoot, [switch]$NoStartup, [switch]$NoLaunch)

$ErrorActionPreference = 'Stop'
$source = if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'logibar.exe')) { $PSScriptRoot } else { Join-Path $PSScriptRoot 'dist' }
$files = @('logibar.exe', 'LICENSE.txt', 'THIRD_PARTY_NOTICES.txt', 'build-info.json', 'uninstall.ps1')
foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath (Join-Path $source $file) -PathType Leaf)) { throw "Missing $file. Run build.ps1 first." }
}
$root = $env:LOCALAPPDATA
if ($SandboxRoot) {
    $root = [IO.Path]::GetFullPath($SandboxRoot)
    $tempPrefix = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $root.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'SandboxRoot must be a subdirectory of the Windows temp directory.' }
}
$installDir = [IO.Path]::GetFullPath((Join-Path $root 'Programs\logibar'))
$ancestor = $installDir
while ($ancestor) {
    if ((Test-Path -LiteralPath $ancestor) -and ((Get-Item -LiteralPath $ancestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Refusing linked install path: $ancestor" }
    $ancestor = Split-Path -Parent $ancestor
}
$exe = Join-Path $installDir 'logibar.exe'
$running = Get-Process -Name logibar -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe }
if ($running) { throw 'Quit the installed logibar from its tray menu before installing.' }
if (-not $PSCmdlet.ShouldProcess($installDir, 'Install logibar files and requested per-user integration')) { return }
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
foreach ($file in $files) { Copy-Item -LiteralPath (Join-Path $source $file) -Destination (Join-Path $installDir $file) -Force }
@{ product = 'menzo.logibar'; version = '2.0.0'; directory = $installDir; files = $files } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $installDir 'install.json') -Encoding UTF8
if (-not $SandboxRoot) {
    $appKey = 'HKCU:\Software\Classes\AppUserModelId\menzo.logibar'
    New-Item -Path $appKey -Force | Out-Null
    Set-ItemProperty -LiteralPath $appKey -Name DisplayName -Value 'logibar'
    Set-ItemProperty -LiteralPath $appKey -Name IconResource -Value "$exe,0"
    Set-ItemProperty -LiteralPath $appKey -Name IconUri -Value "$exe,0"
    if (-not $NoStartup) {
        $runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
        New-Item -Path $runKey -Force | Out-Null
        $existing = (Get-ItemProperty -LiteralPath $runKey -Name logibar -ErrorAction SilentlyContinue).logibar
        $quoted = '"{0}"' -f $exe
        if (-not ($existing -eq $quoted -or ($existing -and $existing.StartsWith($quoted + ' ', [StringComparison]::OrdinalIgnoreCase)))) {
            Set-ItemProperty -LiteralPath $runKey -Name logibar -Value $quoted
        }
    }
    $legacy = Join-Path ([Environment]::GetFolderPath('Startup')) 'logibar.lnk'
    if (-not $NoStartup -and (Test-Path -LiteralPath $legacy)) {
        $shell = New-Object -ComObject WScript.Shell
        if ($shell.CreateShortcut($legacy).TargetPath -eq $exe) { Remove-Item -LiteralPath $legacy -Force }
    }
    if (-not $NoLaunch) { Start-Process -FilePath $exe -WindowStyle Hidden }
}
Write-Output "Installed: $exe"
