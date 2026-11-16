$ErrorActionPreference='Stop'
$taskRoot=Split-Path $PSScriptRoot -Parent
$taskExe=Join-Path $taskRoot 'server/target/debug/orange-rewards-server.exe'
$env:ORANGE_BIND='127.0.0.1:38180'
$taskPreview=Start-Process -FilePath $taskExe -WorkingDirectory (Join-Path $taskRoot 'server') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $taskRoot 'output/server.log') -RedirectStandardError (Join-Path $taskRoot 'output/server-error.log')
$taskPreview.Id | Set-Content (Join-Path $taskRoot 'output/server.pid')
foreach ($taskLine in Get-Content (Join-Path $taskRoot 'private/server.env')) {
  if ($taskLine -and !$taskLine.StartsWith('#')) {
    $taskPair=$taskLine.Split('=',2)
    [Environment]::SetEnvironmentVariable($taskPair[0],$taskPair[1],'Process')
  }
}
$taskSdk=Get-Content (Join-Path $taskRoot 'private/device-trust-test.json') -Raw | ConvertFrom-Json
$env:ORANGE_DEVICE_TRUST_APP_ID=$taskSdk.app_id
$env:ORANGE_DEVICE_TRUST_APP_KEY=$taskSdk.app_key
$env:ORANGE_DEVICE_TRUST_URL=$taskSdk.base_url
$env:ORANGE_DEVICE_TRUST_PINS=$taskSdk.spki_pins -join ','
$env:ORANGE_DEVICE_TRUST_MODE='enforce'
$env:ORANGE_ENV='local'
$env:ORANGE_AD_MODE='csj'
$env:ORANGE_BIND='127.0.0.1:38183'
$env:ORANGE_DB_PATH=Join-Path $taskRoot 'private/sdk05-test.sqlite3'
$taskTest=Start-Process -FilePath $taskExe -WorkingDirectory (Join-Path $taskRoot 'server') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $taskRoot 'output/sdk05-server.log') -RedirectStandardError (Join-Path $taskRoot 'output/sdk05-server-error.log')
$taskTest.Id | Set-Content (Join-Path $taskRoot 'output/sdk05-server.pid')
Write-Output "Preview PID $($taskPreview.Id); isolated SDK 0.5 test PID $($taskTest.Id)"
