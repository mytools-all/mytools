$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:GITHUB_REF_TYPE -ne 'tag') { throw 'Release builds require a GitHub Actions tag run' }
$taskVersion = (Get-Content package.json -Raw | ConvertFrom-Json).version
if ($env:GITHUB_REF_NAME -ne "v$taskVersion") { throw 'Release tag and package version differ' }
$taskCargo = Get-Content src-tauri/Cargo.toml -Raw
if ($taskCargo -notmatch ('(?m)^version = "' + [regex]::Escape($taskVersion) + '"')) { throw 'Rust package version differs' }
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) { throw 'Missing updater signing secret' }
$taskKey = Join-Path $env:RUNNER_TEMP 'mytools-release.key'
try {
    [IO.File]::WriteAllText($taskKey, $env:TAURI_SIGNING_PRIVATE_KEY)
    $taskPublic = (Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).plugins.updater.pubkey
    [IO.File]::WriteAllText("$taskKey.pub", $taskPublic)
    & ./scripts/build-update.ps1 -SigningKey $taskKey -OutputDirectory release-artifacts
    $taskExeRoot = 'src-tauri/target/release'
    if (-not (Test-Path "$taskExeRoot/mytools.exe")) { $taskExeRoot = 'src-tauri/target/x86_64-pc-windows-gnu/release' }
    node scripts/check-runtime-deps.mjs "$taskExeRoot/mytools.exe"
    if ($LASTEXITCODE -ne 0) { throw 'Runtime dependencies are incomplete' }
    $taskFiles = @("$taskExeRoot/mytools.exe", "$taskExeRoot/WebView2Loader.dll", 'LICENSE', 'THIRD_PARTY_NOTICES.txt')
    Compress-Archive -LiteralPath $taskFiles -DestinationPath "release-artifacts/MyTools_${taskVersion}_windows-x64.zip"
    $taskHashes = Get-ChildItem release-artifacts -File | Where-Object Name -ne 'SHA256SUMS.txt' | Sort-Object Name | ForEach-Object { '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Name }
    [IO.File]::WriteAllLines((Join-Path $PWD 'release-artifacts/SHA256SUMS.txt'), $taskHashes, [Text.UTF8Encoding]::new($false))
    $taskNotes = "docs/releases/$taskVersion.md"
    if (-not (Test-Path -LiteralPath $taskNotes)) { throw 'Missing release notes' }
    $taskAssets = @(Get-ChildItem release-artifacts -File | ForEach-Object FullName)
    gh release create $env:GITHUB_REF_NAME @taskAssets --repo $env:GITHUB_REPOSITORY --verify-tag --draft --title "MyTools v$taskVersion" --notes-file $taskNotes
    if ($LASTEXITCODE -ne 0) { throw 'Release draft creation failed' }
} finally {
    Remove-Item -LiteralPath $taskKey,"$taskKey.pub" -Force -ErrorAction SilentlyContinue
}
