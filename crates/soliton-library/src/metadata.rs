use crate::{covers::cache_cover, grouping};
use anyhow::Result;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    picture::PictureType,
    prelude::Accessor,
    probe::Probe,
    tag::ItemKey,
};
use soliton_core::model::Metadata;
use std::path::{Path, PathBuf};

/// Version of the extraction rules. Files read by an older version are re-read.
pub(crate) const EXTRACTOR: i64 = 2;

pub(crate) struct Extracted {
    pub metadata: Metadata,
    pub cover: Option<PathBuf>,
    pub warnings: Vec<String>,
    /// Creation time in seconds since the epoch, when the system records it.
    pub created: Option<i64>,
}

pub(crate) fn read(path: &Path, covers: &Path) -> Result<Extracted> {
    // Probe opens the source read-only. Never call any lofty save API here.
    let tagged = Probe::open(path)?.guess_file_type()?.read()?;
    let tags: Vec<_> = tagged
        .primary_tag()
        .into_iter()
        .chain(
            tagged
                .tags()
                .iter()
                .filter(|tag| Some(tag.tag_type()) != tagged.primary_tag().map(|t| t.tag_type())),
        )
        .collect();
    let first = |key| {
        tags.iter().find_map(|t| {
            t.get_string(key)
                .filter(|s| !s.trim().is_empty())
                .map(clean)
        })
    };
    let properties = tagged.properties();
    let mut metadata = Metadata {
        title: first(ItemKey::TrackTitle).unwrap_or_default(),
        artist: first(ItemKey::TrackArtist).unwrap_or_default(),
        album_artist: first(ItemKey::AlbumArtist).unwrap_or_default(),
        album: first(ItemKey::AlbumTitle).unwrap_or_default(),
        genre: first(ItemKey::Genre).unwrap_or_default(),
        year: first(ItemKey::RecordingDate)
            .or_else(|| first(ItemKey::Year))
            .and_then(|date| date.get(..4)?.parse().ok()),
        track: tags.iter().find_map(|t| t.track()),
        disc: tags.iter().find_map(|t| t.disk()),
        compilation: first(ItemKey::FlagCompilation)
            .is_some_and(|flag| matches!(flag.to_ascii_lowercase().as_str(), "1" | "true" | "yes")),
        duration_ms: properties.duration().as_millis() as u64,
        sample_rate: properties.sample_rate(),
        bit_depth: properties.bit_depth(),
        channels: properties.channels(),
        format: path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_uppercase(),
        inferred: Vec::new(),
    };
    infer(&mut metadata, path);
    let mut warnings = Vec::new();
    let mut cover = None;
    let pictures: Vec<_> = tags.iter().flat_map(|t| t.pictures()).collect();
    for picture in pictures
        .iter()
        .filter(|p| p.pic_type() == PictureType::CoverFront)
        .chain(
            pictures
                .iter()
                .filter(|p| p.pic_type() != PictureType::CoverFront),
        )
    {
        match cache_cover(picture.data(), covers) {
            Ok(path) => {
                cover = Some(path);
                break;
            }
            Err(error) => warnings.push(format!("{}: {error:#}", path.display())),
        }
    }
    let created = std::fs::metadata(path)
        .and_then(|m| m.created())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|time| time.as_secs() as i64);
    Ok(Extracted {
        metadata,
        cover,
        warnings,
        created,
    })
}

fn clean(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(4096)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn infer(metadata: &mut Metadata, path: &Path) {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let number_end = stem
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(stem.len());
    let number = (number_end > 0 && number_end <= 3)
        .then(|| stem[..number_end].parse::<u32>().ok())
        .flatten();
    if metadata.title.is_empty() {
        let title = if number.is_some() {
            stem[number_end..].trim_start_matches([' ', '-', '.', '_'])
        } else {
            &stem
        };
        metadata.title = clean(if title.is_empty() { &stem } else { title });
        metadata.inferred.push("title".into());
    }
    if metadata.track.is_none() && number.is_some() {
        metadata.track = number;
        metadata.inferred.push("track".into());
    }
    let disc = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| grouping::disc_folder(&name.to_string_lossy()));
    if metadata.disc.is_none() && disc.is_some() {
        metadata.disc = disc;
        metadata.inferred.push("disc".into());
    }
    // `Artist/Album/CD1/01.flac` reads like `Artist/Album/01.flac`.
    let album = grouping::album_folder(path);
    let folder_name = |folder: Option<&Path>| {
        folder
            .and_then(Path::file_name)
            .map(|v| clean(&v.to_string_lossy()))
            .unwrap_or_default()
    };
    if metadata.album.is_empty() {
        metadata.album = folder_name(album);
        metadata.inferred.push("album".into());
    }
    if metadata.artist.is_empty() {
        metadata.artist = folder_name(album.and_then(Path::parent));
        metadata.inferred.push("artist".into());
    }
    if metadata.album_artist.is_empty() {
        // Inferred values never join editions; compilations are resolved per edition.
        metadata.album_artist = if !metadata.is_inferred("artist") {
            metadata.artist.clone()
        } else {
            folder_name(album.and_then(Path::parent))
        };
        metadata.inferred.push("album_artist".into());
    }
}

#[cfg(test)]
#[path = "tests/metadata.rs"]
pub(crate) mod tests;
