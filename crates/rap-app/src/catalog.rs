use rap_core::model::LibrarySnapshot;

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
            for track in &album.tracks {
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

pub fn matches(snapshot: &LibrarySnapshot, filter: &Filter) -> Vec<usize> {
    let query = filter.query.to_lowercase();
    snapshot
        .albums
        .iter()
        .enumerate()
        .filter_map(|(index, album)| {
            let album_match = query.is_empty()
                || album.title.to_lowercase().contains(&query)
                || album.artist.to_lowercase().contains(&query);
            let any = album.tracks.iter().any(|track| {
                let m = &track.metadata;
                (album_match
                    || m.title.to_lowercase().contains(&query)
                    || m.artist.to_lowercase().contains(&query)
                    || m.genre.to_lowercase().contains(&query))
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

#[cfg(test)]
#[path = "tests/catalog.rs"]
mod tests;
