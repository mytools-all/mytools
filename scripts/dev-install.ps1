[CmdletBinding()]
param(
    [ValidateSet("Rebuild", "Skip", "Clean")]
    [string]$Mode = "Rebuild",

    [switch]$Cdp,

    [switch]$NoStart
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -lt 7) {
    Write-Host "dev-install.ps1 requires PowerShell 7+ (pwsh). Current: $($PSVersionTable.PSVersion)" -ForegroundColor Red
    exit 2
}
$repoRoot = (Resolve-Path "$PSScriptRoot/..").Path
$srcTauri = Join-Path $repoRoot "src-tauri"
$releaseExe = Join-Path $srcTauri "target/release/mytools.exe"
$installDir = Join-Path $env:LOCALAPPDATA "MyTools"
$installExe = Join-Path $installDir "mytools.exe"
$dllSrc = Join-Path $srcTauri "target/release/WebView2Loader.dll"
$dllDst = Join-Path $installDir "WebView2Loader.dll"

function Write-Step([string]$label, [double]$seconds) {
    $secs = [int]$seconds
    $color = if ($secs -lt 5) { "Green" } elseif ($secs -lt 30) { "Yellow" } else { "Red" }
    Write-Host ("[{0}] {1} ({2}s)" -f (Get-Date -Format "HH:mm:ss"), $label, $secs) -ForegroundColor $color
}

function Stop-MyTools {
    $taskExePaths = @($installExe, $releaseExe, (Join-Path $srcTauri "target/debug/mytools.exe"))
    $taskRoots = @(Get-Process -Name mytools -ErrorAction SilentlyContinue | Where-Object {
        $_.Path -and $taskExePaths -contains $_.Path
    })
    if ($taskRoots.Count -eq 0) { return }
    $taskOwned = [System.Collections.Generic.HashSet[int]]::new()
    foreach ($taskProcess in $taskRoots) { [void]$taskOwned.Add($taskProcess.Id) }
    try {
        $taskSnapshot = @(Get-CimInstance Win32_Process -ErrorAction Stop)
        do {
            $taskAdded = $false
            foreach ($taskProcess in $taskSnapshot) {
                if ($taskProcess.Name -eq "msedgewebview2.exe" -and $taskOwned.Contains([int]$taskProcess.ParentProcessId)) {
                    if ($taskOwned.Add([int]$taskProcess.ProcessId)) { $taskAdded = $true }
                }
            }
        } while ($taskAdded)
    } catch {
        Write-Warning "Cannot inspect WebView2 child processes; stopping only MyTools."
    }
    $taskRootIds = @($taskRoots | ForEach-Object { $_.Id })
    foreach ($taskProcessId in $taskOwned) {
        if ($taskRootIds -notcontains $taskProcessId) {
            Stop-Process -Id $taskProcessId -Force -ErrorAction SilentlyContinue
        }
    }
    foreach ($taskProcess in $taskRoots) {
        Stop-Process -Id $taskProcess.Id -Force -ErrorAction SilentlyContinue
    }
    Write-Host ("stopped MyTools and {0} owned process(es)" -f $taskOwned.Count)
}

function Sync-Dll {
    if (-not (Test-Path -LiteralPath $dllSrc)) {
        Write-Host "WebView2Loader.dll not found at $dllSrc (run a full pnpm tauri build once)" -ForegroundColor Red
        return $false
    }
    if (-not (Test-Path -LiteralPath $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }
    $needs = $true
    if (Test-Path -LiteralPath $dllDst) {
        $srcHash = (Get-FileHash -LiteralPath $dllSrc -Algorithm SHA256).Hash
        $dstHash = (Get-FileHash -LiteralPath $dllDst -Algorithm SHA256).Hash
        if ($srcHash -eq $dstHash) { $needs = $false }
    }
    if ($needs) {
        Copy-Item -LiteralPath $dllSrc -Destination $dllDst -Force
        Write-Host "synced WebView2Loader.dll -> $dllDst"
    } else {
        Write-Host "WebView2Loader.dll already in sync"
    }
    return $true
}

$total = [System.Diagnostics.Stopwatch]::StartNew()
try {
    if ($Mode -eq "Clean") {
        Push-Location $srcTauri
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        cargo clean -p mytools
        $sw.Stop()
        Write-Step "cargo clean -p mytools" $sw.Elapsed.TotalSeconds
        Pop-Location
    }

    if ($Mode -ne "Skip") {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        pnpm tauri build --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "pnpm tauri build --no-bundle failed (exit $LASTEXITCODE)" }
        $sw.Stop()
        Write-Step "pnpm tauri build --no-bundle" $sw.Elapsed.TotalSeconds
    } else {
        if (-not (Test-Path -LiteralPath $releaseExe)) {
            throw "release exe not found at $releaseExe; run Rebuild first"
        }
        Write-Host "[skip] using existing $releaseExe"
    }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Stop-MyTools
    Start-Sleep -Milliseconds 200
    if (-not (Test-Path -LiteralPath $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }
    Copy-Item -LiteralPath $releaseExe -Destination $installExe -Force
    [void](Sync-Dll)
    $sw.Stop()
    Write-Step "deploy to $installExe" $sw.Elapsed.TotalSeconds

    if (-not $NoStart) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        if ($Cdp) {
            & (Join-Path $PSScriptRoot "start-cdp.ps1") -ExePath $installExe
        } else {
            Start-Process -FilePath $installExe -WorkingDirectory $installDir -WindowStyle Hidden | Out-Null
        }
        $sw.Stop()
        Write-Step "launched mytools" $sw.Elapsed.TotalSeconds
    }

    $total.Stop()
    Write-Host ("=== done in {0}s ===" -f [int]$total.Elapsed.TotalSeconds) -ForegroundColor Green
}
catch {
    Pop-Location -ErrorAction SilentlyContinue
    Write-Host ("dev-install failed: {0}" -f $_.Exception.Message) -ForegroundColor Red
    exit 1
}
