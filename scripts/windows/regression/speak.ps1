<#
.SYNOPSIS
  Speaks text into a 16 kHz mono 16-bit WAV with Windows' own text to speech,
  for the regression walkthrough's spoken fixtures.

.DESCRIPTION
  The voice speaks a little fast (-Rate 2), so the sentences fit the holds the
  cases use, which were timed on macOS's voice. -MaxSeconds speeds it up further
  until the speech fits, for the latency fixture that a 5-second hold must cover.
  -SilenceSeconds of silence follow the speech: the voice stops right at the last
  word, and Whisper drops words at an abrupt end. Prints the length of the speech
  in seconds.
#>
param(
    [Parameter(Mandatory = $true)][string]$Text,
    [Parameter(Mandatory = $true)][string]$Path,
    [int]$Rate = 2,
    [double]$MaxSeconds = 0,
    [double]$SilenceSeconds = 0.8
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(
    16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
$voice = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
    foreach ($rate in $Rate..10) {
        $voice.Rate = $rate
        $voice.SetOutputToWaveFile($Path, $format)
        $voice.Speak($Text)
        $voice.SetOutputToNull()
        # 16-bit mono at 16 kHz: 32,000 bytes a second after the 46-byte header.
        $seconds = ((Get-Item -LiteralPath $Path).Length - 46) / 32000
        if ($MaxSeconds -le 0 -or $seconds -le $MaxSeconds) { break }
    }
} finally {
    $voice.Dispose()
}
# The data chunk comes last: append the silence to it and correct both sizes.
$bytes = [System.Collections.Generic.List[byte]]([IO.File]::ReadAllBytes($Path))
$data = 12
while ([Text.Encoding]::ASCII.GetString($bytes.GetRange($data, 4).ToArray()) -ne 'data') {
    $data += 8 + [BitConverter]::ToUInt32($bytes.GetRange($data + 4, 4).ToArray(), 0)
}
$silence = [int]($SilenceSeconds * 16000) * 2
$bytes.AddRange([byte[]]::new($silence))
$dataSize = [BitConverter]::ToUInt32($bytes.GetRange($data + 4, 4).ToArray(), 0) + $silence
$bytes.RemoveRange($data + 4, 4)
$bytes.InsertRange($data + 4, [BitConverter]::GetBytes([uint32]$dataSize))
$bytes.RemoveRange(4, 4)
$bytes.InsertRange(4, [BitConverter]::GetBytes([uint32]($bytes.Count - 8)))
[IO.File]::WriteAllBytes($Path, $bytes.ToArray())
[math]::Round($seconds, 2)
