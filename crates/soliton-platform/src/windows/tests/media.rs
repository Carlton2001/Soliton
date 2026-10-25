use super::*;

#[test]
fn buttons_are_explicit_and_times_are_bounded() {
    assert_eq!(button_command(Button::Play), Some(MediaCommand::Play));
    assert_eq!(button_command(Button::Pause), Some(MediaCommand::Pause));
    assert_eq!(button_command(Button::Stop), Some(MediaCommand::Stop));
    assert_eq!(button_command(Button::Next), Some(MediaCommand::Next));
    assert_eq!(
        button_command(Button::Previous),
        Some(MediaCommand::Previous)
    );
    assert_eq!(button_command(Button::FastForward), None);
    assert_eq!(timespan(123).Duration, 1_230_000);
    assert_eq!(timespan(u64::MAX).Duration, i64::MAX);
}

#[test]
fn missing_or_oversized_cached_artwork_is_reported_before_creating_a_stream() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("cover.png");
    assert!(
        artwork(&path)
            .unwrap_err()
            .to_string()
            .contains("Cannot open cached artwork")
    );
    File::create(&path)
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    assert!(
        artwork(&path)
            .unwrap_err()
            .to_string()
            .contains("exceeds 8 MiB")
    );
}
