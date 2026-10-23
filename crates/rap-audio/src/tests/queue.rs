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

#[test]
fn edits_preserve_current_and_saved_snapshots() {
    let mut queue = Queue::default();
    queue.set(tracks()[2..5].to_vec(), 1);
    let original = queue.snapshot.clone();
    let current = original.current.unwrap();
    let last = original.entries[2].id;
    queue.move_before(last, Some(original.entries[0].id));
    assert_eq!(queue.snapshot.current, Some(current));
    assert_eq!(queue.current().unwrap().id, 3);
    queue.remove(original.entries[0].id);
    assert_eq!(queue.snapshot.entries.len(), 2);
    assert_eq!(queue.snapshot.current, Some(current));
    queue.remove(current); // The current stream has to be stopped explicitly.
    assert_eq!(queue.snapshot.entries.len(), 2);
    assert_eq!(original.entries.len(), 3);
    assert_eq!(original.entries[0].id, 1);
    queue.enqueue(vec![tracks()[7].clone()], true);
    let ids: std::collections::HashSet<_> = queue.snapshot.entries.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), 3);
    assert!(queue.next(false));
    assert_ne!(queue.snapshot.current, Some(current));
}

#[test]
fn next_insertion_and_manual_order_survive_shuffle_round_trip() {
    let mut queue = Queue::default();
    queue.set(tracks()[..3].to_vec(), 0);
    queue.enqueue(vec![tracks()[7].clone()], true);
    assert_eq!(
        queue
            .snapshot
            .entries
            .iter()
            .map(|e| e.track.id)
            .collect::<Vec<_>>(),
        vec![0, 7, 1, 2]
    );
    let last = queue.snapshot.entries[3].id;
    queue.move_before(last, Some(queue.snapshot.entries[1].id));
    let manual = queue
        .snapshot
        .entries
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>();
    queue.shuffle(true);
    queue.shuffle(false);
    assert_eq!(
        queue
            .snapshot
            .entries
            .iter()
            .map(|e| e.id)
            .collect::<Vec<_>>(),
        manual
    );
}

#[test]
fn shuffled_edits_and_clear_upcoming_keep_a_valid_permutation() {
    let mut queue = Queue::default();
    queue.set(tracks(), 3);
    queue.shuffle(true);
    queue.next(false);
    let current = queue.snapshot.current;
    let upcoming = queue.snapshot.entries[4].id;
    queue.move_before(upcoming, Some(queue.snapshot.entries[2].id));
    queue.remove(queue.snapshot.entries[3].id);
    queue.clear_upcoming();
    assert_eq!(queue.snapshot.current, current);
    assert_eq!(queue.snapshot.entries.len(), 2);
    assert!(!queue.next(false));
    queue.shuffle(false);
    let mut ids = queue
        .snapshot
        .entries
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), queue.snapshot.entries.len());
    queue.set(tracks(), 0);
    assert!(!queue.select(current.unwrap())); // Old session handles are rejected.
}

#[test]
fn playlist_highlight_follows_the_exact_occurrence_through_queue_edits() {
    let mut queue = Queue::default();
    queue.set_entries(
        tracks()[2..5]
            .iter()
            .enumerate()
            .map(|(i, track)| (Some(101 + i as i64), track.clone()))
            .collect(),
        1,
    );
    let current = queue.snapshot.current.unwrap();
    assert_eq!(queue.snapshot.current_playlist_entry(), Some(102));
    queue.move_before(current, Some(queue.snapshot.entries[0].id));
    assert_eq!(queue.snapshot.current_playlist_entry(), Some(102));
    queue.shuffle(true);
    queue.shuffle(false);
    assert_eq!(queue.snapshot.current_playlist_entry(), Some(102));
    assert!(queue.next(false));
    assert_eq!(queue.snapshot.current_playlist_entry(), Some(101));
    queue.remove(current);
    queue.clear_upcoming();
    assert_eq!(queue.snapshot.current_playlist_entry(), Some(101));
    queue.set(tracks(), 0);
    assert_eq!(queue.snapshot.current_playlist_entry(), None);
}

#[test]
fn deduplication_preserves_selected_file_origin_and_distinct_recordings() {
    let mut queue = Queue::default();
    let t = tracks(); // Identical metadata, different files.
    queue.set_entries(
        vec![
            (Some(10), t[0].clone()),
            (Some(11), t[0].clone()),
            (Some(12), t[1].clone()),
            (Some(13), t[2].clone()),
        ],
        2,
    );
    assert_eq!(queue.snapshot.entries.len(), 3);
    assert_eq!(queue.current().unwrap().id, 1);
    assert_eq!(queue.snapshot.current_playlist_entry(), Some(12));
    let current = queue.snapshot.current;
    queue.enqueue(vec![t[1].clone(), t[3].clone(), t[3].clone()], true);
    assert_eq!(queue.snapshot.current, current);
    assert_eq!(
        queue
            .snapshot
            .entries
            .iter()
            .map(|e| e.track.id)
            .collect::<Vec<_>>(),
        vec![0, 1, 3, 2]
    );
    let saved = queue.snapshot.clone();
    queue.enqueue(vec![t[0].clone(), t[3].clone()], false);
    assert!(Arc::ptr_eq(&saved, &queue.snapshot));
    queue.set(vec![t[0].clone(), t[1].clone(), t[0].clone()], 2);
    assert_eq!(queue.current().unwrap().id, 0);
    assert_eq!(queue.snapshot.entries.len(), 2);
}
