use super::*;
use crate::{Cancellation, ScanReport};
use lofty::{
    config::WriteOptions,
    picture::{MimeType, Picture, PictureType},
    prelude::TagExt,
    tag::{ItemKey, Tag, TagType},
};
use std::collections::BTreeSet;

fn wav(path: &Path) {
    pcm(path, 44_100, 16);
}

/// Minimal mono PCM file with the given sample rate and depth.
fn pcm(path: &Path, rate: u32, bits: u16) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let align = bits / 8;
    let data = u32::from(align);
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data + data % 2).to_le_bytes());
    bytes.extend(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * data).to_le_bytes());
    bytes.extend(align.to_le_bytes());
    bytes.extend(bits.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data.to_le_bytes());
    // Chunks are word aligned.
    bytes.extend(vec![0; (data + data % 2) as usize]);
    std::fs::write(path, bytes).unwrap();
}

/// Writes tags into a generated fixture; library code never writes tags.
fn tag(path: &Path, items: &[(ItemKey, &str)], cover: bool) {
    let mut tag = Tag::new(TagType::Id3v2);
    for (key, value) in items {
        tag.insert_text(*key, (*value).into());
    }
    if cover {
        let pixels = image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 120, 50, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        pixels.write_to(&mut png, image::ImageFormat::Png).unwrap();
        tag.push_picture(
            Picture::unchecked(png.into_inner())
                .pic_type(PictureType::CoverFront)
                .mime_type(MimeType::Png)
                .build(),
        );
    }
    tag.save_to_path(path, WriteOptions::default()).unwrap();
}

fn album_tags<'a>(album: &'a str, artist: &'a str, year: &'a str) -> Vec<(ItemKey, &'a str)> {
    let mut items = vec![
        (ItemKey::AlbumTitle, album),
        (ItemKey::AlbumArtist, artist),
        (ItemKey::TrackArtist, artist),
    ];
    if !year.is_empty() {
        items.push((ItemKey::RecordingDate, year));
    }
    items
}

fn scan(library: &mut Library, id: i64) -> ScanReport {
    library.scan(id, &Cancellation::default(), |_| {}).unwrap()
}

