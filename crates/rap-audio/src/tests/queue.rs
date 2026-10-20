use super::*;
use rap_core::model::Metadata;

fn tracks() -> Vec<Track> {
    (0..8)
        .map(|id| Track {
            id,
            source_id: 1,
            path: format!("{id}.flac").into(),
            metadata: Metadata::default(),
            cover: None,
            available: true,
        })
        .collect()
}

#[test]
fn shuffle_is_a_permutation_and_preserves_the_current_track() {
    let mut queue = Queue::default();
    queue.set(tracks(), 3);
    queue.shuffle(true);
    assert_eq!(queue.current().unwrap().id, 3);
    let mut ids = vec![queue.current().unwrap().id];
    while queue.next(false) {
        ids.push(queue.current().unwrap().id);
    }
    ids.sort();
    assert_eq!(ids, (0..8).collect::<Vec<_>>());
    queue.shuffle(false);
    assert!(queue.current().is_some());
}

#[test]
fn repeat_modes_manual_navigation_and_empty_queue() {
    let mut queue = Queue::default();
    assert!(!queue.next(true));
    assert!(!queue.previous());
    queue.set(tracks(), 7);
    assert!(!queue.next(true));
    queue.repeat = Repeat::All;
    assert!(queue.next(true));
    assert_eq!(queue.current().unwrap().id, 0);
    queue.repeat = Repeat::Track;
    assert!(queue.next(true));
    assert_eq!(queue.current().unwrap().id, 0);
    assert!(queue.next(false));
    assert_eq!(queue.current().unwrap().id, 1);
    assert!(queue.previous());
    assert_eq!(queue.current().unwrap().id, 0);
}
