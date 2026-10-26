use super::{SEEK_SETTLED_MS, SEEK_TIMEOUT, seek_settled};
use std::time::Duration;

#[test]
fn seek_stays_shown_until_the_engine_reports_a_nearby_position() {
    let early = Duration::from_millis(100);
    // A state sent before the seek still carries the old position.
    assert!(!seek_settled(10_000, 90_000, early, false));
    assert!(seek_settled(90_000, 90_000, early, false));
    // Seeks land on packet boundaries, slightly before or after the target.
    assert!(seek_settled(
        90_000 - SEEK_SETTLED_MS + 1,
        90_000,
        early,
        false
    ));
    assert!(!seek_settled(
        90_000 - SEEK_SETTLED_MS,
        90_000,
        early,
        false
    ));
}

#[test]
fn unconfirmed_seek_is_dropped_on_timeout_or_track_change() {
    assert!(seek_settled(10_000, 90_000, SEEK_TIMEOUT, false));
    assert!(seek_settled(0, 90_000, Duration::from_millis(100), true));
}
