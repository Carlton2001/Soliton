use super::{SEEK_SETTLED_MS, SEEK_TIMEOUT, seek_settled, stale_availability, wheel_steps};
use gpui::{ScrollDelta, point, px};
use soliton_audio::{QueueEntry, QueueSnapshot};
use soliton_core::model::Track;
use std::{collections::HashMap, time::Duration};

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

fn track(id: i64, available: bool) -> Track {
    Track {
        id,
        source_id: 1,
        path: format!("{id}.flac").into(),
        metadata: Default::default(),
        cover: None,
        available,
    }
}

#[test]
fn queued_copies_follow_the_catalog_availability() {
    let queue = QueueSnapshot {
        entries: [
            track(1, false),
            track(2, true),
            track(3, true),
            track(-1, true),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, track)| QueueEntry {
            id: i as u64,
            track,
            playlist_entry: None,
        })
        .collect(),
        current: Some(0),
    };
    // Source back online, source gone offline, track left the library;
    // files outside the library keep their own state.
    let catalog = HashMap::from([(1, track(1, true)), (2, track(2, false))]);
    assert_eq!(
        stale_availability(&queue, &catalog),
        vec![(1, true), (2, false), (3, false)]
    );
    let catalog = HashMap::from([
        (1, track(1, false)),
        (2, track(2, true)),
        (3, track(3, true)),
    ]);
    assert!(stale_availability(&queue, &catalog).is_empty());
}

#[test]
fn wheel_steps_once_per_notch_and_ignores_sideways_scrolling() {
    let mut pending = 0.;
    // Windows reports a notch as several lines.
    assert_eq!(
        wheel_steps(&mut pending, ScrollDelta::Lines(point(0., 3.))),
        1
    );
    assert_eq!(
        wheel_steps(&mut pending, ScrollDelta::Lines(point(0., -3.))),
        -1
    );
    assert_eq!(
        wheel_steps(&mut pending, ScrollDelta::Lines(point(2., 0.))),
        0
    );
    // Fractions of a line and touchpad pixels add up to a step.
    assert_eq!(
        wheel_steps(&mut pending, ScrollDelta::Lines(point(0., 0.5))),
        0
    );
    assert_eq!(
        wheel_steps(&mut pending, ScrollDelta::Lines(point(0., 0.5))),
        1
    );
    assert_eq!(
        wheel_steps(&mut pending, ScrollDelta::Pixels(point(px(0.), px(-15.)))),
        0
    );
    assert_eq!(
        wheel_steps(&mut pending, ScrollDelta::Pixels(point(px(0.), px(-30.)))),
        -2
    );
    assert_eq!(pending, -5.);
}
