//! Versioned migrations, applied in order inside one transaction each.
use crate::{
    catalog,
    database::path_text,
    grouping::{edition_folder, edition_identity},
};
use anyhow::{Result, bail};
use rusqlite::{Connection, TransactionBehavior, params};
use soliton_core::model::Metadata;
use std::{collections::BTreeSet, path::Path};

pub(crate) const VERSION: i64 = 8;

pub(crate) fn version(connection: &Connection) -> Result<i64> {
    Ok(connection.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

/// Requires foreign key enforcement to be off: v2 rebuilds a referenced table.
pub(crate) fn migrate(connection: &mut Connection) -> Result<()> {
    let version = version(connection)?;
    if version > VERSION {
        bail!("Library created by a newer Soliton version");
    }
    if version < 1 {
        connection.execute_batch(include_str!("schema_v1.sql"))?;
    }
    if version < 2 {
        editions_per_album(connection)?;
    }
    if version < 3 {
        refold_identities(connection)?;
    }
    if version < 4 {
        // Arrival dates for sorting by recent additions; modification times are
        // the best estimate available for files indexed earlier.
        connection.execute_batch(
            "BEGIN IMMEDIATE;
             ALTER TABLE files ADD COLUMN added INTEGER NOT NULL DEFAULT 0;
             UPDATE files SET added=CAST(modified AS INTEGER)/1000000000;
             PRAGMA user_version=4;
             COMMIT;",
        )?;
    }
    if version < 5 {
        connection.execute_batch(
            "BEGIN IMMEDIATE;
             CREATE TABLE playlists(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL);
             CREATE TABLE playlist_entries(
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               playlist_id INTEGER NOT NULL REFERENCES playlists ON DELETE CASCADE,
               track_id INTEGER REFERENCES tracks ON DELETE SET NULL,
               position INTEGER NOT NULL,
               metadata TEXT NOT NULL);
             CREATE INDEX playlist_order ON playlist_entries(playlist_id,position,id);
             PRAGMA user_version=5;
             COMMIT;",
        )?;
    }
    if version < 6 {
        connection.execute_batch(
            "BEGIN IMMEDIATE;
             ALTER TABLE playlist_entries ADD COLUMN path TEXT;
             UPDATE playlist_entries SET path=(SELECT path FROM files WHERE track_id=playlist_entries.track_id);
             DELETE FROM playlist_entries AS later WHERE path IS NOT NULL AND EXISTS(
               SELECT 1 FROM playlist_entries AS earlier
               WHERE earlier.playlist_id=later.playlist_id AND earlier.path=later.path
               AND (earlier.position<later.position OR (earlier.position=later.position AND earlier.id<later.id)));
             CREATE UNIQUE INDEX playlist_file ON playlist_entries(playlist_id,path) WHERE path IS NOT NULL;
             PRAGMA user_version=6;
             COMMIT;",
        )?;
    }
    if version < 7 {
        reconcile_album_artists(connection)?;
    }
    if version < 8 {
        source_volumes(connection)?;
    }
    let violations: i64 =
        connection.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
    if violations > 0 {
        bail!("Library database has {violations} inconsistent references");
    }
    Ok(())
}

/// v8: volume of each source root when it last held music, to tell an emptied
/// folder from an unmounted mount point. A library rewound to an older version
/// may already have the column.
fn source_volumes(connection: &mut Connection) -> Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let present: bool = transaction.query_row(
        "SELECT count(*) FROM pragma_table_info('sources') WHERE name='volume'",
        [],
        |row| row.get(0),
    )?;
    if !present {
        transaction.execute_batch("ALTER TABLE sources ADD COLUMN volume INTEGER;")?;
    }
    transaction.pragma_update(None, "user_version", 8)?;
    transaction.commit()?;
    Ok(())
}

/// v7: repair editions split by partially populated album-artist tags. All
/// information comes from SQLite, including for currently inaccessible sources.
fn reconcile_album_artists(connection: &mut Connection) -> Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE INDEX IF NOT EXISTS editions_source_folder ON editions(source_id,folder);",
    )?;
    let editions = transaction
        .prepare("SELECT id FROM editions")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<BTreeSet<i64>>>()?;
    catalog::regroup(&transaction, &editions)?;
    transaction.pragma_update(None, "user_version", 7)?;
    transaction.commit()?;
    Ok(())
}

