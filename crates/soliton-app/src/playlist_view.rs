//! Playlists are an ordinary top-level page, with virtualised editable songs.
use crate::{
    app::Soliton,
    i18n::Text,
    playlists::Dialog,
    track_list::{Selection, TrackRow},
    widgets::{TEXT_SMALL, accent_button, button, cover, icon_button, time},
};
use gpui::{prelude::*, *};
use soliton_audio::Command;

impl Soliton {
    fn playlist_cover(
        &self,
        list: &soliton_core::playlist::Playlist,
    ) -> Option<&std::path::PathBuf> {
        list.entries
            .iter()
            .filter_map(|entry| entry.track_id)
            .filter_map(|id| self.tracks.get(&id))
            .find_map(|track| track.cover.as_ref())
    }
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
                                    selection: Selection::Playlist {
                                        list: id,
                                        entry: e.id,
                                    },
                                    index,
                                    metadata: track.map_or(&e.metadata, |t| &t.metadata),
                                    cover: track.and_then(|t| t.cover.as_ref()),
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
                .gap(crate::widgets::PAGE_SECTION_GAP)
                .pb(crate::widgets::PAGE_SECTION_GAP)
                .child(
                    div()
                        .px(crate::widgets::PAGE_INSET)
                        .pt(crate::widgets::PAGE_TOP_PADDING)
                        .pb(crate::widgets::PAGE_SECTION_GAP)
                        .border_b_1()
                        .border_color(theme.border)
                        .flex_none()
                        .flex()
                        .flex_wrap()
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
                        .child(
                            accent_button("playlist-play", "play", self.t(Text::Play), theme)
                                .opacity(if playable { 1. } else { 0.4 })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.play_playlist(id, None, cx)
                                })),
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
                        )
                        .child(div().flex_1())
                        .child(
                            button(
                                "playlist-rename",
                                "pencil",
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
                        .px(crate::widgets::PAGE_INSET)
                        .flex()
                        .items_center()
                        .gap(crate::widgets::PAGE_SECTION_GAP)
                        .child(cover(self.playlist_cover(list), 64., theme))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(rems(0.25))
                                .child(
                                    div()
                                        .text_size(crate::widgets::TEXT_TITLE)
                                        .font_weight(FontWeight::MEDIUM)
                                        .w_full()
                                        .truncate()
                                        .whitespace_normal()
                                        .line_clamp(1)
                                        .child(list.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(TEXT_SMALL)
                                        .font(crate::widgets::numeric_font())
                                        .text_color(theme.muted)
                                        .child(format!(
                                            "{count} {} · {}",
                                            self.t(Text::Tracks),
                                            time(duration)
                                        )),
                                ),
                        ),
                )
                .when(count == 0, |page| {
                    page.child(
                        div()
                            .px(crate::widgets::PAGE_INSET)
                            .text_size(TEXT_SMALL)
                            .text_color(theme.muted)
                            .child(self.t(Text::PlaylistEmpty)),
                    )
                })
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
                        let playable = p.entries.iter().any(|e| {
                            e.track_id
                                .and_then(|id| this.tracks.get(&id))
                                .is_some_and(|t| t.available)
                        });
                        div()
                            .id(("playlist", id as usize))
                            .tab_index(0)
                            .border_2()
                            .border_color(gpui::transparent_black())
                            .capture_any_mouse_down(crate::widgets::keyboard_focus_only)
                            .focus(move |s| {
                                s.bg(theme.selected).border_2().border_color(theme.accent)
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.focus.focus(window);
                                this.open_playlist(id, cx);
                            }))
                            .h(rems(4.5))
                            .w_full()
                            .pl(rems(0.5))
                            .child(
                                div()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .gap(rems(0.75))
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(
                                        icon_button(
                                            ("playlist-row-play", id as usize),
                                            "play",
                                            this.t(Text::Play),
                                            false,
                                            theme,
                                        )
                                        .opacity(if playable { 1. } else { 0.35 })
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                this.play_playlist(id, None, cx)
                                            }),
                                        ),
                                    )
                                    .child(cover(this.playlist_cover(p), 56., theme))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(rems(12.))
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(crate::widgets::TEXT_SUBTITLE)
                                                    .w_full()
                                                    .truncate()
                                                    .whitespace_normal()
                                                    .line_clamp(1)
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
                                            ("playlist-rename", id as usize),
                                            "pencil",
                                            this.t(Text::RenamePlaylist),
                                            theme,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, window, cx| {
                                                cx.stop_propagation();
                                                this.open_playlist_dialog(
                                                    Dialog::Rename(id),
                                                    window,
                                                    cx,
                                                );
                                            }),
                                        ),
                                    )
                                    .child(
                                        button(
                                            ("playlist-delete", id as usize),
                                            "x",
                                            this.t(Text::Delete),
                                            theme,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, window, cx| {
                                                cx.stop_propagation();
                                                this.open_playlist_dialog(
                                                    Dialog::Delete(id),
                                                    window,
                                                    cx,
                                                );
                                            }),
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
        .px(crate::widgets::PAGE_INSET)
        .flex_1()
        .min_h_0();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(self.page_title(Text::Playlists, cx))
            .child(crate::widgets::page_toolbar(theme).child(
                accent_button("new-playlist", "plus", self.t(Text::NewPlaylist), theme).on_click(
                    cx.listener(|this, _, window, cx| {
                        this.open_playlist_dialog(Dialog::New(Vec::new()), window, cx)
                    }),
                ),
            ))
            .child(
                crate::widgets::page_body()
                    .when(count == 0, |v| {
                        v.child(
                            div()
                                .px(crate::widgets::PAGE_INSET)
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
                    .child(crate::views::scroll_area(rows, &self.playlists.bar, theme)),
            )
            .into_any_element()
    }
}