#[test]
fn partial_album_artist_tags_form_one_edition_and_later_conflicts_split_it() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music/Nirvana");
    let mut originals = Vec::new();
    for index in 1..=11 {
        let path = source.join(format!("Bleach-{index:02}.wav"));
        pcm(&path, 96_000, 24);
        let mut items = album_tags("Bleach", "Nirvana", "1989");
        if index != 5 {
            items.retain(|(key, _)| *key != ItemKey::AlbumArtist);
        }
        tag(&path, &items, false);
        originals.push((path.clone(), std::fs::read(path).unwrap()));
    }
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    scan(&mut library, id);
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 1);
    assert_eq!(snapshot.albums[0].editions.len(), 1);
    assert_eq!(snapshot.albums[0].primary().tracks.len(), 11);
    let ids: BTreeSet<_> = snapshot.albums[0].tracks().map(|t| t.id).collect();
    assert_eq!(scan(&mut library, id).progress.unchanged, 11);
    // Adding a different explicit album artist makes the previously missing tags ambiguous.
    let extra = source.join("other.wav");
    pcm(&extra, 96_000, 24);
    tag(&extra, &album_tags("Bleach", "Other", "1989"), false);
    scan(&mut library, id);
    let snapshot = library.snapshot().unwrap();
    let nirvana = snapshot
        .albums
        .iter()
        .find(|a| a.artist == "Nirvana")
        .unwrap();
    assert_eq!(nirvana.editions.len(), 2);
    assert_eq!(nirvana.tracks().map(|t| t.id).collect::<BTreeSet<_>>(), ids);
    for (path, bytes) in originals {
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn v6_migration_repairs_split_editions_offline_and_preserves_tracks_and_playlists() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music/Nirvana");
    let path = source.join("song.wav");
    wav(&path);
    tag(&path, &album_tags("Bleach", "Nirvana", "1989"), false);
    let missing = source.join("other.wav");
    wav(&missing);
    let mut tags = album_tags("Bleach", "Nirvana", "1989");
    tags.retain(|(key, _)| *key != ItemKey::AlbumArtist);
    tag(&missing, &tags, false);
    let storage = dir.path().join("data");
    let mut library = Library::open(&storage).unwrap();
    let id = library.add_source(&source).unwrap();
    scan(&mut library, id);
    let snapshot = library.snapshot().unwrap();
    let tracks: Vec<_> = snapshot.albums[0].tracks().cloned().collect();
    library
        .change_playlist(rap_core::playlist::PlaylistChange::Create {
            name: "Nirvana".into(),
            tracks: tracks.clone(),
        })
        .unwrap();
    // Recreate the v6 split using stored metadata; IDs and all file rows stay intact.
    let untagged = tracks
        .iter()
        .find(|t| t.metadata.is_inferred("album_artist"))
        .unwrap();
    let folder = path_text(&source.canonicalize().unwrap())
        .unwrap()
        .to_owned();
    let identity = crate::grouping::edition_identity(id, &folder, &untagged.metadata);
    library
        .connection
        .execute(
            "INSERT INTO editions(identity,source_id,folder,album_id) VALUES(?1,?2,?3,?4)",
            params![identity, id, folder, snapshot.albums[0].id],
        )
        .unwrap();
    let edition = library.connection.last_insert_rowid();
    library
        .connection
        .execute(
            "UPDATE tracks SET edition_id=?1 WHERE id=?2",
            params![edition, untagged.id],
        )
        .unwrap();
    library
        .connection
        .execute_batch(
            "UPDATE files SET available=0; UPDATE sources SET available=0; PRAGMA user_version=6;",
        )
        .unwrap();
    drop(library);
    std::fs::rename(&source, dir.path().join("offline")).unwrap();
    let library = Library::open(&storage).unwrap();
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 1);
    assert_eq!(snapshot.albums[0].editions.len(), 1);
    assert_eq!(snapshot.albums[0].primary().tracks.len(), 2);
    assert!(snapshot.albums[0].tracks().all(|t| !t.available));
    assert!(!snapshot.sources[0].available);
    assert_eq!(
        snapshot.albums[0]
            .tracks()
            .map(|t| t.id)
            .collect::<BTreeSet<_>>(),
        tracks.iter().map(|t| t.id).collect()
    );
    let playlist = library.playlists().unwrap().remove(0);
    assert_eq!(
        playlist
            .entries
            .iter()
            .map(|e| e.track_id)
            .collect::<Vec<_>>(),
        tracks.iter().map(|t| Some(t.id)).collect::<Vec<_>>()
    );
    assert_eq!(
        schema::version(&library.connection).unwrap(),
        schema::VERSION
    );
}

#[test]
fn indexes_incrementally_persists_and_preserves_disconnected_sources() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    let audio = source.join("Artist/Album/01 - Song.wav");
    wav(&audio);
    let before = std::fs::read(&audio).unwrap();
    let storage = dir.path().join("data");
    let mut library = Library::open(&storage).unwrap();
    let id = library.add_source(&source).unwrap();
    let report = scan(&mut library, id);
    assert!(report.complete, "{:?}", report.diagnostics);
    assert_eq!(report.progress.indexed, 1);
    let report = scan(&mut library, id);
    assert_eq!(report.progress.unchanged, 1);
    assert_eq!(report.progress.indexed, 0);
    assert_eq!(std::fs::read(&audio).unwrap(), before);
    drop(library);
    let mut library = Library::open(&storage).unwrap();
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 1);
    assert!(snapshot.albums[0].added > 0);
    std::fs::rename(&source, dir.path().join("offline")).unwrap();
    assert!(library.scan(id, &Cancellation::default(), |_| {}).is_err());
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 1);
    assert!(!snapshot.sources[0].available);
    assert!(!snapshot.albums[0].primary().tracks[0].available);
    std::fs::rename(dir.path().join("offline"), &source).unwrap();
    scan(&mut library, id);
    assert!(library.snapshot().unwrap().albums[0].primary().tracks[0].available);
    // A source that suddenly looks empty is more likely an unmounted volume.
    std::fs::remove_file(&audio).unwrap();
    let report = scan(&mut library, id);
    assert_eq!(report.removed, 0);
    assert_eq!(report.diagnostics.len(), 1);
    let snapshot = library.snapshot().unwrap();
    assert!(!snapshot.albums[0].primary().tracks[0].available);
}

