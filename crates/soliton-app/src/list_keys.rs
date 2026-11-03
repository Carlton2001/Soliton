//! Arrow keys move the keyboard focus through the lists of the current page.
use crate::{
    app::{Section, Soliton},
    list_focus::{moved, next_with_letter, page_rows},
    track_list::Selection,
};
use gpui::*;
use soliton_core::config::AlbumSort;

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

    /// Starts a frame; once drawn, a focused item scrolled out of view hands
    /// the focus back to the window, and the arrows resume from it.
    pub(super) fn track_list_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.album_focus.begin_frame();
        self.artist_focus.begin_frame();
        self.track_focus.begin_frame();
        self.playlist_focus.begin_frame();
        cx.on_next_frame(window, |this, window, _| {
            if this.album_focus.focus_hidden(window)
                || this.artist_focus.focus_hidden(window)
                || this.track_focus.focus_hidden(window)
                || this.playlist_focus.focus_hidden(window)
            {
                this.focus.focus(window);
            }
        });
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

    pub(super) fn list_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(row) = self.track_focus.focused(window)
            && self.reorder_key(row, keystroke, window, cx)
        {
            return;
        }
        let key = keystroke.key.as_str();
        let modifiers = keystroke.modifiers;
        let focused = self.focused_list(window);
        // Typing a letter in a focused list jumps to the next name starting with it.
        let letter = key
            .chars()
            .next()
            .filter(|c| key.len() == 1 && c.is_ascii_alphanumeric())
            .filter(|_| focused.is_some() && !modifiers.modified());
        let page_key = matches!(key, "home" | "end" | "pageup" | "pagedown");
        if letter.is_none() && !page_key && !matches!(key, "left" | "right" | "up" | "down") {
            return;
        }
        let Some(list) = focused.or_else(|| self.page_list(window)) else {
            return;
        };
        let focused = focused.is_some();
        let pick = |index: Option<usize>,
                    names: Option<&[&str]>,
                    len: usize,
                    columns: usize,
                    scroll: &UniformListScrollHandle| match letter {
            Some(letter) => {
                next_with_letter(names?.iter().copied(), index, letter.to_ascii_lowercase())
            }
            None => {
                let page = page_rows(scroll, len.div_ceil(columns));
                target(focused, index, len, columns, page, key)
            }
        };
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
                let albums: Vec<_> = self
                    .visible
                    .iter()
                    .map(|&album| &self.snapshot.albums[album])
                    .collect();
                let index = current.and_then(|id| albums.iter().position(|a| a.id == id));
                // Letters follow the sorted name.
                let names: Option<Vec<&str>> = match self.settings.album_sort {
                    AlbumSort::Artist => Some(albums.iter().map(|a| a.artist.as_str()).collect()),
                    AlbumSort::Title => Some(albums.iter().map(|a| a.title.as_str()).collect()),
                    _ => None,
                };
                let Some(target) = pick(
                    index,
                    names.as_deref(),
                    albums.len(),
                    columns,
                    &self.album_scroll,
                ) else {
                    return;
                };
                let id = albums[target].id;
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
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let Some(target) =
                    pick(index, Some(&refs), names.len(), columns, &self.facet_scroll)
                else {
                    return;
                };
                self.artist_focus.focus(names[target].clone(), window, cx);
                self.facet_scroll
                    .scroll_to_item(target / columns, ScrollStrategy::Center);
            }
            // Rows only move vertically.
            List::Tracks | List::Playlists if matches!(key, "left" | "right") => return,
            List::Tracks => {
                let Some((rows, scroll)) = self.track_rows() else {
                    return;
                };
                let current = self.track_focus.focused(window).or(self.track_focus.last);
                let index = current.and_then(|row| rows.iter().position(|r| *r == row));
                let Some(target) = pick(index, None, rows.len(), 1, &scroll) else {
                    return;
                };
                self.track_focus.focus(rows[target], window, cx);
                scroll.scroll_to_item(target, ScrollStrategy::Center);
            }
            List::Playlists => {
                let ids: Vec<_> = self.playlists.lists.iter().map(|p| p.id).collect();
                let names: Vec<&str> = self
                    .playlists
                    .lists
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect();
                let current = self
                    .playlist_focus
                    .focused(window)
                    .or(self.playlist_focus.last);
                let index = current.and_then(|id| ids.iter().position(|i| *i == id));
                let Some(target) = pick(index, Some(&names), ids.len(), 1, &self.playlists.scroll)
                else {
                    return;
                };
                self.playlist_focus.focus(ids[target], window, cx);
                self.playlists
                    .scroll
                    .scroll_to_item(target, ScrollStrategy::Center);
            }
        }
        cx.notify();
    }
}

/// Item a navigation key focuses: the next one from a focused item, otherwise
/// the item where the list was left, or its first item.
fn target(
    focused: bool,
    index: Option<usize>,
    len: usize,
    columns: usize,
    page: usize,
    key: &str,
) -> Option<usize> {
    match index {
        Some(index) if focused => moved(index, len, columns, page, key),
        Some(index) if index < len => Some(index),
        _ => (len > 0).then_some(0),
    }
}
