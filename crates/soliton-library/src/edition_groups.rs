//! Reconcile partially tagged albums using stored metadata, without file access.
use crate::grouping;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use soliton_core::model::Metadata;
use std::collections::{BTreeMap, BTreeSet};

/// Revisit whole affected folders: a later batch may introduce a conflicting
/// explicit artist, in which case an earlier tentative merge must be undone.
/// Track/file IDs and metadata stay unchanged, preserving playlists and queues.
pub(crate) fn reconcile(
    connection: &Connection,
    affected: &BTreeSet<i64>,
) -> Result<BTreeSet<i64>> {
    let mut folders = BTreeSet::new();
    for id in affected {
        if let Some(folder) = connection
            .prepare_cached("SELECT source_id,folder FROM editions WHERE id=?1")?
            .query_row([id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .optional()?
        {
            folders.insert(folder);
        }
    }
    let mut editions = affected.clone();
    for (source, folder) in folders {
        let rows = connection.prepare_cached(
            "SELECT t.id,t.edition_id,t.metadata FROM tracks t
             JOIN editions e ON e.id=t.edition_id WHERE e.source_id=?1 AND e.folder=?2 ORDER BY t.id",
        )?.query_map(params![source, folder], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut albums = BTreeMap::<String, Vec<(i64, i64, Metadata)>>::new();
        for (track, edition, json) in rows {
            let metadata: Metadata = serde_json::from_str(&json)?;
            albums
                .entry(grouping::fold(&metadata.album))
                .or_default()
                .push((track, edition, metadata));
        }
        for tracks in albums.into_values() {
            let compatible = grouping::compatible_album_artist(tracks.iter().map(|(_, _, m)| m));
            for (track, old, metadata) in tracks {
                let identity = match &compatible {
                    Some(artist) => grouping::edition_identity_with_artist(
                        source,
                        &folder,
                        &metadata.album,
                        artist,
                    ),
                    None => grouping::edition_identity(source, &folder, &metadata),
                };
                connection
                    .prepare_cached(
                        "INSERT INTO editions(identity,source_id,folder) VALUES(?1,?2,?3)
                     ON CONFLICT(identity) DO NOTHING",
                    )?
                    .execute(params![identity, source, folder])?;
                let edition: i64 = connection
                    .prepare_cached("SELECT id FROM editions WHERE identity=?1")?
                    .query_row([identity], |row| row.get(0))?;
                if edition != old {
                    connection
                        .prepare_cached("UPDATE tracks SET edition_id=?1 WHERE id=?2")?
                        .execute(params![edition, track])?;
                }
                editions.extend([old, edition]);
            }
        }
    }
    Ok(editions)
}
