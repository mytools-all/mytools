$msedgedriver = "$env:USERPROFILE\AppData\Local\Microsoft\WinGet\Packages\Microsoft.EdgeDriver_Microsoft.Winget.Source_8wekyb3d8bbwe\msedgedriver.exe"
if (-not (Test-Path -LiteralPath $msedgedriver)) {
    Write-Host "[MyTools] msedgedriver.exe not found at $msedgedriver" -ForegroundColor Red
    Write-Host "[MyTools]     install via: winget install Microsoft.EdgeDriver --accept-source-agreements --accept-package-agreements"
    exit 1
}
$env:PATH = "$env:USERPROFILE\AppData\Local\Microsoft\WinGet\Packages\Microsoft.EdgeDriver_Microsoft.Winget.Source_8wekyb3d8bbwe;$env:PATH"
$env:MYTOOLS_E2E_EXE = "$env:LOCALAPPDATA\MyTools\mytools.exe"
if (-not (Test-Path -LiteralPath $env:MYTOOLS_E2E_EXE)) {
    Write-Host "[MyTools] mytools.exe not found at $env:MYTOOLS_E2E_EXE" -ForegroundColor Red
    Write-Host "[MyTools]     build via: pnpm tauri build --no-bundle"
    exit 1
}
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=0"
$taskRepoRoot = Split-Path $PSScriptRoot -Parent
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $taskRepoRoot ("src-tauri/target/cdp/e2e-" + [guid]::NewGuid().ToString("N"))
Write-Host "[MyTools] CDP profile: $env:WEBVIEW2_USER_DATA_FOLDER"
Write-Host "[MyTools] starting tauri-driver on port 4440 (msedgedriver path: $msedgedriver)" -ForegroundColor Cyan
& tauri-driver --port 4440 --native-port 4441