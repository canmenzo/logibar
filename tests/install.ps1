$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$sandbox = Join-Path ([IO.Path]::GetTempPath()) ('logibar-install-test-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $sandbox | Out-Null
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$appKey = 'HKCU:\Software\Classes\AppUserModelId\menzo.logibar'
$beforeRun = (Get-ItemProperty -LiteralPath $runKey -Name logibar -ErrorAction SilentlyContinue).logibar
$beforeApp = Get-ItemProperty -LiteralPath $appKey -ErrorAction SilentlyContinue | ConvertTo-Json
$beforeExplorer = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id) -join ','
$shortcut = Join-Path ([Environment]::GetFolderPath('Startup')) 'logibar.lnk'
$beforeShortcut = if (Test-Path -LiteralPath $shortcut) { (Get-FileHash -LiteralPath $shortcut).Hash } else { $null }
$installDir = Join-Path $sandbox 'Programs\logibar'
& (Join-Path $repo 'install.ps1') -SandboxRoot $sandbox -WhatIf
if (Test-Path -LiteralPath $installDir) { throw 'WhatIf changed the filesystem.' }
$rejected = $false
try { & (Join-Path $repo 'install.ps1') -SandboxRoot $repo } catch { $rejected = $true }
if (-not $rejected) { throw 'Installer accepted a sandbox outside temp.' }
& (Join-Path $repo 'install.ps1') -SandboxRoot $sandbox
$exe = Join-Path $installDir 'logibar.exe'
if ((Get-FileHash -LiteralPath $exe).Hash -ne (Get-FileHash -LiteralPath (Join-Path $repo 'dist\logibar.exe')).Hash) { throw 'Installed binary differs from native build.' }
foreach ($file in @('LICENSE.txt', 'THIRD_PARTY_NOTICES.txt', 'build-info.json', 'uninstall.ps1', 'install.json')) {
    if (-not (Test-Path -LiteralPath (Join-Path $installDir $file))) { throw "Missing installed $file." }
}
& (Join-Path $repo 'uninstall.ps1') -SandboxRoot $sandbox -WhatIf
if (-not (Test-Path -LiteralPath $exe)) { throw 'Uninstaller WhatIf removed the binary.' }
$report = Join-Path $sandbox 'runtime.json'
$instance = 'package-' + [Guid]::NewGuid().ToString('N')
$arguments = '--sandbox --instance {0} --interval 1 --exit-after 3 --db "{1}" --report "{2}"' -f $instance, (Join-Path $sandbox 'missing.db'), $report
$process = Start-Process -FilePath $exe -ArgumentList $arguments -PassThru -WindowStyle Hidden
$deadline = [DateTime]::UtcNow.AddSeconds(10)
while (-not (Test-Path -LiteralPath $report)) {
    if ([DateTime]::UtcNow -gt $deadline) { throw 'Installed application did not produce a report.' }
    Start-Sleep -Milliseconds 100
}
$rejected = $false
try { & (Join-Path $repo 'install.ps1') -SandboxRoot $sandbox } catch { $rejected = $true }
if (-not $rejected) { throw 'Installer attempted to overwrite a running executable.' }
if (-not $process.WaitForExit(10000)) { throw 'Installed application did not exit gracefully.' }
if ($process.ExitCode -ne 0) { throw 'Installed application exited with an error.' }
$runtime = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
if ($runtime.status -ne 'exited' -or $runtime.adds -ne 2 -or $runtime.removes -ne 2) { throw 'Installed application tray lifecycle failed.' }
$sentinel = Join-Path $installDir 'unrelated-user-file.txt'
Set-Content -LiteralPath $sentinel -Value 'preserve me'
& (Join-Path $installDir 'uninstall.ps1') -SandboxRoot $sandbox
if (-not (Test-Path -LiteralPath $sentinel) -or (Test-Path -LiteralPath $exe)) { throw 'Uninstaller did not preserve unrelated files or remove its binary.' }
$resolvedSentinel = [IO.Path]::GetFullPath($sentinel)
if (-not $resolvedSentinel.StartsWith([IO.Path]::GetFullPath($sandbox) + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Cleanup target escaped sandbox.' }
Remove-Item -LiteralPath $resolvedSentinel
& (Join-Path $repo 'install.ps1') -SandboxRoot $sandbox
& (Join-Path $repo 'uninstall.ps1') -SandboxRoot $sandbox
if (Test-Path -LiteralPath $installDir) { throw 'Empty installation directory remains.' }
$afterRun = (Get-ItemProperty -LiteralPath $runKey -Name logibar -ErrorAction SilentlyContinue).logibar
$afterApp = Get-ItemProperty -LiteralPath $appKey -ErrorAction SilentlyContinue | ConvertTo-Json
$afterExplorer = @(Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id) -join ','
$afterShortcut = if (Test-Path -LiteralPath $shortcut) { (Get-FileHash -LiteralPath $shortcut).Hash } else { $null }
if ($beforeRun -ne $afterRun -or $beforeApp -ne $afterApp -or $beforeShortcut -ne $afterShortcut) { throw 'Sandbox test changed user integration.' }
if ($beforeExplorer -ne $afterExplorer) { throw 'Explorer process changed during sandbox test.' }
$checks = @('install WhatIf', 'outside-temp rejection', 'installed binary hash and notices', 'uninstall WhatIf', 'running-executable protection', 'installed native tray lifecycle', 'unrelated file preservation', 'complete owned-file removal', 'startup registry shortcut and Explorer unchanged')
@{ status = 'passed'; checks = $checks; runtime = $runtime } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $sandbox 'install-test-results.json') -Encoding UTF8
Write-Output "PASS: $($checks.Count) packaging checks. Evidence: $sandbox"
