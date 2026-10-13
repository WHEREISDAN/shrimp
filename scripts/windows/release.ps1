<#
.SYNOPSIS
  Builds the Windows installer: a per-user NSIS setup (no administrator rights) with
  Whisper on the GPU through Vulkan and the CPU as fallback.

.DESCRIPTION
  Needs the Vulkan SDK (VULKAN_SDK). Signs every binary when
  SHRIMP_WINDOWS_SIGN_THUMBPRINT names a code-signing certificate (scripts/windows/sign.ps1);
  without it the installer is unsigned. The setup lands in
  $env:CARGO_TARGET_DIR\release\bundle\nsis\.
#>
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')

if (-not $env:VULKAN_SDK) { throw 'Install the Vulkan SDK; VULKAN_SDK is not set.' }
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

Push-Location $root
try {
    pnpm tauri build --features vulkan
    if ($LASTEXITCODE -ne 0) { throw "build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}
Get-ChildItem (Join-Path $env:CARGO_TARGET_DIR 'release\bundle\nsis') -Filter '*.exe'
