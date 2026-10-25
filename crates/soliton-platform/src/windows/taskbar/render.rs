//! The compact now-playing preview. Only the desktop worker reads/decodes artwork.
use super::bitmap::{Frame, Surface};
use crate::MediaState;
use anyhow::Result;
use soliton_core::config::Language;
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct Renderer {
    cover_path: Option<PathBuf>,
    cover: Option<image::RgbaImage>,
}

impl Renderer {
    pub fn render(&mut self, state: &MediaState, language: Language) -> Result<Frame> {
        let colors = state.preview;
        let mut surface = Surface::new(640, 360, colors.background)?;
        let path = state.track.as_ref().and_then(|track| track.cover.as_ref());
        if self.cover_path.as_ref() != path {
            self.cover_path = path.cloned();
            self.cover = path.and_then(|path| {
                (|| -> Result<image::RgbaImage> {
                    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
                    let mut limits = image::Limits::default();
                    limits.max_image_width = Some(2048);
                    limits.max_image_height = Some(2048);
                    limits.max_alloc = Some(16 * 1024 * 1024);
                    reader.limits(limits);
                    Ok(reader.decode()?.thumbnail(180, 180).into_rgba8())
                })()
                .ok()
            });
        }
        if let Some(image) = &self.cover {
            surface.image(
                image,
                24 + (180 - image.width() as i32) / 2,
                28 + (180 - image.height() as i32) / 2,
            );
        } else {
            // An empty cover keeps the same quiet shape as the library's placeholder.
            surface.rect(24, 28, 180, 180, colors.muted);
            surface.rect(28, 32, 172, 172, colors.background);
            surface.text("Soliton", (40, 92, 150, 48), 36, true, colors.muted)?;
        }
        let (title, artist, album) = state
            .track
            .as_ref()
            .map(|track| {
                (
                    track.title.as_str(),
                    track.artist.as_str(),
                    track.album.as_str(),
                )
            })
            .unwrap_or((
                if language == Language::French {
                    "Aucune lecture en cours"
                } else {
                    "Nothing playing"
                },
                "",
                "",
            ));
        // Geometry is fixed; long metadata is ellipsized by GDI, never clipped into controls.
        surface.text(title, (230, 28, 386, 78), 32, true, colors.text)?;
        surface.text(artist, (230, 104, 386, 40), 25, false, colors.muted)?;
        surface.text(album, (230, 146, 386, 40), 23, false, colors.muted)?;
        let current = duration(state.position_ms.min(state.duration_ms));
        let total = duration(state.duration_ms);
        surface.text(
            &format!("{current} / {total}"),
            (24, 258, 592, 38),
            24,
            false,
            colors.muted,
        )?;
        // Complete GDI text before writing the DIB directly for the progress bar.
        unsafe {
            windows::Win32::Graphics::Gdi::GdiFlush().ok()?;
        }
        surface.rect(24, 310, 592, 6, colors.progress_background);
        let filled = progress(state.position_ms, state.duration_ms, 592);
        surface.rect(24, 310, filled, 6, colors.accent);
        surface.frame()
    }
}

fn duration(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

fn progress(position: u64, duration: u64, width: i32) -> i32 {
    if duration == 0 {
        return 0;
    }
    ((u128::from(position.min(duration)) * width as u128) / u128::from(duration)) as i32
}

#[cfg(test)]
#[path = "../tests/taskbar_render.rs"]
mod tests;
