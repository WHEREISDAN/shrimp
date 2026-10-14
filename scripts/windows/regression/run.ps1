<#
.SYNOPSIS
  The Windows regression walkthrough (AUD-02), or with "latency" the
  key-release-to-insert measurement (AUD-03), against Shrimp Dev.

.DESCRIPTION
  Builds shrimp-helper.exe and ShrimpRegressionTarget.exe with the .NET Framework
  compiler into a temporary folder, saves Shrimp Dev's data (except its models),
  the clipboard and the window in front, starts the guard, launches Shrimp Dev
  (Whisper on the GPU unless -Cpu) and the target app, and runs the walkthrough.
  -MuteSpeakers mutes the speakers meanwhile, as the cues play many times.
  Afterwards, whatever happened, it quits both apps, puts the data, the
  clipboard, the speakers and the window in front back, deletes the
  clipboard-history entries made during the run, and deletes the temporary
  folder.

  The guard stops the run the moment the keyboard or mouse is used (input that
  wasn't injected), or another app comes to the front. Run it in the interactive
  desktop session; see docs/DEVELOPMENT.md.

.EXAMPLE
  scripts/windows/regression/run.ps1
  scripts/windows/regression/run.ps1 -Only core,CMD-01 -Report C:\out.json
  scripts/windows/regression/run.ps1 latency -Runs 10 -Model qwen2.5-coder:7b
  scripts/windows/regression/run.ps1 -Extra C:\checks\terminals.mjs -Only extra
#>
param(
    [Parameter(Position = 0)][ValidateSet('walkthrough', 'latency')][string]$Mode = 'walkthrough',
    [switch]$Build,
    [switch]$Cpu,
    [switch]$MuteSpeakers,
    [string]$Only,
    [string]$Report,
    [int]$Runs = 10,
    [string]$Model = '',
    [string]$Extra,
    [string]$KeyLog
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$devApp = Join-Path $root 'scripts\windows\dev-app.ps1'
$data = Join-Path $env:APPDATA 'com.shrimp.desktop.dev'
if (-not $Cpu -and -not $env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR = Join-Path $env:SystemDrive 'st'
}
$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'src-tauri\target' }
$exe = Join-Path $targetDir 'debug\shrimp.exe'
$work = Join-Path ([IO.Path]::GetTempPath()) ('shrimp-regression-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$helper = Join-Path $work 'shrimp-helper.exe'
$targetApp = Join-Path $work 'ShrimpRegressionTarget.exe'
$abort = Join-Path $work 'abort.txt'

function Invoke-DevApp {
    & powershell -NoProfile -ExecutionPolicy Bypass -File $devApp @args
    if ($LASTEXITCODE -ne 0) { throw "dev-app.ps1 $args failed ($LASTEXITCODE)" }
}

function Invoke-Helper {
    $out = & $helper @args
    if ($LASTEXITCODE -ne 0) { throw "shrimp-helper $args failed ($LASTEXITCODE)" }
    if ($out) { $out | ConvertFrom-Json }
}

function Build-Tools {
    $framework = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319'
    $wpf = Join-Path $framework 'WPF'
    & (Join-Path $framework 'csc.exe') /nologo /optimize /platform:x64 "/out:$helper" `
        "/r:$wpf\UIAutomationClient.dll" "/r:$wpf\UIAutomationTypes.dll" "/r:$wpf\WindowsBase.dll" `
        /r:System.Web.Extensions.dll (Join-Path $PSScriptRoot 'helper.cs')
    if ($LASTEXITCODE -ne 0) { throw 'could not build the helper' }
    & (Join-Path $framework 'csc.exe') /nologo /optimize /platform:x64 /target:winexe "/out:$targetApp" `
        (Join-Path $PSScriptRoot 'target.cs')
    if ($LASTEXITCODE -ne 0) { throw 'could not build the target app' }
}

# Clipboard history through WinRT, to delete what the run added (Win+V).
function Get-ClipboardHistory {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $null = [Windows.ApplicationModel.DataTransfer.Clipboard, Windows.ApplicationModel.DataTransfer, ContentType = WindowsRuntime]
    $asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
    } | Select-Object -First 1
    $operation = [Windows.ApplicationModel.DataTransfer.Clipboard]::GetHistoryItemsAsync()
    $task = $asTask.MakeGenericMethod([Windows.ApplicationModel.DataTransfer.ClipboardHistoryItemsResult]).Invoke($null, @($operation))
    if (-not $task.Wait(5000) -or $task.Result.Status -ne 'Success') { return @() }
    @($task.Result.Items)
}

New-Item -ItemType Directory -Path $work | Out-Null
$saved = Join-Path $work 'data'
$state = @{ data = $false; clipboard = $false; history = $null; foreground = $null; watcher = $null; muted = $null }
$status = 1
try {
    Write-Host "Building the helper and the target app in $work..."
    Build-Tools
    if ($Build -or -not (Test-Path $exe)) {
        if ($Cpu) { Invoke-DevApp build } else { Invoke-DevApp build -Vulkan }
    }
    if (-not (Test-Path (Join-Path $data 'models'))) {
        throw "Shrimp Dev has no data yet: launch it once, finish onboarding and download the default model."
    }

    Invoke-DevApp quit
    New-Item -ItemType Directory -Path $saved | Out-Null
    Get-ChildItem -LiteralPath $data -Force | Where-Object Name -ne 'models' |
        Copy-Item -Destination $saved -Recurse -Force
    $state.data = $true
    Invoke-Helper clipboard-save (Join-Path $work 'clipboard.json') | Out-Null
    $state.clipboard = $true
    $state.history = @(Get-ClipboardHistory | ForEach-Object { $_.Id })
    $state.foreground = (Invoke-Helper frontmost).hwnd
    if ($MuteSpeakers) {
        try { $state.muted = (Invoke-Helper output-mute true).muted } catch { Write-Warning "could not mute the speakers: $_" }
    }

    $watch = @('watch', "`"$abort`"")
    if ($KeyLog) { $watch += "`"$KeyLog`"" }
    $state.watcher = Start-Process -FilePath $helper -ArgumentList $watch -PassThru -WindowStyle Hidden
    Start-Sleep -Milliseconds 500
    $silence = Join-Path $root 'src-tauri\tests\fixtures\audio\silence.wav'
    Invoke-DevApp open "SHRIMP_AUDIO_SOURCE=wav:$silence"
    Start-Process -FilePath $targetApp
    Start-Sleep -Seconds 3

    $env:SHRIMP_REGRESSION_ABORT = $abort
    $env:SHRIMP_DEV_EXE = $exe
    $arguments = @((Join-Path $PSScriptRoot 'walkthrough.mjs'), '--work', $work)
    if ($Mode -eq 'latency') { $arguments += @('latency', '--runs', $Runs, '--model', $Model) }
    if ($Only) { $arguments += @('--only', $Only) }
    if ($Report) { $arguments += @('--report', $Report) }
    if ($Extra) { $arguments += @('--extra', $Extra) }
    & node @arguments
    $status = $LASTEXITCODE
} catch {
    Write-Host "Failed: $_"
} finally {
    $ErrorActionPreference = 'Continue'
    if ($state.watcher) { Stop-Process -Id $state.watcher.Id -Force -ErrorAction SilentlyContinue }
    if (Test-Path $abort) { Write-Host "Stopped: $(Get-Content -Raw $abort)" }
    if (Get-Process -Name ShrimpRegressionTarget -ErrorAction SilentlyContinue) { & $helper focus quit | Out-Null }
    try { Invoke-DevApp quit } catch { Write-Warning $_ }
    Get-Process -Name ShrimpRegressionTarget -ErrorAction SilentlyContinue | Stop-Process -Force
    if ($state.data) {
        Get-ChildItem -LiteralPath $data -Force | Where-Object Name -ne 'models' | Remove-Item -Recurse -Force
        Get-ChildItem -LiteralPath $saved -Force | Copy-Item -Destination $data -Recurse -Force
        Write-Host "Restored the dev app's data."
    }
    try {
        if ($state.clipboard) {
            Invoke-Helper clipboard-restore (Join-Path $work 'clipboard.json') | Out-Null
            Write-Host 'Restored the clipboard.'
        }
    } catch { Write-Warning "could not restore the clipboard: $_" }
    try {
        if ($null -ne $state.history) {
            $added = @(Get-ClipboardHistory | Where-Object { $state.history -notcontains $_.Id })
            foreach ($item in $added) {
                $null = [Windows.ApplicationModel.DataTransfer.Clipboard]::DeleteItemFromHistory($item)
            }
            Write-Host "Deleted $($added.Count) clipboard-history entries the run added."
        }
    } catch { Write-Warning "could not clean clipboard history: $_" }
    try {
        if ($null -ne $state.muted) { Invoke-Helper output-mute ([string]$state.muted).ToLower() | Out-Null }
    } catch { Write-Warning "could not restore the speakers: $_" }
    if ($state.foreground) { & $helper activate $state.foreground 2>$null | Out-Null }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
exit $status
