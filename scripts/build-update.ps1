param(
    [Parameter(Mandatory = $true)][string]$SigningKey,
    [string]$OutputDirectory = 'release-artifacts'
)
$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path $PSScriptRoot -Parent
$keyPath = (Resolve-Path -LiteralPath $SigningKey).Path
$oldKey = $env:TAURI_SIGNING_PRIVATE_KEY
$oldPassword = $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
Push-Location $projectDirectory
try {
    $configuredKey = (Get-Content 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json).plugins.updater.pubkey
    if (-not (Test-Path -LiteralPath "$keyPath.pub") -or (Get-Content -LiteralPath "$keyPath.pub" -Raw).Trim() -ne $configuredKey) {
        throw '签名密钥公钥与应用配置不一致，停止构建'
    }
    $env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '' }
    $version = (Get-Content 'package.json' -Raw | ConvertFrom-Json).version
    $tag = "v$version"
    pnpm tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) { throw '主程序更新包构建失败' }
    $installerName = "MyTools_${version}_x64-setup.exe"
    $installer = Join-Path 'src-tauri\target\release\bundle\nsis' $installerName
    if (-not (Test-Path -LiteralPath $installer)) {
        $installer = Join-Path 'src-tauri\target\x86_64-pc-windows-gnu\release\bundle\nsis' $installerName
    }
    if (-not (Test-Path -LiteralPath $installer)) { throw '没有找到当前版本的安装包' }
    $signaturePath = "$installer.sig"
    if (-not (Test-Path -LiteralPath $signaturePath)) { throw '没有生成更新签名，停止打包' }
    $signature = (Get-Content -LiteralPath $signaturePath -Raw).Trim()
    if (-not $signature) { throw '更新签名为空，停止打包' }
    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
    Copy-Item -LiteralPath $installer, $signaturePath -Destination $OutputDirectory -Force
    $manifest = @{
        version = $version
        notes = "MyTools $version"
        pub_date = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
        platforms = @{
            'windows-x86_64' = @{
                signature = $signature
                url = "https://github.com/mytools-all/mytools/releases/download/$tag/$installerName"
            }
        }
    }
    $manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'latest.json') -Encoding utf8
    $hash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $installerName" | Set-Content -LiteralPath (Join-Path $OutputDirectory 'SHA256SUMS.txt') -Encoding utf8
    Write-Output "已生成 $tag 的安装包、签名、latest.json 和校验文件。尚未上传。"
} finally {
    $env:TAURI_SIGNING_PRIVATE_KEY = $oldKey
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $oldPassword
    Pop-Location
}
