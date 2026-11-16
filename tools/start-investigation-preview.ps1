$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
$taskExe = Join-Path $taskRoot 'server/target/debug/orange-rewards-server.exe'
$taskPreview = Start-Process -FilePath $taskExe -WorkingDirectory (Join-Path $taskRoot 'server') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $taskRoot 'output/server.log') -RedirectStandardError (Join-Path $taskRoot 'output/server-error.log')
$taskPreview.Id | Set-Content (Join-Path $taskRoot 'output/server.pid')
foreach ($taskLine in Get-Content (Join-Path $taskRoot 'private/server.env')) {
  if ($taskLine -and !$taskLine.StartsWith('#')) {
    $taskPair = $taskLine.Split('=',2)
    [Environment]::SetEnvironmentVariable($taskPair[0],$taskPair[1],'Process')
  }
}
$env:ORANGE_ENV='local'
$env:ORANGE_AD_MODE='mock'
$env:ORANGE_BIND='127.0.0.1:38182'
$env:ORANGE_DB_PATH=Join-Path $taskRoot 'private/user-investigation-test.sqlite3'
$taskTest = Start-Process -FilePath $taskExe -WorkingDirectory (Join-Path $taskRoot 'server') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $taskRoot 'output/risk-server.log') -RedirectStandardError (Join-Path $taskRoot 'output/risk-server-error.log')
$taskTest.Id | Set-Content (Join-Path $taskRoot 'output/risk-server.pid')
$env:ORANGE_AD_MODE='csj'
$env:ORANGE_BIND='127.0.0.1:38183'
$env:ORANGE_DB_PATH=Join-Path $taskRoot 'private/rain-test.sqlite3'
$taskRainTest = Start-Process -FilePath $taskExe -WorkingDirectory (Join-Path $taskRoot 'server') -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $taskRoot 'output/rain-server.log') -RedirectStandardError (Join-Path $taskRoot 'output/rain-server-error.log')
$taskRainTest.Id | Set-Content (Join-Path $taskRoot 'output/rain-server.pid')
Write-Output "Preview PID $($taskPreview.Id); risk test PID $($taskTest.Id); rain test PID $($taskRainTest.Id)"
