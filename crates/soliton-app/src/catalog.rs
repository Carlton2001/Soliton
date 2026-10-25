use crate::i18n::{Text, text};
use soliton_core::{
    config::{AlbumSort, Language},
    model::{Album, LibrarySnapshot, Metadata, Track},
    text::fold,
};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

/// One tile of the Artists view.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtistCard {
    pub name: String,
    /// Albums by the artist, or containing at least one of their tracks.
    pub albums: usize,
    /// Cover of the most recent of those albums that has one.
    pub cover: Option<PathBuf>,
}

#[derive(Clone, Default)]
pub struct Facets {
    pub artists: Vec<String>,
    /// Same order as `artists`.
    pub artist_cards: Vec<ArtistCard>,
    pub genres: Vec<String>,
    pub formats: Vec<String>,
}
impl Facets {
    pub fn collect(snapshot: &LibrarySnapshot) -> Self {
        let mut artists = Variants::default();
        let mut genres = Variants::default();
        let mut formats = BTreeSet::new();
        for album in &snapshot.albums {
            artists.add(&album.artist);
            for track in album.tracks() {
                let m = &track.metadata;
                artists.add(&m.artist);
                genres.add(&m.genre);
                formats.insert(m.format.clone());
            }
        }
        let artists = artists.into_values();
        Self {
            artist_cards: artist_cards(snapshot, &artists),
            artists,
            genres: genres.into_values(),
            formats: formats.into_iter().collect(),
        }
    }
}

