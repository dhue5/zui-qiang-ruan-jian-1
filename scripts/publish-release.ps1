param(
    [string]$Version = "",
    [switch]$CreateTag,
    [switch]$CreateGitHubRelease
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
if (-not $Version) { $Version = (Get-Content (Join-Path $root 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json).version }
Push-Location $root
try {
    cargo test --manifest-path src-tauri/Cargo.toml --release -- --test-threads=1
    npm run build
    npx tauri build --bundles nsis
    $installer = Join-Path $root "src-tauri\target\release\bundle\nsis\最强软件2_${Version}_x64-setup.exe"
    if (-not (Test-Path $installer)) { throw "installer not found: $installer" }
    $hash = (Get-FileHash $installer -Algorithm SHA256).Hash
    $shaPath = Join-Path $root "releases\最强软件2_${Version}_x64-setup.exe.sha256"
    "$hash  $(Split-Path $installer -Leaf)" | Set-Content $shaPath -Encoding UTF8
    $notes = Join-Path $root "releases\最强软件2_${Version}-notes.md"
    @("# 最强软件2 $Version", "", "- Windows x64 NSIS installer", "- SHA-256: `$hash`") | Set-Content $notes -Encoding UTF8
    if ($CreateTag) { git tag -a "v$Version" -m "最强软件2 $Version"; git push origin "v$Version" }
    if ($CreateGitHubRelease) {
        if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw 'GitHub CLI (gh) is required for release creation' }
        gh release create "v$Version" $installer $shaPath $notes --title "最强软件2 $Version" --notes-file $notes
    }
    Write-Host "release ready: $Version`nsha256: $hash" -ForegroundColor Green
} finally { Pop-Location }
