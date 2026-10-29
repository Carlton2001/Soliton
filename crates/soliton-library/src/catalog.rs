//! Catalogue writes owned by the indexing worker: files, tracks, editions, albums.
use crate::{
    Library, covers,
    database::path_text,
    discovery::Candidate,
    grouping::{self, edition_folder, edition_identity},
    metadata::{EXTRACTOR, Extracted},
};
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use soliton_core::model::Metadata;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{MAIN_SEPARATOR, Path, PathBuf},
};

/// What the database already knows about a discovered file.
pub(crate) enum Known {
    /// Indexed with the current rules, cover cache present.
    Unchanged,
    /// Same bytes failed extraction with the current rules; do not retry.
    Failed,
    Changed,
}

/// A source root and the directories of one scan below it.
pub(crate) struct Scope<'a> {
    pub source: i64,
    pub root: &'a Path,
    /// Never empty; `[root]` for a whole-source scan.
    pub dirs: &'a [PathBuf],
}

impl Scope<'_> {
    pub fn whole(&self) -> bool {
        self.dirs.iter().any(|dir| dir == self.root)
    }
    /// `None` covers the whole source, otherwise one `prefix\` per directory.
    fn prefixes(&self) -> Result<Vec<Option<String>>> {
        if self.whole() {
            return Ok(vec![None]);
        }
        self.dirs
            .iter()
            .map(|dir| Ok(Some(format!("{}{MAIN_SEPARATOR}", path_text(dir)?))))
            .collect()
    }
}

