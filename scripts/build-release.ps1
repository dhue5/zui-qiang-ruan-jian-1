$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$srcTauri = Join-Path $root 'src-tauri'
$release = Join-Path $srcTauri 'target\release'

Write-Host '==> 1/5 品牌字符串检查' -ForegroundColor Cyan
$conf = Get-Content -Raw (Join-Path $srcTauri 'tauri.conf.json')
$cargo = Get-Content -Raw (Join-Path $srcTauri 'Cargo.toml')
$cfgObj = $conf | ConvertFrom-Json
$product = [string]$cfgObj.productName
$id = [string]$cfgObj.identifier
$binNameLine = ($cargo -split "`n" | Where-Object { $_ -match '^\s*name\s*=' } | Select-Object -First 1)
$binName = $binNameLine.Split('"')[1].Trim()
if (-not $product -or -not $id -or -not $binName) {
    throw 'unable to parse productName / identifier / cargo name'
}
if ($product -eq 'Anime Next' -or $id -eq 'com.animenext.app' -or $binName -eq 'anime-next') {
    throw 'tauri.conf.json still contains old brand strings'
}
Write-Host "brand: $product ($binName, $id)" -ForegroundColor Green

Write-Host '==> 1.5/6 版本号自动递增' -ForegroundColor Cyan
& (Join-Path $PSScriptRoot 'bump-version.ps1')
$conf = Get-Content -Raw (Join-Path $srcTauri 'tauri.conf.json')
$newVersion = [regex]::Match($conf, '"version"\s*:\s*"([^"]+)"').Groups[1].Value

Write-Host '==> 2/6 cargo test' -ForegroundColor Cyan
Push-Location $srcTauri
try { cargo test --release -- --test-threads=1 | Select-Object -Last 15 } finally { Pop-Location }

Write-Host '==> 3/6 tauri build' -ForegroundColor Cyan
Push-Location $root
try { npx tauri build | Tee-Object (Join-Path $root 'work\tauri-build.log') } finally { Pop-Location }

Write-Host '==> 4/6 二进制品牌核对' -ForegroundColor Cyan
$exe = Join-Path $release "$binName.exe"
if (-not (Test-Path $exe)) { throw "missing $exe" }
$bytes = [System.IO.File]::ReadAllBytes($exe)
function Count-Str([byte[]]$buf, [string]$needle) {
    $b = [System.Text.Encoding]::UTF8.GetBytes($needle)
    $count = 0
    for ($i = 0; $i -le $buf.Length - $b.Length; $i++) {
        $match = $true
        for ($j = 0; $j -lt $b.Length; $j++) { if ($buf[$i + $j] -ne $b[$j]) { $match = $false; break } }
        if ($match) { $count++ }
    }
    $count
}
$old = Count-Str $bytes 'Anime Next'
$oldId = Count-Str $bytes 'com.animenext.app'
$oldKey = Count-Str $bytes 'sk-1196'
if (($old + $oldId + $oldKey) -gt 0) { throw "old brand leaked: AnimeNext=$old id=$oldId key=$oldKey" }
Write-Host "OK (old brand=0)" -ForegroundColor Green

Write-Host '==> 5/6 生成本地升级清单' -ForegroundColor Cyan
$nsisPath = Join-Path $release "bundle\nsis\最强软件_${newVersion}_x64-setup.exe"
$manifest = @{
    version = $newVersion
    notes = "最强软件 ${newVersion} 更新"
    download_url = "file:///" + (($nsisPath -replace '\\','/'))
    asset_id = "zuiqiang-${newVersion}"
} | ConvertTo-Json
Set-Content -LiteralPath (Join-Path $PSScriptRoot 'update-manifest.json') -Value $manifest -Encoding UTF8
Write-Host "manifest: $newVersion -> $nsisPath" -ForegroundColor Green

Write-Host '==> 6/6 产物清单' -ForegroundColor Cyan
Get-ChildItem $release -Filter *.exe | Select-Object Name, @{N='MiB';E={[math]::Round($_.Length/1MB,2)}}
Get-ChildItem (Join-Path $release 'bundle\msi') -Filter *.msi | Select-Object Name, @{N='MiB';E={[math]::Round($_.Length/1MB,2)}}
Get-ChildItem (Join-Path $release 'bundle\nsis') -Filter *.exe | Select-Object Name, @{N='MiB';E={[math]::Round($_.Length/1MB,2)}}
Write-Host '==> 完成' -ForegroundColor Green
