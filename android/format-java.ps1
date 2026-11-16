$ErrorActionPreference = 'Stop'
$formatter = Join-Path $PSScriptRoot 'tools\google-java-format-1.24.0-all-deps.jar'
if (-not (Test-Path -LiteralPath $formatter)) {
    New-Item -ItemType Directory -Force (Split-Path $formatter) | Out-Null
    Invoke-WebRequest -Uri 'https://repo.maven.apache.org/maven2/com/google/googlejavaformat/google-java-format/1.24.0/google-java-format-1.24.0-all-deps.jar' -OutFile $formatter
}
$sources = @(Get-ChildItem -LiteralPath (Join-Path $PSScriptRoot 'app\src\main\java\com\lingzidev\xiaochengyouxi') -Filter '*.java' | ForEach-Object FullName)
& 'C:\Users\Arlo\.jdks\temurin-17\bin\java.exe' -jar $formatter --replace @sources
if ($LASTEXITCODE -ne 0) { throw 'Java formatting failed' }