#[test]
fn removed_files_are_forgotten_but_never_after_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    let first = source.join("Artist/Album/01.wav");
    let second = source.join("Artist/Album/02.wav");
    wav(&first);
    wav(&second);
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    scan(&mut library, id);
    std::fs::remove_file(&first).unwrap();
    std::fs::write(source.join("broken.flac"), b"invalid").unwrap();
    // A file that cannot be read does not make the traversal incomplete.
    let report = scan(&mut library, id);
    assert_eq!(report.progress.errors, 1);
    assert!(report.complete);
    assert_eq!(report.removed, 1);
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums[0].primary().tracks.len(), 1);
    // Unchanged failures are remembered rather than read again.
    let report = scan(&mut library, id);
    assert_eq!(report.progress.errors, 0);
    assert_eq!(report.progress.unchanged, 2);
    std::fs::remove_file(&second).unwrap();
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let report = library.scan(id, &cancellation, |_| {}).unwrap();
    assert!(report.cancelled && !report.complete);
    assert!(library.snapshot().unwrap().albums[0].primary().tracks[0].available);
    std::fs::remove_file(source.join("broken.flac")).unwrap();
    std::fs::write(source.join("notes.txt"), b"keep the source non-empty").unwrap();
    // Only non-audio files remain: indistinguishable from an emptied mount point.
    assert_eq!(scan(&mut library, id).removed, 0);
}

#[test]
fn rejects_storage_overlap_and_keeps_untagged_folders_separate() {
    let dir = tempfile::tempdir().unwrap();
    let storage = dir.path().join("data");
    let mut library = Library::open(&storage).unwrap();
    assert!(library.add_source(dir.path()).is_err());
    assert!(library.add_source(&storage).is_err());
    let source = dir.path().join("music");
    wav(&source.join("Artist/Album16/01.wav"));
    wav(&source.join("Artist/Album24/01.wav"));
    let id = library.add_source(&source).unwrap();
    assert!(library.add_source(&source.join("Artist")).is_err());
    scan(&mut library, id);
    assert_eq!(library.snapshot().unwrap().albums.len(), 2);
}

#[test]
fn tagged_editions_merge_and_rank_best_first() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    let fixtures = [
        ("CD/01.wav", 44_100, 16, "Kind of Blue", "1959"),
        ("HiRes/01.wav", 96_000, 24, "Kind of Blue", "1959"),
        ("Remaster/01.wav", 96_000, 24, "Kind of Blue", "1997"),
        ("Deluxe/01.wav", 44_100, 16, "Kind of Blue (Deluxe)", "1959"),
        ("NoYear/01.wav", 44_100, 16, "Kind of Blue", ""),
    ];
    for (path, rate, bits, album, year) in fixtures {
        let path = source.join("Miles Davis").join(path);
        pcm(&path, rate, bits);
        tag(&path, &album_tags(album, "Miles Davis", year), false);
    }
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    let report = scan(&mut library, id);
    assert_eq!(report.progress.errors, 0, "{:?}", report.diagnostics);
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 4);
    let merged = snapshot
        .albums
        .iter()
        .find(|album| album.editions.len() == 2)
        .unwrap();
    assert_eq!(merged.title, "Kind of Blue");
    assert_eq!(merged.artist, "Miles Davis");
    assert_eq!(
        merged.editions[0].tracks[0].metadata.sample_rate,
        Some(96_000)
    );
    assert!(merged.editions[1].folder.ends_with("CD"));
    let album_id = merged.id;
    // Album identity survives a rescan, so the interface keeps its selection.
    std::fs::remove_dir_all(source.join("Miles Davis/HiRes")).unwrap();
    scan(&mut library, id);
    let snapshot = library.snapshot().unwrap();
    let album = snapshot.albums.iter().find(|a| a.id == album_id).unwrap();
    assert_eq!(album.editions.len(), 1);
}

