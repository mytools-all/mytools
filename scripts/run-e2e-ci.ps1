[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ApplicationPath,
    [Parameter(Mandatory)][string]$DriverPath,
    [string]$ArtifactsDirectory = 'e2e-artifacts'
)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'This collector is restricted to disposable GitHub Actions runners' }
$taskApp = (Resolve-Path -LiteralPath $ApplicationPath).Path
$taskDriverPath = (Resolve-Path -LiteralPath $DriverPath).Path
New-Item -ItemType Directory -Force -Path $ArtifactsDirectory | Out-Null
$taskArtifacts = (Resolve-Path -LiteralPath $ArtifactsDirectory).Path
$taskStarted = Get-Date
$taskProfileTemp = Join-Path $env:RUNNER_TEMP ('mytools-webdriver-' + [guid]::NewGuid().ToString('N'))
$taskVariables = @('MYTOOLS_E2E_EXE','MYTOOLS_E2E_ARTIFACTS','MYTOOLS_E2E_DRIVER','TAURI_WEBVIEW_AUTOMATION','WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS','WEBVIEW2_USER_DATA_FOLDER','CHROME_LOG_FILE','RUST_LOG','TEMP','TMP')
$taskPrevious = @{}
foreach ($taskVariable in $taskVariables) { $taskPrevious[$taskVariable] = [Environment]::GetEnvironmentVariable($taskVariable, 'Process') }
$taskDriver = $null
$taskMonitor = $null
try {
    $env:MYTOOLS_E2E_EXE = $taskApp
    $env:MYTOOLS_E2E_ARTIFACTS = $taskArtifacts
    $env:MYTOOLS_E2E_DRIVER = 'edge'
    $env:TAURI_WEBVIEW_AUTOMATION = 'true'
    $env:WEBVIEW2_USER_DATA_FOLDER = $null
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--enable-logging --v=1'
    $env:CHROME_LOG_FILE = Join-Path $taskArtifacts 'webview2.log'
    $env:RUST_LOG = 'info,mytools=debug'
    $env:TEMP = $taskProfileTemp
    $env:TMP = $env:TEMP
    New-Item -ItemType Directory -Force -Path $env:TEMP | Out-Null
    @{ application=$taskApp; driver=$taskDriverPath; driverVersion=(& $taskDriverPath --version); started=$taskStarted.ToUniversalTime().ToString('o'); profileOverride='unset'; runtimeArguments=$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS } | ConvertTo-Json | Set-Content (Join-Path $taskArtifacts 'startup.json')
    $taskMonitor = Start-Job -ArgumentList $taskArtifacts -ScriptBlock {
        param($outputDirectory)
        for ($sample=0; $sample -lt 180; $sample++) {
            try {
                $processes = @(Get-CimInstance Win32_Process -Filter "Name='mytools.exe' OR Name='msedgewebview2.exe' OR Name='msedgedriver.exe'" | Select-Object Name,ProcessId,ParentProcessId,ExecutablePath,CommandLine)
                @{time=[DateTime]::UtcNow.ToString('o'); processes=$processes} | ConvertTo-Json -Depth 4 -Compress | Add-Content (Join-Path $outputDirectory 'processes.jsonl')
            } catch { $_.ToString() | Add-Content (Join-Path $outputDirectory 'collector-errors.log') }
            Start-Sleep -Seconds 1
        }
    }
    $taskPolicies = foreach ($taskPolicyRoot in @('HKCU:\Software\Policies\Microsoft\Edge\WebView2','HKLM:\Software\Policies\Microsoft\Edge\WebView2')) {
        if (Test-Path -LiteralPath $taskPolicyRoot) {
            Get-ItemProperty -LiteralPath $taskPolicyRoot
            Get-ChildItem -LiteralPath $taskPolicyRoot -Recurse | Get-ItemProperty
        }
    }
    ConvertTo-Json -InputObject @($taskPolicies) -Depth 5 | Set-Content (Join-Path $taskArtifacts 'webview2-policies.json')
    $taskDriverLog = Join-Path $taskArtifacts 'edgedriver.log'
    $taskDriver = Start-Process -FilePath $taskDriverPath -ArgumentList @('--port=4440','--verbose',('"--log-path=' + $taskDriverLog + '"')) -RedirectStandardOutput (Join-Path $taskArtifacts 'edgedriver-stdout.log') -RedirectStandardError (Join-Path $taskArtifacts 'edgedriver-stderr.log') -WindowStyle Hidden -PassThru
    $taskReady = $false
    for ($attempt=0; $attempt -lt 40; $attempt++) {
        if ($taskDriver.HasExited) { throw 'EdgeDriver exited before becoming ready' }
        try { Invoke-RestMethod http://127.0.0.1:4440/status -NoProxy -TimeoutSec 1 | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $taskArtifacts 'driver-status.json'); $taskReady=$true; break } catch { Start-Sleep -Milliseconds 250 }
    }
    if (-not $taskReady) { throw 'EdgeDriver readiness timed out' }
    & pnpm test:e2e
    if ($LASTEXITCODE -ne 0) { throw "E2E smoke failed (exit $LASTEXITCODE)" }
} finally {
    if ($taskMonitor) { Stop-Job $taskMonitor; Remove-Job $taskMonitor }
    if ($taskDriver -and -not $taskDriver.HasExited) { Stop-Process -Id $taskDriver.Id -Force -ErrorAction Continue }
    try {
        $taskLogRoot = Join-Path $env:LOCALAPPDATA 'MyTools/data/logs'
        if (Test-Path -LiteralPath $taskLogRoot) { Copy-Item -LiteralPath $taskLogRoot -Destination (Join-Path $taskArtifacts 'mytools-logs') -Recurse -Force }
        Get-ChildItem -LiteralPath $taskProfileTemp -Recurse -File -ErrorAction SilentlyContinue | Select-Object FullName,Length | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $taskArtifacts 'profile-files.json')
        $taskPorts = @(Get-ChildItem -LiteralPath $taskProfileTemp -Filter DevToolsActivePort -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object { @{path=$_.FullName; content=(Get-Content -LiteralPath $_.FullName -Raw)} })
        ConvertTo-Json -InputObject $taskPorts -Depth 3 | Set-Content (Join-Path $taskArtifacts 'debug-ports.json')
        Get-WinEvent -FilterHashtable @{LogName='Application'; StartTime=$taskStarted; Level=@(1,2)} -ErrorAction SilentlyContinue | Where-Object { $_.Message -match 'mytools|msedgewebview2|msedgedriver' } | Select-Object TimeCreated,Id,ProviderName,Message | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $taskArtifacts 'windows-errors.json')
    } catch { $_.ToString() | Add-Content (Join-Path $taskArtifacts 'collector-errors.log') }
    foreach ($taskVariable in $taskVariables) { [Environment]::SetEnvironmentVariable($taskVariable, $taskPrevious[$taskVariable], 'Process') }
}
