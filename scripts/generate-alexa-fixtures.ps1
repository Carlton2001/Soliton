$ErrorActionPreference = 'Stop'
# Development fixtures only; no library source is read or modified.
$destination = Join-Path $PSScriptRoot '..\target\alexa-fixtures'
New-Item -ItemType Directory -Force $destination | Out-Null
$fixtures = @(
    @{ File = 'tone-16-44100.flac'; Rate = 44100; Codec = 'flac'; Format = 's16' },
    @{ File = 'tone-24-96000.flac'; Rate = 96000; Codec = 'flac'; Format = 's32' },
    @{ File = 'tone-16-44100.wav'; Rate = 44100; Codec = 'pcm_s16le'; Format = 's16' }
)
foreach ($fixture in $fixtures) {
    $outputPath = Join-Path $destination $fixture.File
    # Never overwrite an existing fixture. Remove a generated file manually to regenerate it.
    if (Test-Path -LiteralPath $outputPath) { continue }
    & ffmpeg -hide_banner -loglevel error -n -f lavfi -i "sine=frequency=440:duration=5:sample_rate=$($fixture.Rate)" -af 'volume=0.03' -ac 2 -sample_fmt $fixture.Format -c:a $fixture.Codec $outputPath
    if ($LASTEXITCODE -ne 0) { throw 'Alexa fixture generation failed' }
}
Write-Output $destination