#[test]
fn disc_folders_and_untagged_compilations_form_one_album() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    wav(&source.join("Artist/Double/CD1/01.wav"));
    wav(&source.join("Artist/Double/CD2/01.wav"));
    for (file, artist) in [("01.wav", "Alpha"), ("02.wav", "Beta")] {
        let path = source.join("Compilations/Hits").join(file);
        wav(&path);
        let items = [
            (ItemKey::AlbumTitle, "Hits"),
            (ItemKey::TrackArtist, artist),
            (ItemKey::RecordingDate, "2001"),
        ];
        tag(&path, &items, false);
    }
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    scan(&mut library, id);
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 2);
    let double = snapshot
        .albums
        .iter()
        .find(|a| a.title == "Double")
        .unwrap();
    assert_eq!(double.editions.len(), 1);
    let discs: Vec<_> = double
        .primary()
        .tracks
        .iter()
        .map(|t| t.metadata.disc)
        .collect();
    assert_eq!(discs, [Some(1), Some(2)]);
    let hits = snapshot.albums.iter().find(|a| a.title == "Hits").unwrap();
    assert!(hits.compilation);
    assert_eq!(hits.artist, "");
    assert_eq!(hits.primary().tracks.len(), 2);
}

#[test]
fn folder_scans_only_forget_entries_inside_their_scope() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    wav(&source.join("Artist/Gone/01.wav"));
    wav(&source.join("Artist/Kept/01.wav"));
    let other = source.join("Other/Album/01.wav");
    wav(&other);
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    scan(&mut library, id);
    let root = &library.sources().unwrap()[0].path;
    std::fs::remove_dir_all(source.join("Artist/Gone")).unwrap();
    std::fs::remove_file(&other).unwrap();
    // The deleted folder resolves to its existing parent.
    let report = library
        .scan_scope(
            id,
            &[root.join("Artist").join("Gone")],
            &Cancellation::default(),
            |_| {},
            &mut (),
        )
        .unwrap();
    assert!(report.complete);
    assert_eq!(report.removed, 1);
    assert_eq!(report.progress.unchanged, 1);
    let titles: Vec<_> = library
        .snapshot()
        .unwrap()
        .albums
        .iter()
        .map(|a| a.title.clone())
        .collect();
    assert_eq!(titles, ["Kept", "Album"]);
}

#[test]
fn missing_cover_cache_is_regenerated_and_orphans_pruned() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    let audio = source.join("Artist/Album/01.wav");
    wav(&audio);
    tag(&audio, &album_tags("Album", "Artist", "2000"), true);
    let before = std::fs::read(&audio).unwrap();
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    scan(&mut library, id);
    let cover = library.snapshot().unwrap().albums[0]
        .cover()
        .unwrap()
        .clone();
    assert!(crate::covers::complete(&cover));
    std::fs::remove_file(&cover).unwrap();
    let report = scan(&mut library, id);
    assert_eq!(report.progress.indexed, 1);
    assert!(crate::covers::complete(&cover));
    let orphan = cover.with_file_name(format!("{}.png", "f".repeat(64)));
    std::fs::write(&orphan, b"x").unwrap();
    assert_eq!(library.prune_covers().unwrap(), 1);
    assert!(!orphan.exists() && crate::covers::complete(&cover));
    assert_eq!(std::fs::read(&audio).unwrap(), before);
}

