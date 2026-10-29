use super::*;
use lofty::{
    config::WriteOptions,
    picture::{MimeType, Picture},
    prelude::TagExt,
    tag::{Tag, TagType},
};
use std::io::{Cursor, Write};

pub(crate) fn wav(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x44\xac\0\0\x88\x58\x01\0\x02\0\x10\0data\x02\0\0\0\0\0").unwrap();
}

#[test]
fn embedded_tags_cover_and_source_remain_intact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Artiste/Album/01 - Déduction.wav");
    wav(&path);
    let mut tag = Tag::new(TagType::Id3v2);
    tag.insert_text(ItemKey::TrackTitle, "Été à Rome".into());
    tag.insert_text(ItemKey::TrackArtist, "Invité".into());
    tag.insert_text(ItemKey::AlbumArtist, "Collectif".into());
    tag.insert_text(ItemKey::AlbumTitle, "Compilation".into());
    tag.insert_text(ItemKey::FlagCompilation, "1".into());
    let pixels = image::RgbaImage::from_pixel(2, 1, image::Rgba([220, 20, 50, 255]));
    let mut png = Cursor::new(Vec::new());
    pixels.write_to(&mut png, image::ImageFormat::Png).unwrap();
    tag.push_picture(
        Picture::unchecked(png.into_inner())
            .pic_type(PictureType::CoverFront)
            .mime_type(MimeType::Png)
            .build(),
    );
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    let before = std::fs::read(&path).unwrap();
    let covers = dir.path().join("cache");
    std::fs::create_dir(&covers).unwrap();
    let value = read(&path, dir.path(), &covers).unwrap();
    assert_eq!(value.metadata.title, "Été à Rome");
    assert_eq!(value.metadata.artist, "Invité");
    assert_eq!(value.metadata.album_artist, "Collectif");
    assert!(value.metadata.compilation);
    assert!(!value.metadata.inferred.contains(&"title".into()));
    assert_eq!(value.metadata.sample_rate, Some(44100));
    assert!(value.cover.unwrap().exists());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn fallback_marks_provenance_and_corruption_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Artiste/Album/03 - Chanson.wav");
    wav(&path);
    let value = read(&path, dir.path(), dir.path()).unwrap();
    assert_eq!(value.metadata.title, "Chanson");
    assert_eq!(value.metadata.album, "Album");
    assert_eq!(value.metadata.artist, "Artiste");
    assert_eq!(value.metadata.track, Some(3));
    assert!(value.metadata.inferred.contains(&"title".into()));
    let disc = dir.path().join("Artiste/Double/CD2/01.wav");
    wav(&disc);
    let value = read(&disc, dir.path(), dir.path()).unwrap();
    assert_eq!(value.metadata.album, "Double");
    assert_eq!(value.metadata.artist, "Artiste");
    assert_eq!(value.metadata.disc, Some(2));
    assert!(value.metadata.is_inferred("disc"));
    std::fs::write(&path, b"broken").unwrap();
    assert!(read(&path, dir.path(), dir.path()).is_err());
}

#[test]
fn names_are_never_inferred_from_the_source_root_or_above() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("Musique");
    let loose = root.join("01 - Loose.wav");
    let folder = root.join("Resolve/02 - Grouped.wav");
    wav(&loose);
    wav(&folder);
    let value = read(&loose, &root, dir.path()).unwrap();
    assert_eq!(value.metadata.title, "Loose");
    assert_eq!(
        (
            value.metadata.album.as_str(),
            value.metadata.artist.as_str()
        ),
        ("", "")
    );
    assert!(value.metadata.is_inferred("album") && value.metadata.is_inferred("artist"));
    let value = read(&folder, &root, dir.path()).unwrap();
    assert_eq!(value.metadata.album, "Resolve");
    assert_eq!(value.metadata.artist, "");
    assert_eq!(value.metadata.album_artist, "");
}
