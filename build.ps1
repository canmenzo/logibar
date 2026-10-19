$ErrorActionPreference = 'Stop'
$cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $cargo) { $cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path -LiteralPath $cargo)) { throw 'Install Rust and MSVC Build Tools first; see README.md.' }
Push-Location $PSScriptRoot
try {
    & $cargo build --release --locked --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Native release build failed.' }
    $metadata = (& $cargo metadata --locked --filter-platform x86_64-pc-windows-msvc --format-version 1 | ConvertFrom-Json)
    if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve dependency notices.' }
    $nodes = @{}
    foreach ($node in $metadata.resolve.nodes) { $nodes[$node.id] = $node }
    $included = @{}
    $pending = New-Object 'System.Collections.Generic.Queue[string]'
    $pending.Enqueue($metadata.resolve.root)
    while ($pending.Count) {
        $id = $pending.Dequeue()
        if ($included.ContainsKey($id)) { continue }
        $included[$id] = $true
        foreach ($dependency in $nodes[$id].deps) {
            if (@($dependency.dep_kinds | Where-Object { $null -eq $_.kind }).Count) { $pending.Enqueue($dependency.pkg) }
        }
    }
    $notices = New-Object System.Text.StringBuilder
    [void]$notices.AppendLine('logibar third-party runtime dependency notices')
    [void]$notices.AppendLine('SQLite is in the public domain: https://sqlite.org/copyright.html')
    foreach ($package in ($metadata.packages | Sort-Object name)) {
        if (-not $included.ContainsKey($package.id) -or $package.name -eq 'logibar') { continue }
        [void]$notices.AppendLine("===== $($package.name) $($package.version) ($($package.license)) =====")
        $packageDir = Split-Path -Parent $package.manifest_path
        $licenses = @(Get-ChildItem -LiteralPath $packageDir -File | Where-Object { $_.Name -match '^(LICENSE|COPYING|UNLICENSE)' })
        if ($package.license_file) { $licenses += Get-Item -LiteralPath (Join-Path $packageDir $package.license_file) }
        if (-not $licenses.Count) { throw "Missing license text for $($package.name)." }
        foreach ($license in ($licenses | Sort-Object FullName -Unique)) {
            [void]$notices.AppendLine("--- $($license.Name) ---")
            [void]$notices.AppendLine([IO.File]::ReadAllText($license.FullName))
        }
    }
    New-Item -ItemType Directory -Path dist -Force | Out-Null
    Copy-Item -LiteralPath target\x86_64-pc-windows-msvc\release\logibar.exe -Destination dist\logibar.exe -Force
    Copy-Item -LiteralPath LICENSE -Destination dist\LICENSE.txt -Force
    Copy-Item -LiteralPath install.ps1,uninstall.ps1 -Destination dist -Force
    [IO.File]::WriteAllText((Join-Path $PSScriptRoot 'dist\THIRD_PARTY_NOTICES.txt'), $notices.ToString())
    $rustc = Join-Path (Split-Path -Parent $cargo) 'rustc.exe'
    $buildInfo = [ordered]@{
        version = '2.0.0'
        target = 'x86_64-pc-windows-msvc'
        c_runtime = 'static (+crt-static from .cargo/config.toml)'
        source_revision = (git rev-parse HEAD)
        source_dirty = [bool](git status --porcelain)
        built_utc = [DateTime]::UtcNow.ToString('o')
        rustc = (& $rustc -Vv) -join [Environment]::NewLine
        cargo = (& $cargo -V)
        command = 'cargo build --release --locked --target x86_64-pc-windows-msvc'
        lockfile_sha256 = (Get-FileHash -LiteralPath Cargo.lock -Algorithm SHA256).Hash
        binary_sha256 = (Get-FileHash -LiteralPath dist\logibar.exe -Algorithm SHA256).Hash
    }
    $buildInfo | ConvertTo-Json | Set-Content -LiteralPath dist\build-info.json -Encoding UTF8
    Write-Output "Built dist\logibar.exe ($($buildInfo.binary_sha256))"
} finally {
    Pop-Location
}