#[test]
fn migrates_version_1_libraries_and_rereads_their_files() {
    let dir = tempfile::tempdir().unwrap();
    let storage = dir.path().join("data");
    std::fs::create_dir_all(&storage).unwrap();
    let source = dir.path().join("music").canonicalize_or_create();
    let audio = source.join("Artist").join("Album").join("01 - Song.wav");
    wav(&audio);
    let connection = Connection::open(storage.join("library.sqlite3")).unwrap();
    connection
        .execute_batch(include_str!("../schema_v1.sql"))
        .unwrap();
    let metadata = r#"{"title":"Song","artist":"Artist","album_artist":"Artist","album":"Album",
        "genre":"","year":null,"track":1,"disc":null,"duration_ms":0,"sample_rate":44100,
        "bit_depth":16,"channels":1,"format":"WAV","inferred":["album_artist"]}"#;
    connection
        .execute_batch(&format!(
            "INSERT INTO sources(id,path,available,generation) VALUES(1,'{}',1,1);
             INSERT INTO artists(id,name) VALUES(1,'Artist');
             INSERT INTO albums(id,identity,title,artist_id) VALUES(1,'legacy','Album',1);
             INSERT INTO editions(id,album_id,folder) VALUES(1,1,'{}');
             INSERT INTO tracks(id,edition_id,metadata) VALUES(1,1,'{metadata}');
             INSERT INTO files(source_id,path,track_id,size,modified,seen,available)
                VALUES(1,'{}',1,0,'0',1,1);",
            source.display(),
            audio.parent().unwrap().display(),
            audio.display()
        ))
        .unwrap();
    drop(connection);
    let mut library = Library::open(&storage).unwrap();
    assert_eq!(
        schema::version(&library.connection).unwrap(),
        schema::VERSION
    );
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 1);
    assert_eq!(snapshot.albums[0].primary().tracks[0].id, 1);
    let report = scan(&mut library, 1);
    assert_eq!(report.progress.indexed, 1);
    assert_eq!(
        library.snapshot().unwrap().albums[0].primary().tracks[0].id,
        1
    );
    assert!(Library::open_reader(&storage).is_ok());
}

trait CanonicalizeOrCreate {
    fn canonicalize_or_create(self) -> PathBuf;
}
impl CanonicalizeOrCreate for PathBuf {
    fn canonicalize_or_create(self) -> PathBuf {
        std::fs::create_dir_all(&self).unwrap();
        self.canonicalize().unwrap()
    }
}

#[test]
fn bounded_pipeline_processes_more_than_all_queue_capacities() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    for index in 0..180 {
        wav(&source.join(format!("Artist/Album/{index:03}.wav")));
    }
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    let report = scan(&mut library, id);
    assert_eq!(report.progress.indexed, 180);
    assert!(report.complete);
}

#[test]
fn removing_a_source_forgets_only_its_entries_and_keeps_files() {
    let dir = tempfile::tempdir().unwrap();
    let kept = dir.path().join("kept");
    let removed = dir.path().join("removed");
    wav(&kept.join("Artist/Kept/01 - Song.wav"));
    let audio = removed.join("Other/Gone/01 - Song.wav");
    wav(&audio);
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let kept_id = library.add_source(&kept).unwrap();
    let removed_id = library.add_source(&removed).unwrap();
    for id in [kept_id, removed_id] {
        scan(&mut library, id);
    }
    assert_eq!(library.snapshot().unwrap().albums.len(), 2);
    library.remove_source(removed_id).unwrap();
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.sources.len(), 1);
    assert_eq!(snapshot.sources[0].id, kept_id);
    assert_eq!(snapshot.albums.len(), 1);
    assert_eq!(snapshot.albums[0].primary().tracks[0].source_id, kept_id);
    assert!(audio.exists());
    // The folder can be added and indexed again afterwards.
    let again = library.add_source(&removed).unwrap();
    scan(&mut library, again);
    assert_eq!(library.snapshot().unwrap().albums.len(), 2);
}

