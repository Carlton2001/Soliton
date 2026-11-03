//! The compact now-playing preview. Only the desktop worker reads/decodes artwork.
use super::bitmap::{Frame, Surface};
use crate::MediaState;
use anyhow::Result;
use soliton_core::config::Language;
use std::path::PathBuf;

const WIDTH: i32 = 640;
const MARGIN: i32 = 24;
const RAIL: i32 = 12;
/// Backdrop artwork stays quiet so the text keeps its contrast.
const BACKDROP_OPACITY: u8 = 56;

#[derive(Default)]
pub(super) struct Renderer {
    cover_key: Option<(PathBuf, i32)>,
    cover: Option<image::RgbaImage>,
}

impl Renderer {
    /// Renders at `aspect` (height per 1000 width) so DWM never stretches the frame.
    pub fn render(&mut self, state: &MediaState, language: Language, aspect: u32) -> Result<Frame> {
        let frame_height = height(aspect);
        let colors = state.preview;
        let mut surface = Surface::new(WIDTH, frame_height, colors.background)?;
        let path = state.track.as_ref().and_then(|track| track.cover.as_ref());
        let key = path.map(|path| (path.clone(), frame_height));
        if self.cover_key != key {
            self.cover_key = key;
            self.cover = path.and_then(|path| {
                (|| -> Result<image::RgbaImage> {
                    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
                    let mut limits = image::Limits::default();
                    limits.max_image_width = Some(2048);
                    limits.max_image_height = Some(2048);
                    limits.max_alloc = Some(16 * 1024 * 1024);
                    reader.limits(limits);
                    Ok(reader
                        .decode()?
                        .resize_to_fill(
                            WIDTH as u32,
                            frame_height as u32,
                            image::imageops::FilterType::Triangle,
                        )
                        .into_rgba8())
                })()
                .ok()
            });
        }
        if let Some(backdrop) = &self.cover {
            surface.image(backdrop, 0, 0, BACKDROP_OPACITY);
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
        let (left, width) = (MARGIN, WIDTH - MARGIN * 2);
        surface.text(title, (left, MARGIN, width, 80), 50, true, colors.text)?;
        surface.text(
            artist,
            (left, MARGIN + 86, width, 62),
            44,
            false,
            colors.text,
        )?;
        surface.text(
            album,
            (left, MARGIN + 150, width, 54),
            38,
            false,
            colors.text,
        )?;
        let current = duration(state.position_ms.min(state.duration_ms));
        let total = duration(state.duration_ms);
        let times = (MARGIN, frame_height - RAIL - 66, WIDTH - MARGIN * 2, 58);
        surface.figures(&current, times, 40, colors.text, false)?;
        surface.figures(&total, times, 40, colors.text, true)?;
        // Complete GDI text before writing the DIB directly for the progress bar.
        unsafe {
            windows::Win32::Graphics::Gdi::GdiFlush().ok()?;
        }
        surface.rect(
            0,
            frame_height - RAIL,
            WIDTH,
            RAIL,
            colors.progress_background,
        );
        let filled = progress(state.position_ms, state.duration_ms, WIDTH);
        surface.rect(0, frame_height - RAIL, filled, RAIL, colors.accent);
        surface.frame()
    }
}

pub(super) fn height(aspect: u32) -> i32 {
    (WIDTH as u32 * aspect / 1000).clamp(300, 480) as i32
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
