//! Transactional manual playlists. No writes to the source collection.
use crate::{Library, database::path_text};
use anyhow::Result;
use rusqlite::{Connection, params};
use soliton_core::{
    failure::{Failure, Kind},
    model::Track,
    playlist::{MAX_NAME_CHARS, Playlist, PlaylistChange, PlaylistEntry, same_name},
};

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
                require_free_name(&tx, &name, None)?;
                tx.execute(
                    "INSERT INTO playlists(name) VALUES(?1)",
                    [valid_name(&name)?],
                )?;
                append(&tx, tx.last_insert_rowid(), &tracks)?;
            }
            PlaylistChange::Rename { id, name } => {
                require_list(&tx, id)?;
                require_free_name(&tx, &name, Some(id))?;
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
                move_entries(&tx, id, &[entry], before)?;
            }
            PlaylistChange::MoveBlock {
                id,
                entries,
                before,
            } => move_entries(&tx, id, &entries, before)?,
        }
        tx.commit()?;
        Ok(())
    }
}

fn move_entries(tx: &Connection, id: i64, moved: &[i64], before: Option<i64>) -> Result<()> {
    require_list(tx, id)?;
    let mut query =
        tx.prepare("SELECT id FROM playlist_entries WHERE playlist_id=?1 ORDER BY position,id")?;
    let mut order = query
        .query_map([id], |row| row.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if before.is_some_and(|e| moved.contains(&e)) {
        return Ok(());
    }
    if moved.iter().chain(&before).any(|e| !order.contains(e)) {
        return Err(Failure::new(Kind::PlaylistGone).into());
    }
    order.retain(|e| !moved.contains(e));
    let at = before
        .and_then(|e| order.iter().position(|&v| v == e))
        .unwrap_or(order.len());
    order.splice(at..at, moved.iter().copied());
    for (position, entry) in order.into_iter().enumerate() {
        tx.execute(
            "UPDATE playlist_entries SET position=?1 WHERE id=?2 AND playlist_id=?3",
            params![position as i64, entry, id],
        )?;
    }
    Ok(())
}

fn valid_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(Failure::new(Kind::PlaylistName).into());
    }
    Ok(name)
}

/// Compared here rather than by an index: SQLite only folds the case of ASCII.
fn require_free_name(tx: &Connection, name: &str, except: Option<i64>) -> Result<()> {
    let mut query = tx.prepare("SELECT id,name FROM playlists")?;
    let rows = query.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (id, taken) = row?;
        if Some(id) != except && same_name(&taken, name) {
            return Err(Failure::new(Kind::PlaylistNameTaken).into());
        }
    }
    Ok(())
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