#[test]
fn accent_variants_merge_and_older_identities_are_rekeyed() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    for (folder, album, artist) in [("CD", "Été", "Björk"), ("HiRes", "ETE", "Bjork")] {
        let path = source.join(folder).join("01.wav");
        wav(&path);
        tag(&path, &album_tags(album, artist, "1997"), false);
    }
    let storage = dir.path().join("data");
    let mut library = Library::open(&storage).unwrap();
    let id = library.add_source(&source).unwrap();
    scan(&mut library, id);
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 1);
    assert_eq!(snapshot.albums[0].editions.len(), 2);
    let tracks: Vec<_> = snapshot.albums[0].tracks().map(|t| t.id).collect();
    // Identities written by version 2 differ; reopening re-keys them.
    library
        .connection
        .execute_batch(
            "UPDATE editions SET identity=identity||'-v2'||id;
             UPDATE albums SET identity='v2';
             ALTER TABLE files DROP COLUMN added;
             DROP TABLE playlist_entries;
             DROP TABLE playlists;
             PRAGMA user_version=2;",
        )
        .unwrap();
    drop(library);
    let library = Library::open(&storage).unwrap();
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 1);
    assert_eq!(snapshot.albums[0].editions.len(), 2);
    let mut again: Vec<_> = snapshot.albums[0].tracks().map(|t| t.id).collect();
    again.sort();
    let mut tracks = tracks;
    tracks.sort();
    assert_eq!(again, tracks);
}

/// Lets one playlist write through, at the first check of the scan.
struct PlaylistWrite {
    waiting: bool,
    runs: usize,
}

impl crate::Interlude for PlaylistWrite {
    fn waiting(&self) -> bool {
        self.waiting
    }
    fn run(&mut self, library: &mut Library) {
        assert!(
            library.connection.is_autocommit(),
            "Interlude ran inside the scan transaction"
        );
        library
            .change_playlist(rap_core::playlist::PlaylistChange::Create {
                name: "During scan".into(),
                tracks: Vec::new(),
            })
            .unwrap();
        self.waiting = false;
        self.runs += 1;
    }
}

#[test]
fn waiting_writes_pass_between_batches_and_the_scan_completes() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    for index in 1..=3 {
        wav(&music.join(format!("Artist/Album/{index:02} - Track.wav")));
    }
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&music).unwrap();
    let mut interlude = PlaylistWrite {
        waiting: true,
        runs: 0,
    };
    let report = library
        .scan_scope(id, &[], &Cancellation::default(), |_| {}, &mut interlude)
        .unwrap();
    assert_eq!(interlude.runs, 1);
    assert!(report.complete);
    assert_eq!(report.progress.indexed, 3);
    assert_eq!(library.playlists().unwrap().len(), 1);
    let albums = library.snapshot().unwrap().albums;
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].editions[0].tracks.len(), 3);
}

/// A file name that is valid on the platform but has no Unicode representation.
fn non_unicode(dir: &Path) -> PathBuf {
    #[cfg(windows)]
    let name = {
        use std::os::windows::ffi::OsStringExt;
        // An unpaired surrogate, as some old tools wrote.
        std::ffi::OsString::from_wide(&[0x0041, 0xD800, 0x002E, 0x0077, 0x0061, 0x0076])
    };
    #[cfg(unix)]
    let name = {
        use std::os::unix::ffi::OsStrExt;
        std::ffi::OsStr::from_bytes(b"A\xff.wav").to_os_string()
    };
    dir.join(name)
}

#[test]
fn non_unicode_paths_are_reported_without_failing_the_scan() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    let album = source.join("Artist/Album");
    wav(&album.join("01.wav"));
    wav(&album.join("02.wav"));
    let odd = non_unicode(&album);
    std::fs::copy(album.join("01.wav"), &odd).unwrap();
    let odd_dir = non_unicode(&source.join("Artist"));
    wav(&odd_dir.join("01.wav"));
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    let report = scan(&mut library, id);
    assert!(report.complete);
    assert_eq!(report.progress.indexed, 2);
    assert_eq!(report.progress.errors, 2);
    // Removals are still concluded on the rest of the source.
    std::fs::remove_file(album.join("02.wav")).unwrap();
    let report = scan(&mut library, id);
    assert!(report.complete);
    assert_eq!(report.removed, 1);
    // A notification inside the odd directory rescans its nearest Unicode ancestor.
    let root = library.sources().unwrap()[0].path.clone();
    let scoped = non_unicode(&root.join("Artist"));
    let report = library
        .scan_scope(id, &[scoped], &Cancellation::default(), |_| {}, &mut ())
        .unwrap();
    assert!(report.complete);
    assert_eq!(report.progress.unchanged, 1);
}
