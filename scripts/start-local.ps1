param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$output = Join-Path $projectRoot 'output'
New-Item -ItemType Directory -Force $output | Out-Null
if (Get-NetTCPConnection -LocalPort 38180 -State Listen -ErrorAction SilentlyContinue) {
    throw '端口 38180 已被使用。请直接打开 http://127.0.0.1:38180，或先关闭原服务。'
}
if (-not $SkipBuild) {
    Push-Location (Join-Path $projectRoot 'web')
    try {
        if (-not (Test-Path 'node_modules')) {
            & npm.cmd ci --userconfig (Join-Path $projectRoot 'tools\.npmrc')
            if ($LASTEXITCODE -ne 0) { throw 'npm ci 失败，请查看上方原始输出。' }
        }
        & npm.cmd run build
        if ($LASTEXITCODE -ne 0) { throw '前端构建失败，请查看上方原始输出。' }
    } finally { Pop-Location }
    & cargo build --manifest-path (Join-Path $projectRoot 'server\Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw 'Rust 构建失败，请查看上方原始输出。' }
}
$binary = Join-Path $projectRoot 'server\target\debug\orange-rewards-server.exe'
$service = Start-Process -FilePath $binary -WorkingDirectory (Join-Path $projectRoot 'server') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $output 'server.log') -RedirectStandardError (Join-Path $output 'server.error.log')
$service.Id | Set-Content (Join-Path $output 'server.pid')
Write-Output "服务进程：$($service.Id)"
Write-Output 'App 预览：http://127.0.0.1:38180'
Write-Output '管理后台：http://127.0.0.1:38180/admin'
Write-Output '演示账号：13800138000 / Orange123!'
