use anyhow::{Context, Result, bail};
use rap_core::model::{Album, LibrarySnapshot, Metadata, Source, Track};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct Library {
    pub(crate) connection: Connection,
    pub(crate) storage: PathBuf,
    pub(crate) covers: PathBuf,
}

impl Library {
    pub fn open(storage: &Path) -> Result<Self> {
        std::fs::create_dir_all(storage.join("covers"))?;
        let storage = storage.canonicalize()?;
        let connection = Connection::open(storage.join("library.sqlite3"))?;
        connection.busy_timeout(std::time::Duration::from_secs(3))?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 1 {
            bail!("Library created by a newer RAP version");
        }
        if version == 0 {
            connection.execute_batch(include_str!("schema.sql"))?;
        }
        Ok(Self {
            covers: storage.join("covers"),
            storage,
            connection,
        })
    }

    pub fn add_source(&mut self, path: &Path) -> Result<i64> {
        let path = path.canonicalize().context("Cannot access source")?;
        if !path.is_dir() {
            bail!("Source is not a directory");
        }
        // A source must not contain application storage, including via a junction.
        if self.storage.starts_with(&path) || path.starts_with(&self.storage) {
            bail!("Source overlaps RAP application storage");
        }
        for source in self.sources()? {
            if path == source.path {
                return Ok(source.id);
            }
            if path.starts_with(&source.path) || source.path.starts_with(&path) {
                bail!("Source overlaps an existing source");
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
             DELETE FROM tracks WHERE id IN (SELECT id FROM removed_tracks);
             DROP TABLE removed_tracks;
             DELETE FROM editions WHERE id NOT IN (SELECT edition_id FROM tracks);
             DELETE FROM albums WHERE id NOT IN (SELECT album_id FROM editions);
             DELETE FROM artists WHERE id NOT IN (SELECT artist_id FROM albums);
             DELETE FROM sources WHERE id={id};"
        ))?;
        transaction.commit()?;
        Ok(())
    }

    pub fn sources(&self) -> Result<Vec<Source>> {
        let mut query = self
            .connection
            .prepare("SELECT id,path,available FROM sources ORDER BY id")?;
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

    pub fn snapshot(&self) -> Result<LibrarySnapshot> {
        let mut albums = BTreeMap::<i64, Album>::new();
        let mut query = self.connection.prepare(
            "SELECT t.id,f.source_id,f.path,t.metadata,t.cover,(f.available AND s.available),a.id,a.title,r.name
             FROM tracks t JOIN files f ON f.track_id=t.id JOIN sources s ON s.id=f.source_id
             JOIN editions e ON e.id=t.edition_id JOIN albums a ON a.id=e.album_id
             JOIN artists r ON r.id=a.artist_id ORDER BY a.id,t.id")?;
        let rows = query.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, bool>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;
        for row in rows {
            let (id, source_id, path, metadata, cover, available, album_id, title, artist) = row?;
            let metadata: Metadata = serde_json::from_str(&metadata)?;
            let album = albums.entry(album_id).or_insert_with(|| Album {
                id: album_id,
                title,
                artist,
                tracks: Vec::new(),
            });
            album.tracks.push(Track {
                id,
                source_id,
                path: path.into(),
                metadata,
                cover: cover.map(PathBuf::from),
                available,
            });
        }
        let mut albums: Vec<_> = albums.into_values().collect();
        for album in &mut albums {
            album.tracks.sort_by_key(|t| {
                (
                    t.metadata.disc.unwrap_or(1),
                    t.metadata.track.unwrap_or(u32::MAX),
                    t.path.clone(),
                )
            });
        }
        albums.sort_by_cached_key(|a| (a.artist.to_lowercase(), a.title.to_lowercase(), a.id));
        Ok(LibrarySnapshot {
            sources: self.sources()?,
            albums,
        })
    }

    pub(crate) fn fingerprint(&self, source: i64, path: &Path) -> Result<Option<(u64, String)>> {
        Ok(self
            .connection
            .query_row(
                "SELECT size,modified FROM files WHERE source_id=?1 AND path=?2",
                params![source, path_text(path)?],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?)
    }

    pub(crate) fn seen(&self, source: i64, path: &Path, generation: i64) -> Result<()> {
        self.connection.execute(
            "UPDATE files SET seen=?1,available=1 WHERE source_id=?2 AND path=?3",
            params![generation, source, path_text(path)?],
        )?;
        Ok(())
    }

    pub(crate) fn store(
        &self,
        source: i64,
        path: &Path,
        size: u64,
        modified: &str,
        generation: i64,
        metadata: &Metadata,
        cover: Option<&Path>,
    ) -> Result<()> {
        let folder = path.parent().context("Missing parent")?;
        // Deliberately conservative: different folders remain different editions/albums.
        let key =
            serde_json::to_string(&(source, folder, &metadata.album_artist, &metadata.album))?;
        self.connection.execute(
            "INSERT OR IGNORE INTO artists(name) VALUES(?1)",
            [&metadata.album_artist],
        )?;
        self.connection.execute("INSERT OR IGNORE INTO albums(identity,title,artist_id) SELECT ?1,?2,id FROM artists WHERE name=?3", params![key, metadata.album, metadata.album_artist])?;
        self.connection.execute("INSERT OR IGNORE INTO editions(album_id,folder) SELECT id,?1 FROM albums WHERE identity=?2", params![path_text(folder)?,key])?;
        let edition: i64 = self.connection.query_row(
            "SELECT e.id FROM editions e JOIN albums a ON a.id=e.album_id WHERE a.identity=?1",
            [&key],
            |r| r.get(0),
        )?;
        let old: Option<i64> = self
            .connection
            .query_row(
                "SELECT track_id FROM files WHERE source_id=?1 AND path=?2",
                params![source, path_text(path)?],
                |r| r.get(0),
            )
            .optional()?;
        let cover = cover.map(path_text).transpose()?;
        let json = serde_json::to_string(metadata)?;
        let track = if let Some(id) = old {
            self.connection.execute(
                "UPDATE tracks SET edition_id=?1,metadata=?2,cover=?3 WHERE id=?4",
                params![edition, json, cover, id],
            )?;
            id
        } else {
            self.connection.execute(
                "INSERT INTO tracks(edition_id,metadata,cover) VALUES(?1,?2,?3)",
                params![edition, json, cover],
            )?;
            self.connection.last_insert_rowid()
        };
        self.connection.execute("INSERT INTO files(source_id,path,track_id,size,modified,seen,available) VALUES(?1,?2,?3,?4,?5,?6,1)
            ON CONFLICT(source_id,path) DO UPDATE SET size=excluded.size,modified=excluded.modified,seen=excluded.seen,available=1", params![source,path_text(path)?,track,size,modified,generation])?;
        Ok(())
    }
}

pub(crate) fn path_text(path: &Path) -> Result<&str> {
    path.to_str()
        .context("Path cannot be represented as Unicode")
}

#[cfg(test)]
#[path = "tests/database.rs"]
mod tests;
