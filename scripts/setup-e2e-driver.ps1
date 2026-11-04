[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ApplicationPath,
    [string]$DriverDirectory = (Join-Path $PSScriptRoot '../src-tauri/target/e2e-driver'),
    [switch]$InspectOnly
)
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7) { throw 'PowerShell 7+ is required' }
if (-not $IsWindows -or [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'X64') { throw 'The E2E driver setup requires Windows x64' }
. (Join-Path $PSScriptRoot 'webview2-runtime.ps1')
$taskApp = (Resolve-Path -LiteralPath $ApplicationPath).Path
$taskLoader = Join-Path (Split-Path $taskApp -Parent) 'WebView2Loader.dll'
$taskRuntime = Get-WebView2RuntimeVersion -LoaderPath $taskLoader
Write-Host "WebView2 Runtime selected by the app loader: $taskRuntime"
if ($InspectOnly) { return $taskRuntime }
New-Item -ItemType Directory -Force -Path $DriverDirectory | Out-Null
$taskDriverRoot = (Resolve-Path -LiteralPath $DriverDirectory).Path
$taskDriver = Join-Path $taskDriverRoot 'msedgedriver.exe'
if (Test-Path -LiteralPath $taskDriver) {
    $taskOutput = & $taskDriver --version
    if ($LASTEXITCODE -eq 0 -and $taskOutput -match '(\d+\.\d+\.\d+\.\d+)' -and (Test-WebViewDriverVersion $taskRuntime $Matches[1])) { return $taskDriver }
}
$taskArchive = Join-Path $taskDriverRoot "edgedriver-$taskRuntime.zip"
Invoke-WebRequest -Uri "https://msedgedriver.microsoft.com/$taskRuntime/edgedriver_win64.zip" -OutFile $taskArchive -TimeoutSec 90
Expand-Archive -LiteralPath $taskArchive -DestinationPath $taskDriverRoot -Force
$taskOutput = & $taskDriver --version
if ($LASTEXITCODE -ne 0 -or $taskOutput -notmatch '(\d+\.\d+\.\d+\.\d+)') { throw 'Cannot read the downloaded driver version' }
if (-not (Test-WebViewDriverVersion $taskRuntime $Matches[1])) { throw "Driver/runtime mismatch: $taskOutput / $taskRuntime" }
Write-Host "Matched WebView2 driver: $taskOutput"
return $taskDriver
