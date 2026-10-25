//! Common presentation and actions for editable queue and playlist rows.
use crate::{
    app::Rap,
    i18n::Text,
    playlists::Dialog,
    widgets::{button, icon_button, quality, time},
};
use gpui::{prelude::*, *};
use rap_audio::Command;
use rap_core::{
    model::{Metadata, Track},
    playlist::PlaylistChange,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Album {
        album: i64,
        edition: usize,
        track: i64,
    },
    Queue(u64),
    Playlist {
        list: i64,
        entry: i64,
    },
}
#[derive(Clone, Copy)]
enum Action {
    Play,
    Up,
    Down,
    Remove,
    Next,
    Append,
    Save,
}

pub struct TrackRow<'a> {
    pub selection: Selection,
    pub index: usize,
    pub cover: Option<&'a std::path::PathBuf>,
    pub metadata: &'a Metadata,
    pub available: bool,
    pub current: bool,
    pub first: bool,
    pub last: bool,
}

/// Source numbering is identical in albums, playlists and the playback queue.
fn track_number(metadata: &Metadata, index: usize) -> String {
    let disc = metadata.disc.filter(|&number| number > 0).unwrap_or(1);
    let track = metadata
        .track
        .filter(|&number| number > 0)
        .map_or_else(|| (index + 1).to_string(), |number| number.to_string());
    format!("{disc}.{track}")
}
impl Rap {
    pub fn selection_row(&self, row: TrackRow<'_>, cx: &mut Context<Self>) -> AnyElement {
        self.album_selection_row(row, false, cx)
    }

