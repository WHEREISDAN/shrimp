<#
.SYNOPSIS
  Signs one Windows binary for the installer build (bundle.windows.signCommand).

.DESCRIPTION
  Signs with signtool and the code-signing certificate whose SHA-1 thumbprint is in
  SHRIMP_WINDOWS_SIGN_THUMBPRINT (from the current user's or the machine's certificate
  store), timestamped through SHRIMP_WINDOWS_TIMESTAMP_URL (default DigiCert). Without
  a thumbprint the binary is left unsigned and the build carries on.
#>
param([Parameter(Mandatory = $true)][string]$File)
$ErrorActionPreference = 'Stop'

$thumbprint = $env:SHRIMP_WINDOWS_SIGN_THUMBPRINT
if (-not $thumbprint) {
    Write-Host "Not signing $File (SHRIMP_WINDOWS_SIGN_THUMBPRINT is not set)."
    exit 0
}
$timestamp = if ($env:SHRIMP_WINDOWS_TIMESTAMP_URL) { $env:SHRIMP_WINDOWS_TIMESTAMP_URL } else { 'http://timestamp.digicert.com' }

$signtool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $signtool) { throw 'signtool.exe not found; install the Windows SDK.' }

& $signtool.FullName sign /sha1 $thumbprint /fd sha256 /tr $timestamp /td sha256 $File
if ($LASTEXITCODE -ne 0) { throw "signtool failed for $File ($LASTEXITCODE)" }
