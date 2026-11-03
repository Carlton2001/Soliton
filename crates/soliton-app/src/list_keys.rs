//! Arrow keys move the keyboard focus through the lists of the current page.
use crate::{
    app::{Section, Soliton},
    list_focus::moved,
    track_list::Selection,
};
use gpui::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum List {
    Albums,
    Artists,
    Tracks,
    Playlists,
}

impl Soliton {
    /// List whose item holds the keyboard focus.
    pub(super) fn focused_list(&self, window: &Window) -> Option<List> {
        if self.album_focus.focused(window).is_some() {
            Some(List::Albums)
        } else if self.artist_focus.focused(window).is_some() {
            Some(List::Artists)
        } else if self.track_focus.focused(window).is_some() {
            Some(List::Tracks)
        } else {
            self.playlist_focus.focused(window).map(|_| List::Playlists)
        }
    }

    /// List taking the arrows when no list item is focused.
    fn page_list(&self, window: &Window) -> Option<List> {
        if self.now_playing || self.menu.is_some() {
            return None;
        }
        match self.section {
            Section::Albums
                if self.selected.is_none()
                    || crate::collection::split_collection(
                        crate::navigation::content_width(window),
                        self.section,
                        true,
                    ) =>
            {
                Some(List::Albums)
            }
            Section::Artists if self.selected.is_none() => Some(List::Artists),
            Section::Playlists if self.playlists.selected.is_none() => Some(List::Playlists),
            Section::Albums | Section::Artists | Section::Playlists | Section::Queue => {
                Some(List::Tracks)
            }
            _ => None,
        }
    }

    /// Rows of the track list shown on the page, in display order.
    fn track_rows(&self) -> Option<(Vec<Selection>, UniformListScrollHandle)> {
        match self.section {
            Section::Queue => Some((
                self.playback
                    .queue
                    .entries
                    .iter()
                    .map(|e| Selection::Queue(e.id))
                    .collect(),
                self.queue_scroll.clone(),
            )),
            Section::Playlists => {
                let list = self.playlists.selected?;
                let playlist = self.playlists.lists.iter().find(|p| p.id == list)?;
                Some((
                    playlist
                        .entries
                        .iter()
                        .map(|e| Selection::Playlist { list, entry: e.id })
                        .collect(),
                    self.playlists.scroll.clone(),
                ))
            }
            Section::Albums | Section::Artists => {
                let (album, edition) = self.selected_album()?;
                Some((
                    album.editions[edition]
                        .tracks
                        .iter()
                        .map(|t| Selection::Album {
                            album: album.id,
                            edition,
                            track: t.id,
                        })
                        .collect(),
                    self.track_scroll.clone(),
                ))
            }
            _ => None,
        }
    }

    pub(super) fn list_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(key, "left" | "right" | "up" | "down") {
            return;
        }
        let focused = self.focused_list(window);
        let Some(list) = focused.or_else(|| self.page_list(window)) else {
            return;
        };
        let focused = focused.is_some();
        match list {
            List::Albums => {
                let columns = if self.settings.album_grid {
                    self.columns.max(1)
                } else {
                    1
                };
                let current = self
                    .album_focus
                    .focused(window)
                    .or(self.album_focus.last)
                    .or(self.selected);
                let index = current.and_then(|id| {
                    self.visible
                        .iter()
                        .position(|&album| self.snapshot.albums[album].id == id)
                });
                let Some(target) = target(focused, index, self.visible.len(), columns, key) else {
                    return;
                };
                let id = self.snapshot.albums[self.visible[target]].id;
                self.album_focus.focus(id, window, cx);
                self.album_scroll
                    .scroll_to_item(target / columns, ScrollStrategy::Center);
            }
            List::Artists => {
                let query = soliton_core::text::fold(&self.search.read(cx).content);
                let names: Vec<_> = crate::catalog::visible_artist_cards(
                    &self.facets.artist_cards,
                    &query,
                    self.settings.hide_unavailable,
                )
                .map(|card| card.name.clone())
                .collect();
                let columns = if self.settings.artist_grid {
                    self.columns.max(1)
                } else {
                    1
                };
                let current = self
                    .artist_focus
                    .focused(window)
                    .or_else(|| self.artist_focus.last.clone());
                let index = current.and_then(|name| names.iter().position(|n| *n == name));
                let Some(target) = target(focused, index, names.len(), columns, key) else {
                    return;
                };
                self.artist_focus.focus(names[target].clone(), window, cx);
                self.facet_scroll
                    .scroll_to_item(target / columns, ScrollStrategy::Center);
            }
            // Rows only move vertically.
            List::Tracks if matches!(key, "up" | "down") => {
                let Some((rows, scroll)) = self.track_rows() else {
                    return;
                };
                let current = self.track_focus.focused(window).or(self.track_focus.last);
                let index = current.and_then(|row| rows.iter().position(|r| *r == row));
                let Some(target) = target(focused, index, rows.len(), 1, key) else {
                    return;
                };
                self.track_focus.focus(rows[target], window, cx);
                scroll.scroll_to_item(target, ScrollStrategy::Center);
            }
            List::Playlists if matches!(key, "up" | "down") => {
                let ids: Vec<_> = self.playlists.lists.iter().map(|p| p.id).collect();
                let current = self
                    .playlist_focus
                    .focused(window)
                    .or(self.playlist_focus.last);
                let index = current.and_then(|id| ids.iter().position(|i| *i == id));
                let Some(target) = target(focused, index, ids.len(), 1, key) else {
                    return;
                };
                self.playlist_focus.focus(ids[target], window, cx);
                self.playlists
                    .scroll
                    .scroll_to_item(target, ScrollStrategy::Center);
            }
            List::Tracks | List::Playlists => return,
        }
        cx.notify();
    }
}

/// Item an arrow key focuses: the next one from a focused item, otherwise the
/// item where the list was left, or its first item.
fn target(
    focused: bool,
    index: Option<usize>,
    len: usize,
    columns: usize,
    key: &str,
) -> Option<usize> {
    match index {
        Some(index) if focused => moved(index, len, columns, key),
        Some(index) if index < len => Some(index),
        _ => (len > 0).then_some(0),
    }
}
