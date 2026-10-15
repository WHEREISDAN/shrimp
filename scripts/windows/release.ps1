<#
.SYNOPSIS
  Builds the Windows release: a per-user NSIS installer (no administrator rights) with
  Whisper on the GPU through Vulkan and the CPU as fallback, and its signed update.

.DESCRIPTION
  Needs the Vulkan SDK (VULKAN_SDK) and the updater key that the macOS release uses
  (docs/RELEASING.md): TAURI_SIGNING_PRIVATE_KEY is the path of the private key file and
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD its password. SHRIMP_UPDATE_FEED is the URL of
  latest.json, built into the app; without it the app says updates aren't configured. The
  update is expected next to latest.json, or for a GitHub Releases feed
  (.../releases/latest/download/latest.json) in the release tagged v<version>.
  Signs every binary when SHRIMP_WINDOWS_SIGN_THUMBPRINT names a code-signing certificate
  (scripts/windows/sign.ps1); without it the installer is unsigned. The installer lands in
  $env:CARGO_TARGET_DIR\release\bundle\nsis\, the update (the installer, its .sig and,
  with a feed, latest.json) in ...\bundle\updater\.

.PARAMETER Version
  Builds this version instead of the one in tauri.conf.json (test builds; a real release
  bumps the version files).

.PARAMETER Config
  A JSON file merged into the configuration last, for test builds (for example a
  throwaway updater public key).

.PARAMETER NoUpdaterKey
  Builds the installer only, without the updater key, which stays on the machine that
  keeps it. Sign the installer there as the update (docs/RELEASING.md).
#>
param([string]$Version, [string]$Config, [switch]$NoUpdaterKey)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$tauri = Join-Path $root 'node_modules\.bin\tauri.cmd'

if (-not $env:VULKAN_SDK) { throw 'Install the Vulkan SDK; VULKAN_SDK is not set.' }
$keyPath = $env:TAURI_SIGNING_PRIVATE_KEY
$keyPassword = $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
if (-not $NoUpdaterKey) {
    if (-not $keyPath -or -not (Test-Path -LiteralPath $keyPath -PathType Leaf)) {
        throw 'Set TAURI_SIGNING_PRIVATE_KEY to the updater private key file (docs/RELEASING.md).'
    }
    if (-not $keyPassword) { throw 'Set TAURI_SIGNING_PRIVATE_KEY_PASSWORD to the updater key password.' }
}
$feed = $env:SHRIMP_UPDATE_FEED
if ($feed -and $feed -notmatch '^(https://|http://(127\.0\.0\.1|localhost)[:/])') {
    throw 'SHRIMP_UPDATE_FEED must be https (http only for 127.0.0.1 or localhost).'
}
if ($Config -and -not (Test-Path -LiteralPath $Config -PathType Leaf)) { throw "No config file $Config." }
if (-not $Version) {
    $Version = (Get-Content -Raw (Join-Path $root 'src-tauri\tauri.conf.json') | ConvertFrom-Json).version
}
# MSBuild's file tracker fails on whisper.cpp's nested Vulkan build beyond MAX_PATH,
# about 230 characters below the target directory.
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $env:SystemDrive 'st' }
# A CPU baseline every x64 PC since 2015 has, instead of this machine's own instructions.
$env:GGML_NATIVE = 'OFF'
$env:GGML_AVX = 'ON'
$env:GGML_AVX2 = 'ON'
$env:GGML_BMI2 = 'ON'
$env:GGML_FMA = 'ON'
$env:GGML_F16C = 'ON'
if (-not $env:SHRIMP_WINDOWS_SIGN_THUMBPRINT) {
    Write-Warning 'SHRIMP_WINDOWS_SIGN_THUMBPRINT is not set: the installer will be unsigned.'
}
if (-not $feed) { Write-Warning "SHRIMP_UPDATE_FEED is not set: this build won't check for updates." }

# The build-time configuration: the version and the update feed.
$updater = @{}
if ($feed) {
    $updater = @{ endpoints = @($feed); dangerousInsecureTransportProtocol = $feed.StartsWith('http:') }
}
$releaseConfig = Join-Path ([IO.Path]::GetTempPath()) "shrimp-release-$PID.json"
$utf8 = New-Object Text.UTF8Encoding($false)
[IO.File]::WriteAllText($releaseConfig, (@{ version = $Version; plugins = @{ updater = $updater } } | ConvertTo-Json -Depth 5), $utf8)
$configArgs = @('--config', $releaseConfig)
if ($Config) { $configArgs += @('--config', (Resolve-Path $Config).Path) }

# The updater key stays out of the build; only the archive below is signed with it.
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY, Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
Push-Location $root
try {
    pnpm tauri build --features vulkan @configArgs
    if ($LASTEXITCODE -ne 0) { throw "build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
    Remove-Item -LiteralPath $releaseConfig
    $env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $keyPassword
}

$bundle = Join-Path $env:CARGO_TARGET_DIR 'release\bundle'
$name = "Shrimp_${Version}_x64-setup.exe"
$installer = Join-Path $bundle "nsis\$name"
if (-not (Test-Path -LiteralPath $installer)) { throw "The build left no $installer." }

# The update is the installer itself, signed with the updater key and bound to the version.
$updates = Join-Path $bundle 'updater'
if (Test-Path -LiteralPath $updates) { Remove-Item -LiteralPath $updates -Recurse -Force }
if ($NoUpdaterKey) {
    Write-Output "Installer: $installer"
    Write-Output 'Updater:   none (sign the installer with the updater key, docs/RELEASING.md)'
    Write-Output "Feed:      $(if ($feed) { $feed } else { 'none (updates are not configured)' })"
    return
}
New-Item -ItemType Directory -Path $updates | Out-Null
$update = Join-Path $updates $name
Copy-Item -LiteralPath $installer -Destination $update
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY
try {
    & $tauri signer sign --private-key-path $keyPath --app-version $Version $update | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "signing the update failed ($LASTEXITCODE)" }
} finally {
    $env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
}
if ($feed) {
    $signature = (Get-Content -Raw -LiteralPath "$update.sig").Trim()
    # Next to latest.json, or for a GitHub Releases feed, in the release tagged v<version>.
    if ($feed -match '^(https://github\.com/[^/]+/[^/]+/releases)/latest/download/[^/]+$') {
        $url = "$($Matches[1])/download/v$Version/$name"
    } else {
        $url = $feed.Substring(0, $feed.LastIndexOf('/') + 1) + $name
    }
    $latest = [ordered]@{
        version = $Version
        pub_date = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss.fffZ')
        platforms = [ordered]@{ 'windows-x86_64' = [ordered]@{ signature = $signature; url = $url } }
    }
    [IO.File]::WriteAllText((Join-Path $updates 'latest.json'), ($latest | ConvertTo-Json -Depth 5) + "`n", $utf8)
}

Write-Output "Installer: $installer"
Write-Output "Updater:   $updates"
Write-Output "Feed:      $(if ($feed) { $feed } else { 'none (updates are not configured)' })"
