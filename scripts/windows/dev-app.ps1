<#
.SYNOPSIS
  Builds, opens and quits the Windows debug app "Shrimp Dev".

.DESCRIPTION
  -Vulkan builds Whisper with the GPU backend (needs the Vulkan SDK) into a short target
  directory, because MSBuild's file tracker fails beyond MAX_PATH; set CARGO_TARGET_DIR to
  choose it. Arguments of the form NAME=value become environment variables of the app.

.EXAMPLE
  scripts/windows/dev-app.ps1 build -Vulkan
  scripts/windows/dev-app.ps1 open --background SHRIMP_AUDIO_SOURCE=wav:C:\speech.wav
  scripts/windows/dev-app.ps1 quit
#>
param(
    [Parameter(Position = 0)][ValidateSet('build', 'open', 'quit')][string]$Command = 'open',
    [switch]$Vulkan,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest = @()
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
if ($Vulkan -and -not $env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR = Join-Path $env:SystemDrive 'st'
}
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'src-tauri\target' }
$exe = Join-Path $target 'debug\shrimp.exe'

function Invoke-Build {
    Push-Location $root
    try {
        $features = if ($Vulkan) { @('--features', 'vulkan') } else { @() }
        pnpm tauri build --debug --no-bundle --config src-tauri/tauri.conf.dev.json @features
        if ($LASTEXITCODE -ne 0) { throw "build failed ($LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
}

function Invoke-Quit {
    if (-not (Get-Process -Name shrimp -ErrorAction SilentlyContinue)) { return }
    # A second launch with --quit asks the running instance to quit (debug builds).
    Start-Process -FilePath $exe -ArgumentList '--quit' -Wait
    $deadline = (Get-Date).AddSeconds(10)
    while ((Get-Process -Name shrimp -ErrorAction SilentlyContinue) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 200
    }
}

switch ($Command) {
    'build' { Invoke-Build }
    'quit' { Invoke-Quit }
    'open' {
        if (-not (Test-Path $exe)) { Invoke-Build }
        Invoke-Quit
        $arguments = @()
        foreach ($item in $Rest) {
            if ($item -match '^([A-Z_][A-Z0-9_]*)=(.*)$') {
                Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2]
            } else {
                $arguments += $item
            }
        }
        if ($arguments.Count -gt 0) {
            Start-Process -FilePath $exe -ArgumentList $arguments
        } else {
            Start-Process -FilePath $exe
        }
    }
}
