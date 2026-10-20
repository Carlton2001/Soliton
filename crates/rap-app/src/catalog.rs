use crate::i18n::{Text, text};
use rap_core::{config::Language, model::LibrarySnapshot, text::fold};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Default)]
pub struct Facets {
    pub artists: Vec<String>,
    pub genres: Vec<String>,
    pub years: Vec<String>,
    pub formats: Vec<String>,
}
impl Facets {
    pub fn collect(snapshot: &LibrarySnapshot) -> Self {
        let mut artists = Variants::default();
        let mut genres = Variants::default();
        let mut years = BTreeSet::new();
        let mut formats = BTreeSet::new();
        for album in &snapshot.albums {
            artists.add(&album.artist);
            for track in album.tracks() {
                let m = &track.metadata;
                artists.add(&m.artist);
                genres.add(&m.genre);
                if let Some(year) = m.year {
                    years.insert(year.to_string());
                }
                formats.insert(m.format.clone());
            }
        }
        Self {
            artists: artists.into_values(),
            genres: genres.into_values(),
            years: years.into_iter().collect(),
            formats: formats.into_iter().collect(),
        }
    }
}

/// Spellings of tag values that differ only by case or spacing, grouped under
/// one entry sorted by its folded form.
#[derive(Default)]
struct Variants<'a> {
    groups: BTreeMap<String, Vec<(&'a str, usize)>>,
}
impl<'a> Variants<'a> {
    fn add(&mut self, value: &'a str) {
        if value.is_empty() {
            return;
        }
        let spellings = self.groups.entry(fold(value)).or_default();
        match spellings
            .iter_mut()
            .find(|(spelling, _)| *spelling == value)
        {
            Some((_, count)) => *count += 1,
            None => spellings.push((value, 1)),
        }
    }
    /// Displays the most frequent spelling, the first met on ties.
    fn into_values(self) -> Vec<String> {
        self.groups
            .into_values()
            .filter_map(|spellings| {
                let best = spellings.iter().map(|(_, count)| *count).max()?;
                let (spelling, _) = spellings.into_iter().find(|(_, count)| *count == best)?;
                Some(spelling.to_owned())
            })
            .collect()
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

/// A snapshot with its facets and folded search keys, built once per library
/// change so that each keystroke only compares prepared strings. Case and spacing
/// differences between tags never hide an album.
pub struct Catalog {
    pub snapshot: Arc<LibrarySnapshot>,
    pub facets: Arc<Facets>,
    albums: Vec<AlbumText>,
}

struct AlbumText {
    /// Title and artist, one per line.
    album: String,
    artist: String,
    /// Every track of every edition, in snapshot order.
    tracks: Vec<TrackText>,
}

struct TrackText {
    /// Title, artist and genre, one per line.
    text: String,
    artist: String,
    genre: String,
}

impl Catalog {
    pub fn new(snapshot: LibrarySnapshot) -> Self {
        let albums = snapshot
            .albums
            .iter()
            .map(|album| {
                let mut haystack = format!("{}\n{}", fold(&album.title), fold(&album.artist));
                if album.compilation && album.artist.is_empty() {
                    // Compilations are found under their displayed label in both languages.
                    for language in [Language::French, Language::English] {
                        haystack.push('\n');
                        haystack.push_str(&fold(text(language, Text::VariousArtists)));
                    }
                }
                AlbumText {
                    album: haystack,
                    artist: fold(&album.artist),
                    tracks: album
                        .tracks()
                        .map(|track| {
                            let m = &track.metadata;
                            let (artist, genre) = (fold(&m.artist), fold(&m.genre));
                            TrackText {
                                text: format!("{}\n{artist}\n{genre}", fold(&m.title)),
                                artist,
                                genre,
                            }
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
        let query = fold(&filter.query);
        let artist = filter.artist.as_deref().map(fold);
        let genre = filter.genre.as_deref().map(fold);
        self.snapshot
            .albums
            .iter()
            .zip(&self.albums)
            .enumerate()
            .filter_map(|(index, (album, text))| {
                let album_match = query.is_empty() || text.album.contains(&query);
                let any = album.tracks().zip(&text.tracks).any(|(track, track_text)| {
                    let m = &track.metadata;
                    (album_match || track_text.text.contains(&query))
                        && artist
                            .as_ref()
                            .is_none_or(|a| &text.artist == a || &track_text.artist == a)
                        && genre.as_ref().is_none_or(|g| &track_text.genre == g)
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
