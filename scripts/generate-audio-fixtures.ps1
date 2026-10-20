$ErrorActionPreference = 'Stop'
# Optional development tool only. RAP itself never calls FFmpeg.
$destination = Join-Path $PSScriptRoot '..\target\codec-fixtures'
New-Item -ItemType Directory -Force $destination | Out-Null
$wave = Join-Path $destination 'tone.wav'
& ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'sine=frequency=440:duration=2:sample_rate=48000' -ac 2 -c:a pcm_s16le $wave
if ($LASTEXITCODE -ne 0) { throw 'WAV fixture generation failed' }
$formats = @(
    @('flac','flac'), @('mp3','libmp3lame'), @('m4a','aac'),
    @('alac.m4a','alac'), @('ogg','libvorbis'), @('aiff','pcm_s16be')
)
foreach ($format in $formats) {
    & ffmpeg -hide_banner -loglevel error -y -i $wave -c:a $format[1] -metadata 'title=Été synthétique' -metadata 'artist=RAP tests' -metadata 'album=Generated tones' -metadata 'album_artist=RAP tests' (Join-Path $destination ('tone.' + $format[0]))
    if ($LASTEXITCODE -ne 0) { throw ('Fixture generation failed: ' + $format[0]) }
}
Write-Output $destination
