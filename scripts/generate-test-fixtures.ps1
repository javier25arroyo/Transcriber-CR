#requires -Version 5
<#
.SYNOPSIS
    Regenera los fixtures de audio del test de integración del sidecar STT.

.DESCRIPTION
    El test `transcribe_wav.rs` necesita un wav de voz para comprobar que
    whisper.cpp transcribe de verdad. En lugar de versionar la voz real de
    alguien, el fixture es habla sintética generada con el motor de voz de
    Windows (SAPI, System.Speech), a 16 kHz mono de 16 bits — exactamente el
    formato que espera whisper.cpp.

    Idempotente: sobrescribe el/los wav de salida.
#>
[CmdletBinding()]
param(
    # Carpeta de salida; por defecto, la de fixtures del sidecar.
    [string]$OutDir = (Join-Path $PSScriptRoot '..\stt-sidecar\tests\fixtures')
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Speech

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# 16 kHz, 16 bits, mono: lo que consume el motor y lo que lee `load_wav_16k_mono`
# en el test. Cambiar esto obliga a cambiar las aserciones del test.
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(
    16000,
    [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen,
    [System.Speech.AudioFormat.AudioChannel]::Mono
)

# Cada fixture: nombre de archivo => frase a decir. La aserción del test busca
# una palabra de esta frase, así que si se agrega uno hay que cablear el test.
$fixtures = @{
    'hola_mundo.wav' = 'hola mundo'
}

foreach ($name in $fixtures.Keys) {
    $path = Join-Path $OutDir $name
    $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
    try {
        $synth.SetOutputToWaveFile($path, $format)
        $synth.Speak($fixtures[$name])
    }
    finally {
        $synth.Dispose()
    }
    $size = (Get-Item $path).Length
    Write-Host "Generado $path ($size bytes): «$($fixtures[$name])»"
}
