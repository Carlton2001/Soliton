//! Content-addressed thumbnails of embedded covers, inside Soliton storage only.
//!
//! Each embedded image is hashed; a changed image yields a new name, so a file
//! rewritten with another cover never shows a stale thumbnail. Unreferenced
//! thumbnails are pruned after complete scans.
use anyhow::Result;
use std::{
    collections::HashSet,
    io::Cursor,
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_COVER_SIDE: u32 = 8192;
const MAX_COVER_ALLOCATION: u64 = 288 * 1024 * 1024;
const SMALL_COVER_ALLOCATION: u64 = 64 * 1024 * 1024;
static LARGE_COVERS: std::sync::Mutex<()> = std::sync::Mutex::new(());
/// Temporary files younger than this may belong to a scan still writing them.
const STALE_TEMPORARY: Duration = Duration::from_secs(3600);

/// Large and small thumbnails exist for this cache entry.
pub(crate) fn complete(full: &Path) -> bool {
    full.is_file() && small(full).is_file()
}

fn small(full: &Path) -> PathBuf {
    full.with_extension("small.png")
}

pub(crate) fn cache_cover(bytes: &[u8], directory: &Path) -> Result<PathBuf> {
    anyhow::ensure!(
        bytes.len() <= 16 * 1024 * 1024,
        "Embedded cover exceeds 16 MiB"
    );
    let hash = blake3::hash(bytes).to_hex().to_string();
    let full = directory.join(format!("{hash}.png"));
    if complete(&full) {
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
    for (path, side) in [(full.clone(), 512), (small(&full), 160)] {
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
        if let Err(error) = std::fs::rename(&temporary, &path) {
            let _ = std::fs::remove_file(&temporary);
            if !path.exists() {
                return Err(error.into());
            }
        }
    }
    Ok(full)
}

/// Removes cache files not referenced by `keep` (large thumbnail paths) and old
/// temporary files. Only touches entries named by this module inside `directory`.
/// Returns the number of removed files.
pub(crate) fn prune(directory: &Path, keep: &HashSet<PathBuf>) -> Result<usize> {
    let keep: HashSet<_> = keep
        .iter()
        .filter_map(|path| path.file_name().map(|name| name.to_os_string()))
        .collect();
    let mut removed = 0;
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let text = name.to_string_lossy();
        let Some(hash) = text.split('.').next().filter(|hash| is_hash(hash)) else {
            continue;
        };
        let obsolete = if text.ends_with(".tmp") {
            entry
                .metadata()?
                .modified()?
                .elapsed()
                .is_ok_and(|age| age > STALE_TEMPORARY)
        } else if text == format!("{hash}.png") || text == format!("{hash}.small.png") {
            !keep.contains(std::ffi::OsStr::new(&format!("{hash}.png")))
        } else {
            false
        };
        if obsolete && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

fn is_hash(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
#[path = "tests/covers.rs"]
mod tests;
