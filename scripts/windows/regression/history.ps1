<#
.SYNOPSIS
  Lists clipboard history (Win+V) as JSON, [{ "id": ..., "text": ... }], for
  the walkthrough's WIN-05 case. Items without text have an empty text.
#>
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.ApplicationModel.DataTransfer.Clipboard, Windows.ApplicationModel.DataTransfer, ContentType = WindowsRuntime]
$methods = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
}
$asTask = $methods | Select-Object -First 1

function Wait-Operation($operation, [type]$type) {
    $task = $asTask.MakeGenericMethod($type).Invoke($null, @($operation))
    if (-not $task.Wait(5000)) { throw 'clipboard history did not answer' }
    $task.Result
}

$result = Wait-Operation ([Windows.ApplicationModel.DataTransfer.Clipboard]::GetHistoryItemsAsync()) `
    ([Windows.ApplicationModel.DataTransfer.ClipboardHistoryItemsResult])
$items = foreach ($item in $result.Items) {
    $text = ''
    if ($item.Content.Contains('Text')) {
        $text = Wait-Operation ($item.Content.GetTextAsync()) ([string])
    }
    [pscustomobject]@{ id = $item.Id; text = $text }
}
ConvertTo-Json -InputObject @($items) -Compress
