[CmdletBinding(SupportsShouldProcess)]
param([string]$SandboxRoot)

$ErrorActionPreference = 'Stop'
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
$marker = Join-Path $installDir 'install.json'
if (-not (Test-Path -LiteralPath $marker)) { throw 'No native logibar installation manifest found. See README.md for legacy removal.' }
$manifest = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
if ($manifest.product -ne 'menzo.logibar' -or $manifest.directory -ne $installDir) { throw 'Installation manifest does not match this directory.' }
$exe = Join-Path $installDir 'logibar.exe'
if (Get-Process -Name logibar -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe }) { throw 'Quit logibar from its tray menu before uninstalling.' }
if (-not $PSCmdlet.ShouldProcess($installDir, 'Remove owned logibar files and matching per-user integration')) { return }
if (-not $SandboxRoot) {
    $runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
    $command = (Get-ItemProperty -LiteralPath $runKey -Name logibar -ErrorAction SilentlyContinue).logibar
    $quoted = '"{0}"' -f $exe
    if ($command -eq $quoted -or ($command -and $command.StartsWith($quoted + ' ', [StringComparison]::OrdinalIgnoreCase))) {
        Remove-ItemProperty -LiteralPath $runKey -Name logibar
    }
    $appKey = 'HKCU:\Software\Classes\AppUserModelId\menzo.logibar'
    if ((Get-ItemProperty -LiteralPath $appKey -Name IconResource -ErrorAction SilentlyContinue).IconResource -eq "$exe,0") { Remove-Item -LiteralPath $appKey -Force }
    $legacy = Join-Path ([Environment]::GetFolderPath('Startup')) 'logibar.lnk'
    if (Test-Path -LiteralPath $legacy) {
        $shell = New-Object -ComObject WScript.Shell
        if ($shell.CreateShortcut($legacy).TargetPath -eq $exe) { Remove-Item -LiteralPath $legacy -Force }
    }
}
$owned = @('logibar.exe', 'LICENSE.txt', 'THIRD_PARTY_NOTICES.txt', 'build-info.json', 'uninstall.ps1', 'install.json')
foreach ($file in $owned) {
    $path = Join-Path $installDir $file
    if (Test-Path -LiteralPath $path -PathType Leaf) { Remove-Item -LiteralPath $path -Force }
}
if (-not (Get-ChildItem -LiteralPath $installDir -Force)) { Remove-Item -LiteralPath $installDir -Force }
Write-Output "Removed logibar from $installDir. Any unrelated files were retained."
