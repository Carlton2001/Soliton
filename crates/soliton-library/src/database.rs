use crate::schema;
use anyhow::{Result, bail};
use rusqlite::{Connection, params};
use soliton_core::{
    failure::{Failure, Kind},
    model::{Album, Edition, LibrarySnapshot, Metadata, Source, Track},
    text::fold,
};
use std::{
    cmp::Reverse,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct Library {
    pub(crate) connection: Connection,
    pub(crate) storage: PathBuf,
    pub(crate) covers: PathBuf,
    /// Checks a source before touching it from the writer thread.
    pub(crate) prober: crate::Prober,
}

impl Library {
    /// Opens the writer connection, applying migrations first.
    pub fn open(storage: &Path) -> Result<Self> {
        let mut library = Self::connect(storage)?;
        // Bundled SQLite enables enforcement by default; v2 rebuilds a referenced table.
        library
            .connection
            .execute_batch("PRAGMA foreign_keys=OFF;")?;
        schema::migrate(&mut library.connection)?;
        library
            .connection
            .execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(library)
    }

    /// Opens a connection for snapshots only. The writer must have migrated first.
    pub fn open_reader(storage: &Path) -> Result<Self> {
        let library = Self::connect(storage)?;
        if schema::version(&library.connection)? != schema::VERSION {
            bail!("Library database is not at the expected version");
        }
        Ok(library)
    }

    fn connect(storage: &Path) -> Result<Self> {
        std::fs::create_dir_all(storage.join("covers"))?;
        let storage = storage.canonicalize()?;
        let connection = Connection::open(storage.join("library.sqlite3"))?;
        connection.busy_timeout(std::time::Duration::from_secs(3))?;
        connection.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(Self {
            covers: storage.join("covers"),
            storage,
            connection,
            prober: crate::Prober::default(),
        })
    }

    /// Shared with the caller's probes, so a hung share keeps one waiting thread.
    pub fn prober(&self) -> crate::Prober {
        self.prober.clone()
    }

    /// Changes whenever another connection commits; cheap enough to poll per query.
    pub fn data_version(&self) -> Result<i64> {
        Ok(self
            .connection
            .query_row("PRAGMA data_version", [], |row| row.get(0))?)
    }

    pub fn add_source(&mut self, path: &Path) -> Result<i64> {
        let shown = path.display().to_string();
        let path = path.canonicalize().map_err(|error| {
            Failure::with(Kind::SourceInaccessible, error.to_string()).about(&shown)
        })?;
        if !path.is_dir() {
            return Err(Failure::new(Kind::SourceNotDirectory).about(shown).into());
        }
        // A source must not contain application storage, including via a junction.
        if self.storage.starts_with(&path) || path.starts_with(&self.storage) {
            return Err(Failure::new(Kind::SourceOverlapsStorage)
                .about(shown)
                .into());
        }
        for source in self.sources()? {
            if path == source.path {
                return Ok(source.id);
            }
            if path.starts_with(&source.path) || source.path.starts_with(&path) {
                let existing = source.path.display();
                return Err(
                    Failure::with(Kind::SourceOverlapsSource, existing.to_string())
                        .about(shown)
                        .into(),
                );
            }
        }
        self.connection.execute(
            "INSERT INTO sources(path,available) VALUES(?1,1)",
            [path_text(&path)?],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    /// Forgets a source and the catalogue entries only it provided. Files on disk,
    /// their tags and the shared cover cache are left untouched.
    pub fn remove_source(&mut self, id: i64) -> Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute_batch(&format!(
            "CREATE TEMP TABLE removed_tracks AS SELECT track_id AS id FROM files WHERE source_id={id};
             DELETE FROM files WHERE source_id={id};
             DELETE FROM failures WHERE source_id={id};
             DELETE FROM tracks WHERE id IN (SELECT id FROM removed_tracks);
             DROP TABLE removed_tracks;
             DELETE FROM editions WHERE id NOT IN (SELECT edition_id FROM tracks);
             DELETE FROM albums WHERE id NOT IN (SELECT album_id FROM editions WHERE album_id IS NOT NULL);
             DELETE FROM artists WHERE id NOT IN (SELECT artist_id FROM albums);
             DELETE FROM sources WHERE id={id};"
        ))?;
        transaction.commit()?;
        Ok(())
    }

    pub fn sources(&self) -> Result<Vec<Source>> {
        let mut query = self
            .connection
            .prepare_cached("SELECT id,path,available FROM sources ORDER BY id")?;
        Ok(query
            .query_map([], |row| {
                Ok(Source {
                    id: row.get(0)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    available: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
    }

    /// Records the outcome of an availability probe. Entries are never removed here.
    /// Returns whether the state changed.
    pub fn set_source_available(&mut self, id: i64, available: bool) -> Result<bool> {
        Ok(self.connection.execute(
            "UPDATE sources SET available=?1 WHERE id=?2 AND available<>?1",
            params![available, id],
        )? > 0)
    }

    pub fn snapshot(&self) -> Result<LibrarySnapshot> {
        struct Group {
            album: Album,
            editions: BTreeMap<i64, Edition>,
        }
        let mut albums = BTreeMap::<i64, Group>::new();
        let mut query = self.connection.prepare_cached(
            "SELECT t.id,f.source_id,f.path,t.metadata,t.cover,(f.available AND s.available),
                    e.id,e.folder,a.id,a.title,r.name,a.compilation,f.added
             FROM tracks t JOIN files f ON f.track_id=t.id JOIN sources s ON s.id=f.source_id
             JOIN editions e ON e.id=t.edition_id JOIN albums a ON a.id=e.album_id
             JOIN artists r ON r.id=a.artist_id",
        )?;
        let mut rows = query.query([])?;
        while let Some(row) = rows.next()? {
            let metadata: Metadata = serde_json::from_str(&row.get::<_, String>(3)?)?;
            let source_id = row.get(1)?;
            let edition_id = row.get(6)?;
            let album_id = row.get(8)?;
            let group = match albums.entry(album_id) {
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::btree_map::Entry::Vacant(entry) => entry.insert(Group {
                    album: Album {
                        id: album_id,
                        title: row.get(9)?,
                        artist: row.get(10)?,
                        compilation: row.get(11)?,
                        added: 0,
                        editions: Vec::new(),
                    },
                    editions: BTreeMap::new(),
                }),
            };
            group.album.added = group.album.added.max(row.get(12)?);
            let edition = match group.editions.entry(edition_id) {
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::btree_map::Entry::Vacant(entry) => entry.insert(Edition {
                    id: edition_id,
                    source_id,
                    folder: PathBuf::from(row.get::<_, String>(7)?),
                    tracks: Vec::new(),
                }),
            };
            edition.tracks.push(Track {
                id: row.get(0)?,
                source_id,
                path: PathBuf::from(row.get::<_, String>(2)?),
                metadata,
                cover: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
                available: row.get(5)?,
            });
        }
        let mut albums: Vec<_> = albums
            .into_values()
            .map(|group| {
                let mut album = group.album;
                album.editions = group.editions.into_values().collect();
                for edition in &mut album.editions {
                    edition.tracks.sort_by_cached_key(|t| {
                        (
                            t.metadata.disc.unwrap_or(1),
                            t.metadata.track.unwrap_or(u32::MAX),
                            t.path.clone(),
                        )
                    });
                }
                album.editions.sort_by_key(edition_rank);
                album
            })
            .collect();
        albums.sort_by_cached_key(|a| (fold(&a.artist), fold(&a.title), a.id));
        Ok(LibrarySnapshot {
            sources: self.sources()?,
            albums,
        })
    }
}

/// Available editions first, then the highest resolution; lossy formats carry no
/// bit depth. Ties keep indexing order.
fn edition_rank(edition: &Edition) -> (bool, Reverse<u8>, Reverse<u32>, i64) {
    let best = |value: fn(&Metadata) -> Option<u32>| {
        edition
            .tracks
            .iter()
            .filter_map(|t| value(&t.metadata))
            .max()
            .unwrap_or(0)
    };
    (
        !edition.available(),
        Reverse(best(|m| m.bit_depth.map(u32::from)) as u8),
        Reverse(best(|m| m.sample_rate)),
        edition.id,
    )
}

pub(crate) fn path_text(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| {
        Failure::new(Kind::PathNotUnicode)
            .about(path.display())
            .into()
    })
}

#[cfg(test)]
#[path = "tests/database.rs"]
mod tests;
