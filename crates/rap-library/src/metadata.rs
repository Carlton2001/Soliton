use anyhow::Result;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    picture::PictureType,
    prelude::Accessor,
    probe::Probe,
    tag::ItemKey,
};
use rap_core::model::Metadata;
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

pub(crate) struct Extracted {
    pub metadata: Metadata,
    pub cover: Option<PathBuf>,
    pub warnings: Vec<String>,
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
    Ok(Extracted {
        metadata,
        cover,
        warnings,
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
    let parent = path.parent();
    if metadata.album.is_empty() {
        metadata.album = parent
            .and_then(Path::file_name)
            .map(|v| clean(&v.to_string_lossy()))
            .unwrap_or_default();
        metadata.inferred.push("album".into());
    }
    if metadata.artist.is_empty() {
        metadata.artist = parent
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .map(|v| clean(&v.to_string_lossy()))
            .unwrap_or_default();
        metadata.inferred.push("artist".into());
    }
    if metadata.album_artist.is_empty() {
        // Folder identity groups untagged compilations without conflating track artists.
        metadata.album_artist = if !metadata.inferred.contains(&"artist".into()) {
            metadata.artist.clone()
        } else {
            parent
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .map(|v| clean(&v.to_string_lossy()))
                .unwrap_or_default()
        };
        metadata.inferred.push("album_artist".into());
    }
}

const MAX_COVER_SIDE: u32 = 8192;
const MAX_COVER_ALLOCATION: u64 = 288 * 1024 * 1024;
const SMALL_COVER_ALLOCATION: u64 = 64 * 1024 * 1024;
static LARGE_COVERS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn cache_cover(bytes: &[u8], directory: &Path) -> Result<PathBuf> {
    anyhow::ensure!(
        bytes.len() <= 16 * 1024 * 1024,
        "Embedded cover exceeds 16 MiB"
    );
    let hash = blake3::hash(bytes).to_hex().to_string();
    let full = directory.join(format!("{hash}.png"));
    let small = directory.join(format!("{hash}.small.png"));
    if full.exists() && small.exists() {
        return Ok(full);
    }
    let reader = || -> Result<_> {
        let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_COVER_SIDE);
        limits.max_image_height = Some(MAX_COVER_SIDE);
        limits.max_alloc = Some(MAX_COVER_ALLOCATION);
        reader.limits(limits);
        Ok(reader)
    };
    // High-resolution scans (5000+ px) decode to ~100 MiB; decode them one at a time
    // so parallel readers cannot multiply that peak.
    let (width, height) = reader()?.into_dimensions()?;
    let large = u64::from(width) * u64::from(height) * 4 > SMALL_COVER_ALLOCATION;
    let _guard = large.then(|| LARGE_COVERS.lock().unwrap_or_else(|e| e.into_inner()));
    let image = reader()?.decode()?;
    for (path, side) in [(&full, 512), (&small, 160)] {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let temporary = path.with_extension(format!(
            "{}.{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        image
            .thumbnail(side, side)
            .save_with_format(&temporary, image::ImageFormat::Png)?;
        // Concurrent identical covers are harmless; the destination is content-addressed.
        if let Err(error) = std::fs::rename(&temporary, path) {
            let _ = std::fs::remove_file(&temporary);
            if !path.exists() {
                return Err(error.into());
            }
        }
    }
    Ok(full)
}

#[cfg(test)]
#[path = "tests/metadata.rs"]
mod tests;