/// Cover of the artist's most recent album: albums credited to the artist come
/// before those where they only appear on a track; undated albums come last.
fn artist_cards(snapshot: &LibrarySnapshot, artists: &[String]) -> Vec<ArtistCard> {
    /// Credited, dated, newest, then title: the smallest key wins.
    type Rank = (bool, bool, Reverse<u32>, String);
    #[derive(Default)]
    struct Found<'a> {
        albums: usize,
        best: Option<(Rank, &'a PathBuf)>,
    }
    let mut found: HashMap<String, Found> = HashMap::new();
    for album in &snapshot.albums {
        let year = album
            .primary()
            .tracks
            .iter()
            .filter_map(|track| track.metadata.year)
            .max();
        let credited = fold(&album.artist);
        let mut names: BTreeSet<String> = album
            .tracks()
            .map(|track| fold(&track.metadata.artist))
            .collect();
        names.insert(credited.clone());
        names.remove("");
        for name in names {
            let entry = found.entry(name.clone()).or_default();
            entry.albums += 1;
            if let Some(cover) = album.cover() {
                let rank = (
                    name != credited,
                    year.is_none(),
                    Reverse(year.unwrap_or(0)),
                    fold(&album.title),
                );
                if entry.best.as_ref().is_none_or(|(best, _)| rank < *best) {
                    entry.best = Some((rank, cover));
                }
            }
        }
    }
    artists
        .iter()
        .map(|name| {
            let entry = found.get(&fold(name));
            ArtistCard {
                name: name.clone(),
                albums: entry.map_or(0, |entry| entry.albums),
                cover: entry
                    .and_then(|entry| entry.best.as_ref().map(|(_, cover)| (*cover).clone())),
            }
        })
        .collect()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    Decade(u32),
    Year(u32),
}
impl Period {
    fn contains(self, year: u32) -> bool {
        match self {
            Period::Decade(decade) => year / 10 * 10 == decade,
            Period::Year(value) => year == value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quality {
    /// More than 16 bits or more than 48 kHz.
    HiRes,
    /// Lossless up to 16 bits and 48 kHz.
    Cd,
    /// Lossy codecs report no bit depth.
    Lossy,
}
impl Quality {
    pub const ALL: [Quality; 3] = [Quality::HiRes, Quality::Cd, Quality::Lossy];

    pub fn of(metadata: &Metadata) -> Self {
        match metadata.bit_depth {
            None => Quality::Lossy,
            Some(bits) if bits > 16 || metadata.sample_rate.is_some_and(|r| r > 48_000) => {
                Quality::HiRes
            }
            Some(_) => Quality::Cd,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub query: String,
    pub artist: Option<String>,
    pub genre: Option<String>,
    pub period: Option<Period>,
    pub format: Option<String>,
    pub quality: Option<Quality>,
    pub sort: AlbumSort,
    pub hide_unavailable: bool,
}

impl Filter {
    /// Any criterion narrowing the albums, search excluded.
    pub fn active(&self) -> bool {
        self.artist.is_some()
            || self.genre.is_some()
            || self.period.is_some()
            || self.format.is_some()
            || self.quality.is_some()
    }
}

/// Folded artists and genres, years, formats and qualities offered by the
/// filter menu; the facets keep their displayed spellings.
#[derive(Clone, Debug, Default)]
pub struct FilterOptions {
    pub artists: HashSet<String>,
    pub genres: HashSet<String>,
    pub years: BTreeSet<u32>,
    pub formats: HashSet<String>,
    pub qualities: HashSet<Quality>,
}

/// A snapshot with its facets and folded search keys, built once per library
/// change so that each keystroke only compares prepared strings. Case, accent and
/// spacing differences between tags never hide an album.
pub struct Catalog {
    pub tracks: Arc<HashMap<i64, Track>>,
    pub snapshot: Arc<LibrarySnapshot>,
    pub facets: Arc<Facets>,
    albums: Vec<AlbumText>,
}

struct AlbumText {
    /// Title and artist, one per line.
    album: String,
    artist: String,
    title: String,
    year: Option<u32>,
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
        let albums = snapshot.albums.iter().map(album_text).collect();
        Self {
            tracks: Arc::new(
                snapshot
                    .albums
                    .iter()
                    .flat_map(|a| a.tracks())
                    .map(|t| (t.id, t.clone()))
                    .collect(),
            ),
            facets: Arc::new(Facets::collect(&snapshot)),
            snapshot: Arc::new(snapshot),
            albums,
        }
    }

    /// Values of each filter kind still reachable under the other active
    /// criteria, so that combining filters never leads to an empty list.
    pub fn options(&self, filter: &Filter) -> FilterOptions {
        let artist = filter.artist.as_deref().map(fold);
        let genre = filter.genre.as_deref().map(fold);
        let mut options = FilterOptions::default();
        let albums = self
            .snapshot
            .albums
            .iter()
            .zip(&self.albums)
            .filter(|(album, _)| {
                !filter.hide_unavailable || album.editions.iter().any(|e| e.available())
            });
        for (album, text) in albums {
            for (track, track_text) in album.tracks().zip(&text.tracks) {
                let m = &track.metadata;
                let quality = Quality::of(m);
                let ok = [
                    artist
                        .as_ref()
                        .is_none_or(|a| &text.artist == a || &track_text.artist == a),
                    genre.as_ref().is_none_or(|g| &track_text.genre == g),
                    filter
                        .period
                        .is_none_or(|period| m.year.is_some_and(|y| period.contains(y))),
                    filter.format.as_ref().is_none_or(|f| &m.format == f),
                    filter.quality.is_none_or(|q| quality == q),
                ];
                let others = |skip: usize| ok.iter().enumerate().all(|(i, &ok)| i == skip || ok);
                if others(0) {
                    options.artists.insert(text.artist.clone());
                    options.artists.insert(track_text.artist.clone());
                }
                if others(1) {
                    options.genres.insert(track_text.genre.clone());
                }
                if others(2) {
                    options.years.extend(m.year);
                }
                if others(3) {
                    options.formats.insert(m.format.clone());
                }
                if others(4) {
                    options.qualities.insert(quality);
                }
            }
        }
        options
    }

    /// Indices of matching albums, in the requested order.
    pub fn matches(&self, filter: &Filter) -> Vec<usize> {
        let query = fold(&filter.query);
        let artist = filter.artist.as_deref().map(fold);
        let genre = filter.genre.as_deref().map(fold);
        let mut visible: Vec<usize> = self
            .snapshot
            .albums
            .iter()
            .zip(&self.albums)
            .enumerate()
            .filter(|(_, (album, _))| {
                !filter.hide_unavailable || album.editions.iter().any(|e| e.available())
            })
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
                            .period
                            .is_none_or(|period| m.year.is_some_and(|y| period.contains(y)))
                        && filter
                            .format
                            .as_ref()
                            .is_none_or(|format| &m.format == format)
                        && filter
                            .quality
                            .is_none_or(|quality| Quality::of(m) == quality)
                });
                any.then_some(index)
            })
            .collect();
        let albums = &self.albums;
        let added = |index: usize| self.snapshot.albums[index].added;
        // The snapshot is already ordered by artist, then title.
        match filter.sort {
            AlbumSort::Artist => {}
            AlbumSort::Title => visible.sort_by(|&a, &b| albums[a].title.cmp(&albums[b].title)),
            AlbumSort::Year => visible.sort_by_key(|&i| (albums[i].year.is_none(), albums[i].year)),
            AlbumSort::Added => visible.sort_by_key(|&i| Reverse(added(i))),
        }
        visible
    }
}

fn album_text(album: &Album) -> AlbumText {
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
        title: fold(&album.title),
        year: album.primary().tracks.iter().find_map(|t| t.metadata.year),
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
}

#[cfg(test)]
#[path = "tests/catalog.rs"]
mod tests;
