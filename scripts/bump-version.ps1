param(
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$confPath = Join-Path $root 'src-tauri\tauri.conf.json'
$cargoPath = Join-Path $root 'src-tauri\Cargo.toml'
$pkgPath = Join-Path $root 'package.json'

$conf = Get-Content -Raw $confPath
$old = [regex]::Match($conf, '"version"\s*:\s*"([^"]+)"').Groups[1].Value
if (-not $old) { throw 'unable to read current version from tauri.conf.json' }

if ($Version) {
    $new = $Version
} else {
    $parts = $old -split '\.'
    if ($parts.Count -ne 3) { throw "unsupported version format: $old" }
    $new = '{0}.{1}.{2}' -f $parts[0], $parts[1], ([int]$parts[2] + 1)
}

$conf = [regex]::Replace($conf, '("version"\s*:\s*")[^"]+(")', {
    param($m)
    return $m.Groups[1].Value + $new + $m.Groups[2].Value
})
Set-Content -LiteralPath $confPath -Value $conf -Encoding UTF8

$cargo = Get-Content -Raw $cargoPath
$cargo = [regex]::Replace($cargo, '(?m)^version\s*=\s*"([^"]+)"', {
    param($m)
    return 'version = "' + $new + '"'
})
Set-Content -LiteralPath $cargoPath -Value $cargo -Encoding UTF8

$pkg = Get-Content -Raw $pkgPath
$pkg = [regex]::Replace($pkg, '("version"\s*:\s*")[^"]+(")', {
    param($m)
    return $m.Groups[1].Value + $new + $m.Groups[2].Value
})
Set-Content -LiteralPath $pkgPath -Value $pkg -Encoding UTF8

Write-Host "version: $old -> $new"
