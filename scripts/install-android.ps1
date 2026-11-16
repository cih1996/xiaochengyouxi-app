param([Parameter(Mandatory=$true)][string]$Device, [switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
if (-not $SkipBuild) {
    Push-Location (Join-Path $projectRoot 'android')
    try { & '.\build-debug.ps1' } finally { Pop-Location }
}
$apk = Join-Path $projectRoot 'android\build-output\app\outputs\apk\debug\app-debug.apk'
if (-not (Test-Path $apk)) { throw "APK 不存在：$apk" }
& adb -s $Device reverse tcp:38180 tcp:38180
if ($LASTEXITCODE -ne 0) { throw 'adb reverse 失败。' }
& adb -s $Device install -r $apk
if ($LASTEXITCODE -ne 0) { throw 'APK 安装失败。' }
& adb -s $Device shell am start -n 'com.lingzidev.xiaochengyouxi/.MainActivity'
if ($LASTEXITCODE -ne 0) { throw 'App 启动失败。' }
