use super::*;
use crate::test_support as support;

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

/// Reads a file until `fail_at`, then fails as a lost network share does.
struct Failing {
    inner: std::io::Cursor<Vec<u8>>,
    fail_at: u64,
}

impl std::io::Read for Failing {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let left = self.fail_at.saturating_sub(self.inner.position()) as usize;
        if left == 0 {
            return Err(std::io::Error::other("network name no longer available"));
        }
        let end = buffer.len().min(left);
        self.inner.read(&mut buffer[..end])
    }
}

impl std::io::Seek for Failing {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(position)
    }
}

impl MediaSource for Failing {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.inner.get_ref().len() as u64)
    }
}

#[test]
fn a_read_error_during_playback_ends_the_track_with_what_was_decoded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    support::wav(&path, 44100, 2);
    let bytes = std::fs::read(&path).unwrap();
    let fail_at = bytes.len() as u64 / 2;
    let mut decoder = Reader::from_source(&path, || {
        Ok(Box::new(Failing {
            inner: std::io::Cursor::new(bytes),
            fail_at,
        }))
    })
    .unwrap();
    let mut frames = 0;
    while let Some(samples) = decoder.next().unwrap() {
        frames += samples.len();
    }
    assert!(frames > 0 && frames < 88200, "{frames}");
    assert_eq!(decoder.skipped, 1);
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

#[test]
#[ignore = "Generate local synthetic fixtures with scripts/generate-audio-fixtures.ps1 first"]
fn isolated_corruption_is_skipped_and_counted() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/codec-fixtures");
    let dir = tempfile::tempdir().unwrap();
    for extension in ["flac", "mp3"] {
        let mut bytes = std::fs::read(root.join(format!("tone.{extension}"))).unwrap();
        // A damaged region in the middle, as a bad sector or transfer would leave.
        let middle = bytes.len() / 2;
        bytes[middle..middle + 64].fill(0x55);
        let path = dir.path().join(format!("damaged.{extension}"));
        std::fs::write(&path, bytes).unwrap();
        let mut reader = Reader::open(&path).unwrap_or_else(|e| panic!("{extension}: {e:#}"));
        let mut frames = 0;
        while let Some(samples) = reader
            .next()
            .unwrap_or_else(|e| panic!("{extension}: {e:#}"))
        {
            frames += samples.len() / reader.channels;
        }
        assert!(frames > 80000, "{extension}: {frames}");
        // The FLAC reader resynchronises on the next frame by itself; the MP3
        // decoder rejects the damaged packet, which used to stop playback.
        if extension == "mp3" {
            assert!(reader.skipped > 0);
        }
    }
}
