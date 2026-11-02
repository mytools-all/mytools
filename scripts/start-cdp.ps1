[CmdletBinding()]
param(
    [string]$ExePath,
    [ValidateRange(1, 60)][int]$TimeoutSeconds = 20
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -lt 7) { throw "PowerShell 7+ is required" }
$taskRepo = Split-Path $PSScriptRoot -Parent
if (-not $ExePath) { $ExePath = Join-Path $taskRepo "src-tauri/target/release/mytools.exe" }
$taskExe = (Resolve-Path -LiteralPath $ExePath).Path
$taskSessionDir = Join-Path $taskRepo "src-tauri/target/cdp"
$taskProfile = Join-Path $taskSessionDir ([guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $taskProfile -Force | Out-Null
$taskSessionFile = Join-Path $taskRepo "src-tauri/target/cdp-session.json"
$taskPreviousArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$taskPreviousProfile = $env:WEBVIEW2_USER_DATA_FOLDER
$taskProcess = $null
try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=0"
    $env:WEBVIEW2_USER_DATA_FOLDER = $taskProfile
    $taskProcess = Start-Process -FilePath $taskExe -WorkingDirectory (Split-Path $taskExe) -WindowStyle Hidden -PassThru
    $taskDeadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $taskActiveFiles = @((Join-Path $taskProfile "EBWebView/DevToolsActivePort"), (Join-Path $taskProfile "DevToolsActivePort"))
    $taskEndpoint = $null
    while ([DateTime]::UtcNow -lt $taskDeadline) {
        if ($taskProcess.HasExited) { throw "MyTools exited before CDP became ready (exit $($taskProcess.ExitCode))" }
        foreach ($taskFile in $taskActiveFiles) {
            if (-not (Test-Path -LiteralPath $taskFile)) { continue }
            $taskLines = @(Get-Content -LiteralPath $taskFile)
            if ($taskLines.Count -lt 2 -or $taskLines[0] -notmatch '^\d+$' -or $taskLines[1] -notmatch '^/devtools/browser/[a-zA-Z0-9-]+$') { continue }
            $taskActualPort = [int]$taskLines[0]
            if ($taskActualPort -lt 1 -or $taskActualPort -gt 65535) { continue }
            try {
                $taskVersion = Invoke-RestMethod -Uri "http://127.0.0.1:$taskActualPort/json/version" -NoProxy -TimeoutSec 1
                if (([uri]$taskVersion.webSocketDebuggerUrl).AbsolutePath -eq $taskLines[1]) {
                    $taskEndpoint = "http://127.0.0.1:$taskActualPort"
                    break
                }
            } catch {}
        }
        if ($taskEndpoint) { break }
        Start-Sleep -Milliseconds 100
    }
    if (-not $taskEndpoint) { throw "CDP did not start; check WebView2 flags and run from a non-elevated terminal" }
    @{ profile = $taskProfile; pid = $taskProcess.Id; endpoint = $taskEndpoint } | ConvertTo-Json | Set-Content -LiteralPath $taskSessionFile -Encoding utf8NoBOM
    Write-Host "CDP ready: $taskEndpoint (PID $($taskProcess.Id))"
    Write-Host "Use: node scripts/cdp-eval.mjs <expression>"
} catch {
    if ($taskProcess -and -not $taskProcess.HasExited) { Stop-Process -Id $taskProcess.Id -ErrorAction SilentlyContinue }
    throw
} finally {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $taskPreviousArgs
    $env:WEBVIEW2_USER_DATA_FOLDER = $taskPreviousProfile
}
