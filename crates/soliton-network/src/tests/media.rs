use super::*;

#[test]
fn xml_and_upnp_durations_handle_metadata_without_markup_injection() {
    assert_eq!(
        xml("Été & <album> \"live\" 'mix'"),
        "Été &amp; &lt;album&gt; &quot;live&quot; &apos;mix&apos;"
    );
    assert_eq!(duration(3_723_999), "1:02:03");
    assert_eq!(milliseconds("1:02:03.5"), Some(3_723_500));
    for invalid in [
        "NOT_IMPLEMENTED",
        "0:60:00",
        "0:00:NaN",
        "0:00:inf",
        "0:00:-1",
        "0:00:60",
        "0:00:01:02",
        "999999999999999999:00:00",
    ] {
        assert_eq!(milliseconds(invalid), None, "{invalid}");
    }
}

fn track(path: &str) -> Track {
    Track {
        id: 1,
        source_id: 1,
        path: path.into(),
        metadata: Default::default(),
        cover: None,
        available: true,
    }
}

#[test]
fn network_formats_follow_the_extension_whatever_its_case() {
    for (path, expected) in [
        ("a.mp3", "audio/mpeg"),
        ("a.FLAC", "audio/flac"),
        ("a.m4a", "audio/mp4"),
        ("a.mp4", "audio/mp4"),
        ("a.aac", "audio/aac"),
        ("a.Wav", "audio/wav"),
        ("a.wave", "audio/wav"),
        ("a.aif", "audio/aiff"),
        ("a.AIFF", "audio/aiff"),
        ("a.ogg", "audio/ogg"),
        ("a.oga", "audio/ogg"),
    ] {
        assert_eq!(mime(&track(path)).unwrap(), expected, "{path}");
    }
    for path in ["a.opus", "a.wma", "no-extension"] {
        assert!(mime(&track(path)).is_err(), "{path}");
    }
}

#[test]
fn durations_round_trip_at_the_second_and_start_at_zero() {
    assert_eq!(duration(0), "0:00:00");
    assert_eq!(duration(999), "0:00:00");
    assert_eq!(duration(36_000_000), "10:00:00");
    assert_eq!(milliseconds("0:00:00"), Some(0));
    assert_eq!(milliseconds("10:00:00"), Some(36_000_000));
    assert_eq!(milliseconds(&duration(3_723_000)), Some(3_723_000));
    // Renderers often report fractions beyond milliseconds.
    assert_eq!(milliseconds("0:00:01.2345"), Some(1_234));
    for invalid in ["", "1:02", ":00:00", "a:00:00", "0:-1:00"] {
        assert_eq!(milliseconds(invalid), None, "{invalid}");
    }
}
