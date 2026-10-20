use crate::i18n::{Text, text};
use rap_core::{config::Language, model::LibrarySnapshot};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct Facets {
    pub artists: Vec<String>,
    pub genres: Vec<String>,
    pub years: Vec<String>,
    pub formats: Vec<String>,
}
impl Facets {
    pub fn collect(snapshot: &LibrarySnapshot) -> Self {
        use std::collections::BTreeSet;
        let mut artists = BTreeSet::new();
        let mut genres = BTreeSet::new();
        let mut years = BTreeSet::new();
        let mut formats = BTreeSet::new();
        for album in &snapshot.albums {
            if !album.artist.is_empty() {
                artists.insert(album.artist.clone());
            }
            for track in album.tracks() {
                let m = &track.metadata;
                if !m.artist.is_empty() {
                    artists.insert(m.artist.clone());
                }
                if !m.genre.is_empty() {
                    genres.insert(m.genre.clone());
                }
                if let Some(year) = m.year {
                    years.insert(year.to_string());
                }
                formats.insert(m.format.clone());
            }
        }
        Self {
            artists: artists.into_iter().collect(),
            genres: genres.into_iter().collect(),
            years: years.into_iter().collect(),
            formats: formats.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub query: String,
    pub artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<String>,
    pub format: Option<String>,
}

/// A snapshot with its facets and lowercase search text, built once per library
/// change so that each keystroke only compares prepared strings.
pub struct Catalog {
    pub snapshot: Arc<LibrarySnapshot>,
    pub facets: Arc<Facets>,
    albums: Vec<AlbumText>,
}

struct AlbumText {
    /// Title and artist, one per line.
    album: String,
    /// Title, artist and genre of every track of every edition, in snapshot order.
    tracks: Vec<String>,
}

impl Catalog {
    pub fn new(snapshot: LibrarySnapshot) -> Self {
        let albums = snapshot
            .albums
            .iter()
            .map(|album| {
                let mut haystack = format!("{}\n{}", album.title, album.artist);
                if album.compilation && album.artist.is_empty() {
                    // Compilations are found under their displayed label in both languages.
                    for language in [Language::French, Language::English] {
                        haystack.push('\n');
                        haystack.push_str(text(language, Text::VariousArtists));
                    }
                }
                AlbumText {
                    album: haystack.to_lowercase(),
                    tracks: album
                        .tracks()
                        .map(|track| {
                            let m = &track.metadata;
                            format!("{}\n{}\n{}", m.title, m.artist, m.genre).to_lowercase()
                        })
                        .collect(),
                }
            })
            .collect();
        Self {
            facets: Arc::new(Facets::collect(&snapshot)),
            snapshot: Arc::new(snapshot),
            albums,
        }
    }

    pub fn matches(&self, filter: &Filter) -> Vec<usize> {
        let query = filter.query.to_lowercase();
        self.snapshot
            .albums
            .iter()
            .zip(&self.albums)
            .enumerate()
            .filter_map(|(index, (album, text))| {
                let album_match = query.is_empty() || text.album.contains(&query);
                let any = album.tracks().zip(&text.tracks).any(|(track, track_text)| {
                    let m = &track.metadata;
                    (album_match || track_text.contains(&query))
                        && filter
                            .artist
                            .as_ref()
                            .is_none_or(|artist| &album.artist == artist || &m.artist == artist)
                        && filter.genre.as_ref().is_none_or(|genre| &m.genre == genre)
                        && filter
                            .year
                            .as_ref()
                            .is_none_or(|year| m.year.map(|y| y.to_string()).as_ref() == Some(year))
                        && filter
                            .format
                            .as_ref()
                            .is_none_or(|format| &m.format == format)
                });
                any.then_some(index)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/catalog.rs"]
mod tests;
