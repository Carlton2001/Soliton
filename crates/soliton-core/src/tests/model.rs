use super::*;

fn album(inferred: &[&str]) -> Album {
    let metadata = Metadata {
        inferred: inferred.iter().map(|&field| field.into()).collect(),
        ..Metadata::default()
    };
    let track = Track {
        id: 1,
        source_id: 1,
        path: PathBuf::from("a.flac"),
        metadata,
        cover: None,
        available: true,
    };
    Album {
        id: 1,
        title: String::new(),
        artist: String::new(),
        compilation: false,
        added: 0,
        editions: vec![Edition {
            id: 1,
            source_id: 1,
            folder: PathBuf::new(),
            tracks: vec![track],
        }],
    }
}

#[test]
fn album_artist_copied_from_the_artist_tag_is_not_inferred() {
    assert!(!album(&["album_artist"]).artist_inferred());
    assert!(album(&["artist", "album_artist"]).artist_inferred());
    assert!(!album(&[]).artist_inferred());
}
