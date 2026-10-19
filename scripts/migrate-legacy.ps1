$ErrorActionPreference = 'Stop'
$legacy = Join-Path $env:APPDATA 'com.animenext.app\chats.duckdb'
if (-not (Test-Path $legacy)) {
    Write-Host "未发现原版数据库: $legacy" -ForegroundColor Yellow
    exit 0
}
Write-Host "原版数据库存在: $legacy" -ForegroundColor Cyan
$dump = Join-Path $PSScriptRoot 'legacy-dump.json'
Write-Host @"
请先用任一方式把 chats.duckdb 导出为 JSON 并保存到:
  $dump
JSON 结构: { "conversations": [...], "messages": [...] }
推荐: 用 DuckDB 命令行执行
  INSTALL httpfs; LOAD httpfs;
  COPY (SELECT * FROM conversations) TO 'dump.json' (FORMAT JSON);
导出后运行:
  node scripts/migrate-legacy.js "$dump"
"@ -ForegroundColor Yellow