impl Library {
    pub(crate) fn known(
        &self,
        source: i64,
        path: &Path,
        size: u64,
        modified: &str,
        covers_present: &mut HashMap<PathBuf, bool>,
    ) -> Result<Known> {
        let path = path_text(path)?;
        let indexed = self
            .connection
            .prepare_cached(
                "SELECT f.size,f.modified,f.extractor,t.cover FROM files f
                 JOIN tracks t ON t.id=f.track_id WHERE f.source_id=?1 AND f.path=?2",
            )?
            .query_row(params![source, path], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .optional()?;
        if let Some((known_size, known_modified, extractor, cover)) = indexed {
            let same = known_size == size && known_modified == modified && extractor == EXTRACTOR;
            // A deleted cache entry is regenerated from the file on the next scan.
            let cover_present = cover.is_none_or(|cover| {
                let cover = PathBuf::from(cover);
                *covers_present
                    .entry(cover)
                    .or_insert_with_key(|cover| covers::complete(cover))
            });
            if same && cover_present {
                return Ok(Known::Unchanged);
            }
        }
        let failed = self
            .connection
            .prepare_cached(
                "SELECT 1 FROM failures WHERE source_id=?1 AND path=?2
                 AND size=?3 AND modified=?4 AND extractor=?5",
            )?
            .exists(params![source, path, size, modified, EXTRACTOR])?;
        Ok(if failed {
            Known::Failed
        } else {
            Known::Changed
        })
    }

    pub(crate) fn seen(&self, source: i64, path: &Path, generation: i64) -> Result<()> {
        let path = path_text(path)?;
        self.connection
            .prepare_cached("UPDATE files SET seen=?1,available=1 WHERE source_id=?2 AND path=?3")?
            .execute(params![generation, source, path])?;
        self.connection
            .prepare_cached("UPDATE failures SET seen=?1 WHERE source_id=?2 AND path=?3")?
            .execute(params![generation, source, path])?;
        Ok(())
    }

    /// Remembers that these bytes cannot be read, so unchanged files are not retried.
    /// A previous entry for the file is kept and stays visible.
    pub(crate) fn record_failure(
        &self,
        source: i64,
        path: &Path,
        size: u64,
        modified: &str,
        generation: i64,
    ) -> Result<()> {
        self.connection
            .prepare_cached(
                "INSERT OR REPLACE INTO failures(source_id,path,size,modified,extractor,seen)
                 VALUES(?1,?2,?3,?4,?5,?6)",
            )?
            .execute(params![
                source,
                path_text(path)?,
                size,
                modified,
                EXTRACTOR,
                generation
            ])?;
        Ok(())
    }

    /// Stores a track and returns the editions whose album must be recomputed.
    pub(crate) fn store(
        &self,
        source: i64,
        root: &Path,
        candidate: &Candidate,
        generation: i64,
        extracted: &Extracted,
    ) -> Result<[Option<i64>; 2]> {
        let (metadata, cover) = (&extracted.metadata, extracted.cover.as_deref());
        let connection = &self.connection;
        let path = candidate.path.as_path();
        let folder = path_text(edition_folder(path, root))?;
        let identity = edition_identity(source, folder, metadata);
        connection
            .prepare_cached(
                "INSERT INTO editions(identity,source_id,folder) VALUES(?1,?2,?3)
                 ON CONFLICT(identity) DO NOTHING",
            )?
            .execute(params![identity, source, folder])?;
        let edition: i64 = connection
            .prepare_cached("SELECT id FROM editions WHERE identity=?1")?
            .query_row([&identity], |r| r.get(0))?;
        let file = path_text(path)?;
        let old: Option<(i64, i64)> = connection
            .prepare_cached(
                "SELECT t.id,t.edition_id FROM files f JOIN tracks t ON t.id=f.track_id
                 WHERE f.source_id=?1 AND f.path=?2",
            )?
            .query_row(params![source, file], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        let cover = cover.map(path_text).transpose()?;
        let json = serde_json::to_string(metadata)?;
        let track = if let Some((id, _)) = old {
            connection
                .prepare_cached("UPDATE tracks SET edition_id=?1,metadata=?2,cover=?3 WHERE id=?4")?
                .execute(params![edition, json, cover, id])?;
            id
        } else {
            connection
                .prepare_cached("INSERT INTO tracks(edition_id,metadata,cover) VALUES(?1,?2,?3)")?
                .execute(params![edition, json, cover])?;
            connection.last_insert_rowid()
        };
        connection
            .prepare_cached(
                "INSERT INTO files(source_id,path,track_id,size,modified,seen,available,extractor,added)
                 VALUES(?1,?2,?3,?4,?5,?6,1,?7,?8)
                 ON CONFLICT(source_id,path) DO UPDATE SET size=excluded.size,
                 modified=excluded.modified,seen=excluded.seen,available=1,
                 extractor=excluded.extractor",
            )?
            .execute(params![
                source,
                file,
                track,
                candidate.size,
                candidate.modified,
                generation,
                EXTRACTOR,
                extracted.created.unwrap_or(candidate.added)
            ])?;
        connection
            .prepare_cached("DELETE FROM failures WHERE source_id=?1 AND path=?2")?
            .execute(params![source, file])?;
        Ok([Some(edition), old.map(|(_, edition)| edition)])
    }

    /// Forgets files of the scope not seen by `generation`, after a complete traversal
    /// of an accessible source. Only database rows are removed; returns their count.
    pub(crate) fn purge(&self, scope: &Scope, generation: i64) -> Result<usize> {
        let mut editions = BTreeSet::new();
        let mut removed = 0;
        for prefix in scope.prefixes()? {
            let filter = "f.source_id=?1 AND f.seen<>?2
                AND (?3 IS NULL OR substr(f.path,1,length(?3))=?3)";
            let missing = {
                let mut query = self.connection.prepare(&format!(
                    "SELECT f.id,t.id,t.edition_id FROM files f JOIN tracks t ON t.id=f.track_id
                     WHERE {filter}"
                ))?;
                query
                    .query_map(params![scope.source, generation, prefix], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get(2)?))
                    })?
                    .collect::<rusqlite::Result<Vec<(i64, i64, i64)>>>()?
            };
            for (file, track, edition) in missing {
                self.connection
                    .prepare_cached("DELETE FROM files WHERE id=?1")?
                    .execute([file])?;
                self.connection
                    .prepare_cached("DELETE FROM tracks WHERE id=?1")?
                    .execute([track])?;
                editions.insert(edition);
                removed += 1;
            }
            self.connection.execute(
                &format!("DELETE FROM failures AS f WHERE {filter}"),
                params![scope.source, generation, prefix],
            )?;
        }
        regroup(&self.connection, &editions)?;
        Ok(removed)
    }

    /// Counts the entries below `dir` as seen by `generation`, so that the purge
    /// keeps them, but flags them unavailable: the folder could not be listed.
    pub(crate) fn keep_unreadable(&self, source: i64, dir: &Path, generation: i64) -> Result<()> {
        let prefix = format!("{}{MAIN_SEPARATOR}", path_text(dir)?);
        self.connection.execute(
            "UPDATE files SET seen=?1,available=0 WHERE source_id=?2 AND seen<>?1
             AND substr(path,1,length(?3))=?3",
            params![generation, source, prefix],
        )?;
        Ok(())
    }

    /// Keeps entries of files not seen by `generation` but flags them unavailable.
    pub(crate) fn mark_unseen_unavailable(&self, scope: &Scope, generation: i64) -> Result<()> {
        for prefix in scope.prefixes()? {
            self.connection.execute(
                "UPDATE files AS f SET available=0 WHERE f.source_id=?1 AND f.seen<>?2
                 AND (?3 IS NULL OR substr(f.path,1,length(?3))=?3)",
                params![scope.source, generation, prefix],
            )?;
        }
        Ok(())
    }

    pub(crate) fn source_volume(&self, source: i64) -> Result<Option<u64>> {
        let volume: Option<i64> = self.connection.query_row(
            "SELECT volume FROM sources WHERE id=?1",
            [source],
            |row| row.get(0),
        )?;
        // Stored as SQLite's signed integer, bit for bit.
        Ok(volume.map(|volume| volume as u64))
    }

    pub(crate) fn set_source_volume(&self, source: i64, volume: Option<u64>) -> Result<()> {
        self.connection.execute(
            "UPDATE sources SET volume=?1 WHERE id=?2",
            params![volume.map(|volume| volume as i64), source],
        )?;
        Ok(())
    }

    pub(crate) fn has_files(&self, source: i64) -> Result<bool> {
        Ok(self
            .connection
            .prepare_cached("SELECT 1 FROM files WHERE source_id=?1")?
            .exists([source])?)
    }

    /// Deletes cache thumbnails no track references. Run only between scans.
    pub fn prune_covers(&self) -> Result<usize> {
        let mut query = self
            .connection
            .prepare("SELECT DISTINCT cover FROM tracks WHERE cover IS NOT NULL")?;
        let keep = query
            .query_map([], |row| row.get::<_, String>(0).map(PathBuf::from))?
            .collect::<rusqlite::Result<HashSet<_>>>()?;
        covers::prune(&self.covers, &keep)
    }
}

