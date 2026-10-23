use super::*;

fn track(album: &str, artist: &str, year: Option<u32>) -> Metadata {
    Metadata {
        album: album.into(),
        artist: artist.into(),
        album_artist: artist.into(),
        year,
        // As produced by extraction when only the track artist is tagged.
        inferred: vec!["album_artist".into()],
        ..Default::default()
    }
}

#[test]
fn recognises_disc_folders_only() {
    for (name, disc) in [
        ("CD1", Some(1)),
        ("cd 2", Some(2)),
        ("Disc 03", Some(3)),
        ("Disque_4", Some(4)),
        ("disk-2 (Bonus)", Some(2)),
        ("CD2 - Live", Some(2)),
        ("Discography", None),
        ("CD", None),
        ("Disco 2000", None),
        ("CD1999", None),
        ("CDX", None),
        ("Abbey Road", None),
    ] {
        assert_eq!(disc_folder(name), disc, "{name}");
    }
}

#[test]
fn disc_subfolders_belong_to_the_album_folder_inside_the_source() {
    let root = Path::new("music");
    let file = Path::new("music/Artist/Album/CD2/01.flac");
    assert_eq!(album_folder(file), Some(Path::new("music/Artist/Album")));
    assert_eq!(edition_folder(file, root), Path::new("music/Artist/Album"));
    // A disc folder added directly as a source never climbs outside it.
    let disc_root = Path::new("music/Artist/Album/CD2");
    assert_eq!(edition_folder(file, disc_root), disc_root);
    let plain = Path::new("music/Artist/Album/01.flac");
    assert_eq!(edition_folder(plain, root), Path::new("music/Artist/Album"));
}

#[test]
fn identical_artist_title_and_year_merge_across_folders() {
    let a = assign("a", &[track("Kind of Blue", "Miles Davis", Some(1959))]).unwrap();
    let b = assign("b", &[track("kind  of blue", "MILES DAVIS", Some(1959))]).unwrap();
    assert_eq!(a.identity, b.identity);
    assert_eq!(a.artist, "Miles Davis");
    assert!(!a.compilation);
}

#[test]
fn ambiguous_editions_stay_apart() {
    let base = assign("a", &[track("Album", "Artist", Some(2000))]).unwrap();
    let other_year = assign("b", &[track("Album", "Artist", Some(2011))]).unwrap();
    let deluxe = assign("c", &[track("Album (Deluxe)", "Artist", Some(2000))]).unwrap();
    let no_year = assign("d", &[track("Album", "Artist", None)]).unwrap();
    let no_year_again = assign("e", &[track("Album", "Artist", None)]).unwrap();
    let mut inferred = track("Album", "Artist", Some(2000));
    inferred.inferred.push("album".into());
    let inferred = assign("f", &[inferred]).unwrap();
    for album in [&other_year, &deluxe, &no_year, &inferred] {
        assert_ne!(album.identity, base.identity);
    }
    assert_ne!(no_year.identity, no_year_again.identity);
}

#[test]
fn untagged_compilations_resolve_per_edition() {
    let tracks = [
        track("Hits", "Alpha", Some(2001)),
        track("Hits", "Beta", Some(1999)),
        track("Hits", "Gamma", Some(2001)),
    ];
    let album = assign("a", &tracks).unwrap();
    assert!(album.compilation);
    assert_eq!(album.artist, "");
    // The most frequent year identifies the compilation.
    let mut same = tracks.clone();
    same[1].year = Some(2001);
    assert_eq!(assign("b", &same).unwrap().identity, album.identity);
    let mut flagged = track("Solo", "Alpha", None);
    flagged.compilation = true;
    assert!(assign("c", &[flagged]).unwrap().compilation);
    let mut tagged = track("Hits", "Alpha", Some(2001));
    tagged.album_artist = "Various".into();
    tagged.inferred.clear();
    let tagged = assign("d", &[tagged, track("Hits", "Beta", Some(2001))]).unwrap();
    assert_eq!(tagged.artist, "Various");
    assert!(!tagged.compilation);
}

#[test]
fn edition_identity_ignores_inferred_album_artists() {
    let alpha = track("Hits", "Alpha", None);
    let beta = track("Hits", "Beta", None);
    assert_eq!(
        edition_identity(1, "dir", &alpha),
        edition_identity(1, "dir", &beta)
    );
    assert_ne!(
        edition_identity(1, "dir", &alpha),
        edition_identity(2, "dir", &alpha)
    );
}

#[test]
fn partial_album_artist_tags_require_a_single_explicit_artist_and_matching_tracks() {
    let missing = track("Bleach", "Nirvana", Some(1989));
    let mut tagged = missing.clone();
    tagged.inferred.clear();
    assert_eq!(
        compatible_album_artist([&missing, &tagged]),
        Some("nirvana".into())
    );
    assert_eq!(compatible_album_artist([&missing]), None);
    let mut other = tagged.clone();
    other.album_artist = "Other".into();
    assert_eq!(compatible_album_artist([&missing, &tagged, &other]), None);
    let guest = track("Bleach", "Guest", Some(1989));
    assert_eq!(compatible_album_artist([&tagged, &guest]), None);
    let mut flagged = missing.clone();
    flagged.compilation = true;
    assert_eq!(compatible_album_artist([&tagged, &flagged]), None);
    let mut inferred = missing.clone();
    inferred.inferred.push("album".into());
    assert_eq!(compatible_album_artist([&tagged, &inferred]), None);
}
