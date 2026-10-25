use super::*;
use soliton_core::model::Metadata;

fn track(id: i64) -> Track {
    Track {
        id,
        source_id: 1,
        path: format!("{id}.flac").into(),
        metadata: Metadata {
            title: format!("Song {id}"),
            duration_ms: 30_000,
            ..Default::default()
        },
        cover: None,
        available: true,
    }
}

#[test]
fn manual_lists_persist_unique_files_order_and_unavailable_entries() {
    let dir = tempfile::tempdir().unwrap();
    let mut library = Library::open(dir.path()).unwrap();
    // Missing catalogue tracks are retained by their metadata without a dangling FK.
    library
        .change_playlist(PlaylistChange::Create {
            name: "  Evening  ".into(),
            tracks: vec![track(1), track(2), track(1), track(3)],
        })
        .unwrap();
    let list = library.playlists().unwrap().remove(0);
    assert_eq!(list.name, "Evening");
    assert_eq!(list.entries.len(), 3);
    assert!(list.entries.iter().all(|e| e.track_id.is_none()));
    assert_ne!(list.entries[0].id, list.entries[2].id);
    library
        .change_playlist(PlaylistChange::Move {
            id: list.id,
            entry: list.entries[2].id,
            before: Some(list.entries[0].id),
        })
        .unwrap();
    library
        .change_playlist(PlaylistChange::Rename {
            id: list.id,
            name: "Night".into(),
        })
        .unwrap();
    drop(library);
    let mut library = Library::open(dir.path()).unwrap();
    let saved = library.playlists().unwrap().remove(0);
    assert_eq!(saved.name, "Night");
    assert_eq!(
        saved.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![list.entries[2].id, list.entries[0].id, list.entries[1].id]
    );
    library
        .change_playlist(PlaylistChange::Append {
            id: list.id,
            tracks: vec![track(3), track(4), track(4)],
        })
        .unwrap();
    library
        .change_playlist(PlaylistChange::Remove {
            id: list.id,
            entry: list.entries[0].id,
        })
        .unwrap();
    assert_eq!(library.playlists().unwrap()[0].entries.len(), 3);
    library
        .change_playlist(PlaylistChange::Delete(list.id))
        .unwrap();
    assert!(library.playlists().unwrap().is_empty());
    assert_eq!(
        library
            .connection
            .query_row("SELECT COUNT(*) FROM playlist_entries", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn invalid_names_and_stale_moves_do_not_change_saved_lists() {
    let dir = tempfile::tempdir().unwrap();
    let mut library = Library::open(dir.path()).unwrap();
    for name in ["  ".to_string(), "x".repeat(201)] {
        assert!(
            library
                .change_playlist(PlaylistChange::Create {
                    name,
                    tracks: vec![]
                })
                .is_err()
        );
    }
    library
        .change_playlist(PlaylistChange::Create {
            name: "Écoute 🎵".into(),
            tracks: vec![track(1), track(2)],
        })
        .unwrap();
    let list = library.playlists().unwrap().remove(0);
    assert!(
        library
            .change_playlist(PlaylistChange::Move {
                id: list.id,
                entry: list.entries[0].id,
                before: Some(99999)
            })
            .is_err()
    );
    assert_eq!(
        library.playlists().unwrap()[0].entries[0].id,
        list.entries[0].id
    );
    assert!(
        library
            .change_playlist(PlaylistChange::Rename {
                id: list.id,
                name: "".into()
            })
            .is_err()
    );
    assert_eq!(library.playlists().unwrap()[0].name, "Écoute 🎵");
}

#[test]
fn catalogue_removal_keeps_playlist_metadata_and_releases_reference() {
    let dir = tempfile::tempdir().unwrap();
    let mut library = Library::open(dir.path()).unwrap();
    library
        .connection
        .execute(
            "INSERT INTO sources(id,path,available) VALUES(1,'music',1)",
            [],
        )
        .unwrap();
    library
        .connection
        .execute(
            "INSERT INTO editions(id,identity,source_id,folder) VALUES(1,'edition',1,'music')",
            [],
        )
        .unwrap();
    library
        .connection
        .execute(
            "INSERT INTO tracks(id,edition_id,metadata) VALUES(1,1,?1)",
            [serde_json::to_string(&track(1).metadata).unwrap()],
        )
        .unwrap();
    library
        .change_playlist(PlaylistChange::Create {
            name: "Keep".into(),
            tracks: vec![track(1)],
        })
        .unwrap();
    assert_eq!(library.playlists().unwrap()[0].entries[0].track_id, Some(1));
    library
        .connection
        .execute("DELETE FROM tracks WHERE id=1", [])
        .unwrap();
    let saved = library.playlists().unwrap();
    assert_eq!(saved[0].entries[0].track_id, None);
    assert_eq!(saved[0].entries[0].metadata.title, "Song 1");
    library
        .change_playlist(PlaylistChange::Append {
            id: saved[0].id,
            tracks: vec![track(1)],
        })
        .unwrap();
    assert_eq!(library.playlists().unwrap()[0].entries.len(), 1);
}

#[test]
fn deduplication_uses_file_paths_and_is_scoped_to_each_playlist() {
    let dir = tempfile::tempdir().unwrap();
    let mut library = Library::open(dir.path()).unwrap();
    let first = track(1);
    let mut other = first.clone();
    other.id = 2;
    other.path = "another-edition.flac".into();
    for name in ["A", "B"] {
        library
            .change_playlist(PlaylistChange::Create {
                name: name.into(),
                tracks: vec![first.clone(), other.clone(), first.clone()],
            })
            .unwrap();
    }
    let lists = library.playlists().unwrap();
    assert!(lists.iter().all(|list| list.entries.len() == 2));
    drop(library);
    let mut library = Library::open(dir.path()).unwrap();
    library
        .change_playlist(PlaylistChange::Append {
            id: lists[0].id,
            tracks: vec![first, other],
        })
        .unwrap();
    assert_eq!(library.playlists().unwrap()[0].entries.len(), 2);
}

#[test]
fn version_five_migration_keeps_first_file_in_saved_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut library = Library::open(dir.path()).unwrap();
    library
        .connection
        .execute_batch(
            "INSERT INTO sources(id,path,available) VALUES(1,'music',1);
         INSERT INTO editions(id,identity,source_id,folder) VALUES(1,'edition',1,'music');
         DROP INDEX playlist_file;
         ALTER TABLE playlist_entries DROP COLUMN path;
         PRAGMA user_version=5;",
        )
        .unwrap();
    library
        .connection
        .execute(
            "INSERT INTO tracks(id,edition_id,metadata) VALUES(1,1,?1)",
            [serde_json::to_string(&track(1).metadata).unwrap()],
        )
        .unwrap();
    library.connection.execute_batch(
        "INSERT INTO files(source_id,path,track_id,size,modified,seen,available) VALUES(1,'1.flac',1,1,'0',0,1);
         INSERT INTO playlists(id,name) VALUES(1,'Old list');"
    ).unwrap();
    let metadata = serde_json::to_string(&track(1).metadata).unwrap();
    for (id, position) in [(10, 2), (11, 0), (12, 0)] {
        library.connection.execute("INSERT INTO playlist_entries(id,playlist_id,track_id,position,metadata) VALUES(?1,1,1,?2,?3)", params![id, position, metadata]).unwrap();
    }
    // Older unavailable entries have no recoverable path; never merge by title.
    library
        .connection
        .execute(
            "INSERT INTO playlist_entries(id,playlist_id,position,metadata) VALUES(13,1,3,?1)",
            [metadata],
        )
        .unwrap();
    drop(library);
    library = Library::open(dir.path()).unwrap();
    let list = library.playlists().unwrap().remove(0);
    assert_eq!(
        list.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![11, 13]
    );
    library
        .change_playlist(PlaylistChange::Append {
            id: 1,
            tracks: vec![track(1)],
        })
        .unwrap();
    assert_eq!(library.playlists().unwrap()[0].entries.len(), 2);
    assert!(library.connection.execute("INSERT INTO playlist_entries(playlist_id,track_id,position,metadata,path) VALUES(1,1,4,'{}','1.flac')", []).is_err());
}
