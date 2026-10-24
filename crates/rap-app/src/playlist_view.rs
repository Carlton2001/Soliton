//! Playlists are an ordinary top-level page, with virtualised editable songs.
use crate::{
    app::Rap,
    i18n::Text,
    playlists::Dialog,
    track_list::{Selection, TrackRow},
    widgets::{TEXT_DISPLAY, TEXT_SMALL, accent_button, button, icon, time},
};
use gpui::{prelude::*, *};
use rap_audio::Command;

impl Rap {
    pub fn playlists_view(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(list) = self
            .playlists
            .selected
            .and_then(|id| self.playlists.lists.iter().find(|p| p.id == id))
        {
            let id = list.id;
            let known = self.tracks.clone();
            let count = list.entries.len();
            let duration = list
                .entries
                .iter()
                .map(|e| {
                    known
                        .get(&e.track_id.unwrap_or(-1))
                        .map_or(e.metadata.duration_ms, |t| t.metadata.duration_ms)
                })
                .sum();
            let playable = list.entries.iter().any(|e| {
                e.track_id
                    .and_then(|id| known.get(&id))
                    .is_some_and(|t| t.available)
            });
            let theme = self.theme();
            let rows = uniform_list(
                "playlist-tracks",
                count,
                cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                    let Some(list) = this.playlists.lists.iter().find(|p| p.id == id) else {
                        return Vec::new();
                    };
                    let current = this.playback.queue.current_playlist_entry();
                    range
                        .filter_map(|index| list.entries.get(index).map(|e| (index, e)))
                        .map(|(index, e)| {
                            let track = e.track_id.and_then(|id| known.get(&id));
                            this.selection_row(
                                TrackRow {
                                    multi_disc: false,
                                    selection: Selection::Playlist {
                                        list: id,
                                        entry: e.id,
                                    },
                                    index,
                                    metadata: track.map_or(&e.metadata, |t| &t.metadata),
                                    available: track.is_some_and(|t| t.available),
                                    current: current == Some(e.id),
                                    first: index == 0,
                                    last: index + 1 == list.entries.len(),
                                },
                                cx,
                            )
                        })
                        .collect()
                }),
            )
            .track_scroll(self.playlists.scroll.clone())
            .flex_1()
            .min_h_0();
            return div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .gap(rems(0.75))
                .child(
                    div()
                        .px(rems(0.75))
                        .pt(rems(0.75))
                        .flex()
                        .items_center()
                        .gap(rems(0.5))
                        .child(
                            button(
                                "playlist-back",
                                "arrow-left",
                                self.t(Text::Playlists),
                                theme,
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    this.focus.focus(window);
                                    this.playlists.selected = None;
                                    cx.notify();
                                },
                            )),
                        )
                        .child(div().flex_1())
                        .child(
                            button(
                                "playlist-rename",
                                "sliders-horizontal",
                                self.t(Text::RenamePlaylist),
                                theme,
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.open_playlist_dialog(Dialog::Rename(id), window, cx)
                                },
                            )),
                        )
                        .child(
                            button("playlist-delete", "x", self.t(Text::DeletePlaylist), theme)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_playlist_dialog(Dialog::Delete(id), window, cx)
                                })),
                        ),
                )
                .child(
                    div()
                        .px(rems(0.75))
                        .flex()
                        .flex_col()
                        .gap(rems(0.5))
                        .child(div().text_size(TEXT_DISPLAY).child(list.name.clone()))
                        .child(
                            div()
                                .font(crate::widgets::numeric_font())
                                .text_color(theme.muted)
                                .child(format!(
                                    "{count} {} · {}",
                                    self.t(Text::Tracks),
                                    time(duration)
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(rems(0.5))
                                .flex_wrap()
                                .child(
                                    accent_button(
                                        "playlist-play",
                                        "play",
                                        self.t(Text::Play),
                                        theme,
                                    )
                                    .opacity(if playable { 1. } else { 0.4 })
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| this.play_playlist(id, None, cx),
                                    )),
                                )
                                .child(
                                    button(
                                        "playlist-enqueue",
                                        "list-end",
                                        self.t(Text::AppendQueue),
                                        theme,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            if let Some(p) =
                                                this.playlists.lists.iter().find(|p| p.id == id)
                                            {
                                                let tracks = this.playlist_tracks(p);
                                                if !tracks.is_empty() {
                                                    this.audio_command(
                                                        Command::Enqueue {
                                                            tracks,
                                                            next: false,
                                                        },
                                                        cx,
                                                    );
                                                }
                                            }
                                        },
                                    )),
                                ),
                        )
                        .child(
                            div()
                                .text_size(TEXT_SMALL)
                                .text_color(theme.muted)
                                .child(self.t(if count == 0 {
                                    Text::PlaylistEmpty
                                } else {
                                    Text::QueueUnavailable
                                })),
                        ),
                )
                .child(crate::views::scroll_area(rows, &self.playlists.bar, theme))
                .into_any_element();
        }
        let theme = self.theme();
        let count = self.playlists.lists.len();
        let rows = uniform_list(
            "playlists",
            count,
            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                let theme = this.theme();
                range
                    .map(|i| {
                        let p = &this.playlists.lists[i];
                        let id = p.id;
                        div()
                            .id(("playlist", id as usize))
                            .tab_index(0)
                            .focus(move |s| {
                                s.bg(theme.selected).border_2().border_color(theme.accent)
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.focus.focus(window);
                                this.open_playlist(id, cx);
                            }))
                            .h(rems(5.5))
                            .px(rems(0.75))
                            .child(
                                div()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .gap(rems(0.75))
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(
                                        div()
                                            .size(rems(3.5))
                                            .flex_none()
                                            .bg(theme.selected)
                                            .rounded(crate::widgets::RADIUS)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(icon("list", theme)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(crate::widgets::TEXT_SUBTITLE)
                                                    .truncate()
                                                    .child(p.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(TEXT_SMALL)
                                                    .text_color(theme.muted)
                                                    .child(format!(
                                                        "{} {}",
                                                        p.entries.len(),
                                                        this.t(Text::Tracks)
                                                    )),
                                            ),
                                    )
                                    .child(
                                        button(
                                            ("playlist-open", id as usize),
                                            "list",
                                            this.t(Text::OpenPlaylist),
                                            theme,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, window, cx| {
                                                this.focus.focus(window);
                                                cx.stop_propagation();
                                                this.open_playlist(id, cx);
                                            }),
                                        ),
                                    ),
                            )
                    })
                    .collect()
            }),
        )
        .track_scroll(self.playlists.scroll.clone())
        .flex_1()
        .min_h_0();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .p(rems(0.75))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(rems(0.75))
                    .flex_wrap()
                    .child(
                        accent_button("new-playlist", "plus", self.t(Text::NewPlaylist), theme)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_playlist_dialog(Dialog::New(Vec::new()), window, cx)
                            })),
                    ),
            )
            .when(count == 0, |v| {
                v.child(
                    div()
                        .p(rems(0.75))
                        .flex()
                        .flex_col()
                        .gap(rems(0.5))
                        .child(
                            div()
                                .text_size(crate::widgets::TEXT_TITLE)
                                .child(self.t(Text::NoPlaylists)),
                        )
                        .child(
                            div()
                                .text_color(theme.muted)
                                .child(self.t(Text::NoPlaylistsHint)),
                        ),
                )
            })
            .child(crate::views::scroll_area(rows, &self.playlists.bar, theme))
            .into_any_element()
    }
}