/// Recomputes the album of each edition, then forgets empty editions and albums.
pub(crate) fn regroup(connection: &Connection, editions: &BTreeSet<i64>) -> Result<()> {
    let editions = crate::edition_groups::reconcile(connection, editions)?;
    for edition in editions {
        let identity: Option<String> = connection
            .prepare_cached("SELECT identity FROM editions WHERE id=?1")?
            .query_row([edition], |row| row.get(0))
            .optional()?;
        let Some(identity) = identity else { continue };
        let tracks = connection
            .prepare_cached("SELECT metadata FROM tracks WHERE edition_id=?1 ORDER BY id")?
            .query_map([edition], |row| row.get::<_, String>(0))?
            .map(|json| Ok(serde_json::from_str::<Metadata>(&json?)?))
            .collect::<Result<Vec<_>>>()?;
        let Some(album) = grouping::assign(&identity, &tracks) else {
            connection
                .prepare_cached("DELETE FROM editions WHERE id=?1")?
                .execute([edition])?;
            continue;
        };
        connection
            .prepare_cached("INSERT OR IGNORE INTO artists(name) VALUES(?1)")?
            .execute([&album.artist])?;
        connection
            .prepare_cached(
                "INSERT INTO albums(identity,title,artist_id,compilation)
                 SELECT ?1,?2,id,?4 FROM artists WHERE name=?3
                 ON CONFLICT(identity) DO UPDATE SET title=excluded.title,
                 artist_id=excluded.artist_id,compilation=excluded.compilation",
            )?
            .execute(params![
                album.identity,
                album.title,
                album.artist,
                album.compilation
            ])?;
        connection
            .prepare_cached(
                "UPDATE editions SET album_id=(SELECT id FROM albums WHERE identity=?1) WHERE id=?2",
            )?
            .execute(params![album.identity, edition])?;
    }
    connection.execute_batch(
        "DELETE FROM albums WHERE id NOT IN
            (SELECT album_id FROM editions WHERE album_id IS NOT NULL);
         DELETE FROM artists WHERE id NOT IN (SELECT artist_id FROM albums);",
    )?;
    Ok(())
}
