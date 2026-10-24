//! Transactional manual playlists. No writes to the source collection.
use crate::{Library, database::path_text};
use anyhow::Result;
use rap_core::{
    failure::{Failure, Kind},
    model::Track,
    playlist::{Playlist, PlaylistChange, PlaylistEntry},
};
use rusqlite::{Connection, params};

impl Library {
    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let mut query = self
            .connection
            .prepare("SELECT id,name FROM playlists ORDER BY id")?;
        let mut lists = query
            .query_map([], |row| {
                Ok(Playlist {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entries: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut entries = self.connection.prepare(
            "SELECT id,track_id,metadata FROM playlist_entries WHERE playlist_id=?1 ORDER BY position,id")?;
        for list in &mut lists {
            let rows = entries.query_map([list.id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get::<_, String>(2)?))
            })?;
            for row in rows {
                let (id, track_id, metadata) = row?;
                list.entries.push(PlaylistEntry {
                    id,
                    track_id,
                    metadata: serde_json::from_str(&metadata)?,
                });
            }
        }
        Ok(lists)
    }

    pub fn change_playlist(&mut self, change: PlaylistChange) -> Result<()> {
        let tx = self.connection.transaction()?;
        match change {
            PlaylistChange::Create { name, tracks } => {
                tx.execute(
                    "INSERT INTO playlists(name) VALUES(?1)",
                    [valid_name(&name)?],
                )?;
                append(&tx, tx.last_insert_rowid(), &tracks)?;
            }
            PlaylistChange::Rename { id, name } => {
                require_list(&tx, id)?;
                tx.execute(
                    "UPDATE playlists SET name=?1 WHERE id=?2",
                    params![valid_name(&name)?, id],
                )?;
            }
            PlaylistChange::Delete(id) => {
                require_list(&tx, id)?;
                tx.execute("DELETE FROM playlists WHERE id=?1", [id])?;
            }
            PlaylistChange::Append { id, tracks } => {
                require_list(&tx, id)?;
                append(&tx, id, &tracks)?;
            }
            PlaylistChange::Remove { id, entry } => {
                require_list(&tx, id)?;
                tx.execute(
                    "DELETE FROM playlist_entries WHERE playlist_id=?1 AND id=?2",
                    params![id, entry],
                )?;
            }
            PlaylistChange::Move { id, entry, before } => {
                require_list(&tx, id)?;
                let mut query = tx.prepare(
                    "SELECT id FROM playlist_entries WHERE playlist_id=?1 ORDER BY position,id",
                )?;
                let mut order = query
                    .query_map([id], |row| row.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                if Some(entry) != before {
                    let Some(from) = order.iter().position(|&e| e == entry) else {
                        return Err(Failure::new(Kind::PlaylistGone).into());
                    };
                    if before.is_some_and(|e| !order.contains(&e)) {
                        return Err(Failure::new(Kind::PlaylistGone).into());
                    }
                    order.remove(from);
                    let at = before
                        .and_then(|e| order.iter().position(|&v| v == e))
                        .unwrap_or(order.len());
                    order.insert(at, entry);
                    for (position, entry) in order.into_iter().enumerate() {
                        tx.execute("UPDATE playlist_entries SET position=?1 WHERE id=?2 AND playlist_id=?3", params![position as i64, entry, id])?;
                    }
                }
            }
        }
        tx.commit()?;
        Ok(())
    }
}

fn valid_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(Failure::new(Kind::PlaylistName).into());
    }
    Ok(name)
}

fn require_list(tx: &Connection, id: i64) -> Result<()> {
    if !tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM playlists WHERE id=?1)",
        [id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Failure::new(Kind::PlaylistGone).into());
    }
    Ok(())
}

fn append(tx: &Connection, id: i64, tracks: &[Track]) -> Result<()> {
    let start: i64 = tx.query_row(
        "SELECT COALESCE(MAX(position)+1,0) FROM playlist_entries WHERE playlist_id=?1",
        [id],
        |r| r.get(0),
    )?;
    for (index, track) in tracks.iter().enumerate() {
        // A catalogue removal may have been processed since the UI snapshot.
        tx.execute(
            "INSERT INTO playlist_entries(playlist_id,track_id,position,metadata,path)
            VALUES(?1,(SELECT id FROM tracks WHERE id=?2),?3,?4,?5)
            ON CONFLICT(playlist_id,path) WHERE path IS NOT NULL DO NOTHING",
            params![
                id,
                track.id,
                start + index as i64,
                serde_json::to_string(&track.metadata)?,
                path_text(&track.path)?
            ],
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/playlists.rs"]
mod tests;
