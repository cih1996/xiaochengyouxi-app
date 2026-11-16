$ErrorActionPreference='Stop'
$env:JAVA_HOME='C:\Users\Arlo\.jdks\temurin-17'
$env:ANDROID_HOME='C:\Users\Arlo\AppData\Local\Android\Sdk'
Push-Location $PSScriptRoot
try { & "$env:USERPROFILE\.gradle\standalone\gradle-8.11.1\bin\gradle.bat" --console=plain --init-script china-mirrors.init.gradle :app:assembleDebug; if($LASTEXITCODE -ne 0){throw 'Android build failed'} } finally { Pop-Location }