    pub(super) fn album_selection_row(
        &self,
        row: TrackRow<'_>,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme();
        let selection = row.selection;
        let key = match selection {
            Selection::Queue(id) => id as usize,
            Selection::Playlist { entry, .. } => entry as usize,
            Selection::Album { track, .. } => track as usize,
        };
        let playing = row.current && self.playback.state == rap_audio::PlaybackState::Playing;
        let mut actions = Vec::new();
        if !matches!(selection, Selection::Queue(_)) {
            actions.push(("skip-forward", Text::PlayNext, Action::Next));
            actions.push(("list-end", Text::AppendQueue, Action::Append));
        }
        if !matches!(selection, Selection::Playlist { .. }) {
            actions.push(("list-plus", Text::AddToPlaylist, Action::Save));
        }
        if !matches!(selection, Selection::Album { .. }) {
            if !row.first {
                actions.push(("arrow-up", Text::MoveUp, Action::Up));
            }
            if !row.last {
                actions.push(("arrow-down", Text::MoveDown, Action::Down));
            }
            if !row.current || matches!(selection, Selection::Playlist { .. }) {
                actions.push(("x", Text::RemoveEntry, Action::Remove));
            }
        }
        let open = self
            .track_menu
            .filter(|(selected, _)| *selected == selection);
        let menu = div()
            .id(("track-action-menu", key))
            .w(rems(15.))
            .p(rems(0.5))
            .flex()
            .flex_col()
            .gap(rems(0.25))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.control_border)
            .rounded(crate::widgets::RADIUS_LARGE)
            .shadow_md()
            .occlude()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.track_menu = None;
                cx.notify();
            }))
            .children(
                actions
                    .into_iter()
                    .enumerate()
                    .map(|(i, (icon, label, action))| {
                        button(("track-action", i), icon, self.t(label), theme)
                            .w_full()
                            .justify_start()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.track_menu = None;
                                this.selection_action(selection, action, window, cx);
                            }))
                    }),
            );
        let inset = if matches!(selection, Selection::Album { .. }) && !compact {
            rems(0.)
        } else {
            crate::widgets::PAGE_INSET
        };
        let line = div()
            .id(("selection-row", key))
            .relative()
            .w_full()
            .min_w_0()
            .h(rems(3.5))
            .px(rems(0.5))
            .flex()
            .items_center()
            .gap(rems(if compact { 0.375 } else { 0.75 }))
            .rounded(crate::widgets::RADIUS)
            .when(row.current, |v| v.bg(theme.selected))
            .hover(move |s| s.bg(theme.hover))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                if event.click_count() == 2 {
                    this.selection_action(selection, Action::Play, window, cx);
                }
            }))
            .child(
                icon_button(
                    ("selection-play", key),
                    if playing { "pause" } else { "play" },
                    self.t(if playing { Text::Pause } else { Text::Play }),
                    false,
                    theme,
                )
                .opacity(if row.available { 1. } else { 0.35 })
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.selection_action(selection, Action::Play, window, cx)
                })),
            )
            .child(
                div()
                    .w(rems(2.75))
                    .flex_none()
                    .text_color(theme.muted)
                    .font(crate::widgets::numeric_font())
                    .child(track_number(row.metadata, row.index)),
            )
            .when(!matches!(selection, Selection::Album { .. }), |line| {
                line.child(crate::widgets::cover(row.cover, 32., theme))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .whitespace_normal()
                            .line_clamp(1)
                            .font_weight(if row.current {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if row.current {
                                theme.accent
                            } else {
                                theme.text
                            })
                            .child(row.metadata.title.clone()),
                    )
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .whitespace_normal()
                            .line_clamp(1)
                            .text_size(crate::widgets::TEXT_SMALL)
                            .text_color(theme.muted)
                            .child(if row.available {
                                if matches!(selection, Selection::Album { .. }) {
                                    format!("{} · {}", quality(row.metadata), row.metadata.artist)
                                } else {
                                    [row.metadata.artist.as_str(), row.metadata.album.as_str()]
                                        .into_iter()
                                        .filter(|value| !value.is_empty())
                                        .collect::<Vec<_>>()
                                        .join(" · ")
                                }
                            } else if !matches!(selection, Selection::Album { .. }) {
                                [
                                    row.metadata.artist.as_str(),
                                    row.metadata.album.as_str(),
                                    self.t(Text::Unavailable),
                                ]
                                .into_iter()
                                .filter(|value| !value.is_empty())
                                .collect::<Vec<_>>()
                                .join(" · ")
                            } else {
                                self.t(Text::Unavailable).into()
                            }),
                    ),
            )
            .child(
                div()
                    .w(rems(if compact { 2.5 } else { 3.5 }))
                    .flex_none()
                    .text_right()
                    .font(crate::widgets::numeric_font())
                    .text_color(theme.muted)
                    .child(time(row.metadata.duration_ms)),
            )
            .child(
                div()
                    .relative()
                    .flex_shrink()
                    .min_w(rems(4.))
                    .max_w(relative(0.24))
                    .child(
                        button(
                            ("selection-actions", key),
                            "sliders-horizontal",
                            self.t(Text::Actions),
                            theme,
                        )
                        .max_w_full()
                        .child(crate::widgets::icon("chevron-down", theme))
                        .when(compact, |v| {
                            v.px(rems(0.375)).text_size(crate::widgets::TEXT_CAPTION)
                        })
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                let above = f32::from(window.mouse_position().y)
                                    > f32::from(window.viewport_size().height)
                                        - crate::scale::scaled(230.);
                                this.track_menu = Some((selection, above));
                                cx.notify();
                            },
                        )),
                    )
                    .when_some(open, |v, (_, above)| {
                        v.child(
                            deferred(
                                div()
                                    .absolute()
                                    .right_0()
                                    .when(above, |v| v.bottom_full().mb(rems(0.25)))
                                    .when(!above, |v| v.top_full().mt(rems(0.25)))
                                    .child(menu),
                            )
                            .with_priority(1),
                        )
                    }),
            )
            .into_any_element();
        div().w_full().px(inset).child(line).into_any_element()
    }

    fn selection_track(&self, selection: Selection) -> Option<Track> {
        match selection {
            Selection::Album { track, .. } => self.tracks.get(&track).cloned(),
            Selection::Queue(id) => {
                let entry = self.playback.queue.entries.iter().find(|e| e.id == id)?;
                if entry.track.id <= 0 {
                    return Some(entry.track.clone());
                }
                Some(
                    self.tracks
                        .get(&entry.track.id)
                        .cloned()
                        .unwrap_or_else(|| Track {
                            available: false,
                            ..entry.track.clone()
                        }),
                )
            }
            Selection::Playlist { list, entry } => {
                let track = self
                    .playlists
                    .lists
                    .iter()
                    .find(|p| p.id == list)?
                    .entries
                    .iter()
                    .find(|e| e.id == entry)?
                    .track_id?;
                self.tracks.get(&track).cloned()
            }
        }
    }

    fn selection_action(
        &mut self,
        selection: Selection,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(action, Action::Save) {
            if let Some(track) = self.selection_track(selection) {
                self.open_playlist_dialog(Dialog::Add(vec![track]), window, cx);
            }
            return;
        }
        match selection {
            Selection::Album {
                album,
                edition,
                track,
            } => match action {
                Action::Play => {
                    if self.playback.track.as_ref().is_some_and(|t| t.id == track) {
                        self.play_pause(cx);
                    } else if let Some(index) = self
                        .snapshot
                        .albums
                        .iter()
                        .find(|a| a.id == album)
                        .and_then(|a| a.editions.get(edition))
                        .and_then(|e| e.tracks.iter().position(|t| t.id == track && t.available))
                    {
                        self.play_album(album, edition, index, cx);
                    }
                }
                Action::Next | Action::Append => {
                    if let Some(track) = self.selection_track(selection).filter(|t| t.available) {
                        self.audio_command(
                            Command::Enqueue {
                                tracks: vec![track],
                                next: matches!(action, Action::Next),
                            },
                            cx,
                        );
                    }
                }
                _ => {}
            },
            Selection::Queue(id) => match action {
                Action::Play => {
                    if self.playback.queue.current == Some(id) {
                        self.play_pause(cx);
                    } else if self.selection_track(selection).is_some_and(|t| t.available) {
                        self.audio_command(Command::PlayQueued(id), cx);
                    }
                }
                Action::Remove => self.audio_command(Command::RemoveQueued(id), cx),
                Action::Up | Action::Down => {
                    let entries = &self.playback.queue.entries;
                    let Some(at) = entries.iter().position(|e| e.id == id) else {
                        return;
                    };
                    let before = match action {
                        Action::Up if at > 0 => Some(entries[at - 1].id),
                        Action::Down if at + 1 < entries.len() => entries.get(at + 2).map(|e| e.id),
                        _ => return,
                    };
                    self.audio_command(Command::MoveQueued { id, before }, cx);
                }
                _ => {}
            },
            Selection::Playlist { list, entry } => match action {
                Action::Save => {}
                Action::Play => {
                    if self.playback.queue.current_playlist_entry() == Some(entry) {
                        self.play_pause(cx);
                    } else {
                        self.play_playlist(list, Some(entry), cx);
                    }
                }
                Action::Next | Action::Append => {
                    if let Some(track) = self.selection_track(selection).filter(|t| t.available) {
                        self.audio_command(
                            Command::Enqueue {
                                tracks: vec![track],
                                next: matches!(action, Action::Next),
                            },
                            cx,
                        );
                    }
                }
                Action::Remove => {
                    self.change_playlist(PlaylistChange::Remove { id: list, entry }, cx);
                }
                Action::Up | Action::Down => {
                    let Some(p) = self.playlists.lists.iter().find(|p| p.id == list) else {
                        return;
                    };
                    let Some(at) = p.entries.iter().position(|e| e.id == entry) else {
                        return;
                    };
                    let before = match action {
                        Action::Up if at > 0 => Some(p.entries[at - 1].id),
                        Action::Down if at + 1 < p.entries.len() => {
                            p.entries.get(at + 2).map(|e| e.id)
                        }
                        _ => return,
                    };
                    self.change_playlist(
                        PlaylistChange::Move {
                            id: list,
                            entry,
                            before,
                        },
                        cx,
                    );
                }
            },
        }
    }
}

#[cfg(test)]
#[path = "tests/track_list.rs"]
mod tests;
