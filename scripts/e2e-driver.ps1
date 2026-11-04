[CmdletBinding()]
param([string]$ApplicationPath = (Join-Path $PSScriptRoot '../src-tauri/target/release/mytools.exe'))
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7) { throw 'PowerShell 7+ is required' }
if (-not (Test-Path -LiteralPath $ApplicationPath)) { throw 'Build the application first: pnpm tauri build --no-bundle' }
$env:MYTOOLS_E2E_EXE = (Resolve-Path -LiteralPath $ApplicationPath).Path
$taskDriver = & (Join-Path $PSScriptRoot 'setup-e2e-driver.ps1') -ApplicationPath $env:MYTOOLS_E2E_EXE
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=0'
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $PSScriptRoot ("../src-tauri/target/e2e-profile/" + [guid]::NewGuid().ToString('N'))
Write-Host "Application: $env:MYTOOLS_E2E_EXE"
Write-Host 'Use another PowerShell terminal: pnpm test:e2e'
& tauri-driver --port 4440 --native-port 4441 --native-driver $taskDriver
exit $LASTEXITCODE
