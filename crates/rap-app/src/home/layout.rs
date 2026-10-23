//! Automatic, incompressible column widths, in unscaled design pixels.
use super::*;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct HomeMetrics {
    artists: f32,
    albums: HashMap<i64, f32>,
    tracks: HashMap<i64, f32>,
}

impl HomeMetrics {
    /// Shape on the query worker using independent font faces. Never share a
    /// WindowTextSystem: its frame cache locks are not ordered for concurrent
    /// shaping and finish_frame, and can deadlock the renderer and query worker.
    pub fn measure(snapshot: &LibrarySnapshot, facets: &Facets) -> Self {
        let data = include_bytes!("../../../../assets/fonts/Inter-Regular.ttf");
        let face = |weight| {
            let mut face =
                rustybuzz::Face::from_slice(data, 0).expect("Invalid bundled Inter font");
            face.set_variations(&[rustybuzz::Variation {
                tag: rustybuzz::ttf_parser::Tag::from_bytes(b"wght"),
                value: weight,
            }]);
            face
        };
        let regular = face(400.);
        let semibold = face(600.);
        let width = |value: &str, size: f32, weight: FontWeight| {
            let value = value.replace(['\r', '\n'], " ");
            let face = if weight == FontWeight::SEMIBOLD {
                &semibold
            } else {
                &regular
            };
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(&value);
            buffer.guess_segment_properties();
            let glyphs = rustybuzz::shape(face, &[], buffer);
            let units = face.units_per_em() as f32;
            glyphs
                .glyph_positions()
                .iter()
                .zip(glyphs.glyph_infos())
                .map(|(position, info)| {
                    let advance = position.x_advance.unsigned_abs() as f32;
                    // Allow an em for a native fallback glyph missing from Inter.
                    if info.glyph_id == 0 {
                        advance.max(units)
                    } else {
                        advance
                    }
                })
                .sum::<f32>()
                * size
                / units
        };
        let artists = facets
            .artists
            .iter()
            .map(|name| width(name, 13., FontWeight::NORMAL))
            .fold(0., f32::max);
        let albums = snapshot
            .albums
            .iter()
            .map(|album| {
                let meta = &album.primary().tracks[0].metadata;
                let details_width = width(&quality(meta), 12., FontWeight::NORMAL)
                    + meta.year.map_or(0., |year| {
                        width(&year.to_string(), 12., FontWeight::SEMIBOLD)
                            + width(" · ", 12., FontWeight::NORMAL)
                    });
                let content = width(&album.title, 13., FontWeight::SEMIBOLD)
                    .max(width(&album.artist, 12., FontWeight::NORMAL))
                    .max(width("Artistes divers", 12., FontWeight::NORMAL))
                    .max(width("Various artists", 12., FontWeight::NORMAL))
                    .max(details_width);
                // Outside inset, row padding, cover, gap, and column border.
                (album.id, (content.ceil() + 125.).max(128.))
            })
            .collect();
        let tracks = snapshot
            .albums
            .iter()
            .flat_map(|album| &album.editions)
            .map(|edition| {
                let content = edition
                    .tracks
                    .iter()
                    .map(|track| {
                        width(&track.metadata.title, 13., FontWeight::SEMIBOLD).max(width(
                            &format!("{} · {}", quality(&track.metadata), track.metadata.artist),
                            12.,
                            FontWeight::NORMAL,
                        ))
                    })
                    .fold(0., f32::max);
                // Transport, number, duration, Actions, row padding, gaps and rail gutter.
                (edition.id, content.ceil() + 304.)
            })
            .collect();
        Self {
            tracks,
            // Outside inset, row padding and column border.
            artists: (artists.ceil() + 49.).max(96.),
            albums,
        }
    }
}

impl HomeMetrics {
    pub(super) fn artists_width(&self) -> f32 {
        self.artists.max(96.)
    }
    pub(super) fn album_width(&self, album: i64) -> f32 {
        self.albums.get(&album).copied().unwrap_or(128.).max(128.)
    }
    pub(crate) fn tracks_width(&self, edition: i64) -> f32 {
        self.tracks.get(&edition).copied().unwrap_or(512.).max(512.)
    }
}

#[cfg(test)]
#[path = "../tests/home_layout.rs"]
mod tests;
