use super::*;
#[path = "support.rs"]
mod support;

#[test]
fn streaming_decoder_reads_pcm_and_seeks_without_changing_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    support::wav(&path, 44100, 2);
    let before = std::fs::read(&path).unwrap();
    let mut decoder = Reader::open(&path).unwrap();
    assert_eq!(decoder.rate, 44100);
    assert_eq!(decoder.channels, 1);
    assert_eq!(decoder.duration_ms, 2000);
    let mut frames = 0;
    while let Some(samples) = decoder.next().unwrap() {
        assert!(samples.iter().all(|s| s.is_finite()));
        frames += samples.len();
    }
    assert_eq!(frames, 88200);
    assert_eq!(decoder.seek(1000).unwrap(), 1000);
    let mut after_seek = 0;
    while let Some(samples) = decoder.next().unwrap() {
        after_seek += samples.len();
    }
    assert_eq!(after_seek, 44100);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn malformed_audio_returns_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.flac");
    std::fs::write(&path, b"not audio").unwrap();
    assert!(Reader::open(&path).is_err());
}

#[test]
#[ignore = "Generate local synthetic fixtures with scripts/generate-audio-fixtures.ps1 first"]
fn generated_codec_fixtures_decode_in_rust() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/codec-fixtures");
    for extension in ["wav", "flac", "mp3", "m4a", "alac.m4a", "ogg", "aiff"] {
        let path = root.join(format!("tone.{extension}"));
        let mut reader = Reader::open(&path).unwrap_or_else(|e| panic!("{extension}: {e:#}"));
        let mut frames = 0;
        while let Some(samples) = reader
            .next()
            .unwrap_or_else(|e| panic!("{extension}: {e:#}"))
        {
            frames += samples.len() / reader.channels;
        }
        assert!((90000..=100000).contains(&frames), "{extension}: {frames}");
        reader
            .seek(1000)
            .unwrap_or_else(|e| panic!("{extension} seek: {e:#}"));
        assert!(
            reader.next().unwrap().is_some(),
            "{extension} seek reached EOF"
        );
    }
}