/// v2: several editions per album, extractor versions and remembered failures.
/// Existing tracks are regrouped from their stored metadata, without file access.
fn editions_per_album(connection: &mut Connection) -> Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "ALTER TABLE files ADD COLUMN extractor INTEGER NOT NULL DEFAULT 1;
         ALTER TABLE albums ADD COLUMN compilation INTEGER NOT NULL DEFAULT 0;
         CREATE TABLE failures(source_id INTEGER NOT NULL REFERENCES sources, path TEXT NOT NULL,
            size INTEGER NOT NULL, modified TEXT NOT NULL, extractor INTEGER NOT NULL,
            seen INTEGER NOT NULL, PRIMARY KEY(source_id,path));
         CREATE TABLE editions_v2(id INTEGER PRIMARY KEY, identity TEXT NOT NULL UNIQUE,
            source_id INTEGER NOT NULL REFERENCES sources, folder TEXT NOT NULL,
            album_id INTEGER REFERENCES albums);
         DELETE FROM tracks WHERE id NOT IN (SELECT track_id FROM files);",
    )?;
    let editions = assign_editions(&transaction, "editions_v2")?;
    // Documented SQLite procedure for changing constraints of a referenced table.
    transaction.execute_batch(
        "DROP TABLE editions;
         ALTER TABLE editions_v2 RENAME TO editions;
         CREATE INDEX editions_album ON editions(album_id);
         DELETE FROM albums;
         DELETE FROM artists;",
    )?;
    catalog::regroup(&transaction, &editions)?;
    transaction.pragma_update(None, "user_version", 2)?;
    transaction.commit()?;
    Ok(())
}

/// v3: identities ignore accents and ligatures. Editions and albums are re-keyed
/// from stored metadata; tracks keep their identifiers.
fn refold_identities(connection: &mut Connection) -> Result<()> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let editions = assign_editions(&transaction, "editions")?;
    transaction
        .execute_batch("DELETE FROM editions WHERE id NOT IN (SELECT edition_id FROM tracks);")?;
    catalog::regroup(&transaction, &editions)?;
    transaction.pragma_update(None, "user_version", 3)?;
    transaction.commit()?;
    Ok(())
}

/// Moves every track to the edition of its current identity in `table`, creating
/// editions as needed, and returns the editions now holding tracks.
fn assign_editions(transaction: &Connection, table: &str) -> Result<BTreeSet<i64>> {
    let rows = {
        let mut query = transaction.prepare(
            "SELECT t.id,t.metadata,f.path,f.source_id,s.path FROM tracks t
             JOIN files f ON f.track_id=t.id JOIN sources s ON s.id=f.source_id",
        )?;
        query
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut editions = BTreeSet::new();
    for (track, metadata, path, source, root) in rows {
        let metadata: Metadata = serde_json::from_str(&metadata)?;
        let folder = path_text(edition_folder(Path::new(&path), Path::new(&root)))?;
        let identity = edition_identity(source, folder, &metadata);
        transaction.execute(
            &format!(
                "INSERT INTO {table}(identity,source_id,folder) VALUES(?1,?2,?3)
                 ON CONFLICT(identity) DO NOTHING"
            ),
            params![identity, source, folder],
        )?;
        let edition: i64 = transaction.query_row(
            &format!("SELECT id FROM {table} WHERE identity=?1"),
            [&identity],
            |row| row.get(0),
        )?;
        transaction.execute(
            "UPDATE tracks SET edition_id=?1 WHERE id=?2",
            params![edition, track],
        )?;
        editions.insert(edition);
    }
    Ok(editions)
}
