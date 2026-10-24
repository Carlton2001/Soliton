//! Album sheet: cover, summary, edition choice and the tracks of that edition.
use crate::{
    app::Rap,
    i18n::Text,
    views::scroll_area,
    widgets::{button, cover, quality, time},
};
use gpui::{prelude::*, *};
use rap_core::model::{Album, Edition};

impl Rap {
    fn selected_album(&self) -> Option<(&Album, usize)> {
        let album = self
            .snapshot
            .albums
            .iter()
            .find(|a| Some(a.id) == self.selected)?;
        // A refresh may remove editions; fall back to the best one.
        let edition = if self.selected_edition < album.editions.len() {
            self.selected_edition
        } else {
            0
        };
        Some((album, edition))
    }

    pub fn album_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        self.album_sheet(window, false, cx)
    }

    pub(super) fn album_panel(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        self.album_sheet(window, true, cx)
    }

    fn album_sheet(&self, window: &Window, panel: bool, cx: &mut Context<Self>) -> AnyElement {
        let Some((album, edition_index)) = self.selected_album() else {
            return div().into_any_element();
        };
        let edition = &album.editions[edition_index];
        let theme = self.theme();
        let first = &edition.tracks[0].metadata;
        let album_id = album.id;
        let wide = !panel && f32::from(window.viewport_size().width) > crate::scale::scaled(1000.);
        let summary = [
            first.year.map(|y| y.to_string()),
            (!first.genre.is_empty()).then(|| first.genre.clone()),
            Some(format!(
                "{} {} · {}",
                edition.tracks.len(),
                self.t(Text::Tracks),
                time(edition.duration_ms())
            )),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        let information = div()
            .flex()
            .flex_col()
            .min_w_0()
            .gap(rems(0.25))
            .child(
                div()
                    .text_size(crate::widgets::TEXT_DISPLAY)
                    .font_weight(FontWeight::MEDIUM)
                    .child(album.title.clone()),
            )
            .child({
                let artist = album.artist.clone();
                div()
                    .id("album-artist")
                    .tab_index(0)
                    .border_2()
                    .border_color(gpui::transparent_black())
                    .capture_any_mouse_down(crate::widgets::keyboard_focus_only)
                    .focus(move |s| s.bg(theme.selected).border_2().border_color(theme.accent))
                    .text_size(crate::widgets::TEXT_SUBTITLE)
                    .text_color(theme.accent)
                    .rounded(crate::widgets::RADIUS)
                    .hover(move |s| s.bg(theme.hover))
                    .child(self.album_artist(album))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.focus.focus(window);
                        this.show_artist(artist.clone(), cx);
                    }))
            })
            .child(
                div()
                    .font(crate::widgets::numeric_font())
                    .text_color(theme.muted)
                    .child(summary),
            )
            .child(
                div()
                    .text_size(crate::widgets::TEXT_SMALL)
                    .text_color(theme.muted)
                    .child(quality(first)),
            );
        let record = div()
            .flex()
            .gap(rems(0.75))
            .min_w_0()
            .when(wide, |v| v.flex_col().w(rems(19.)).flex_none())
            .when(!wide && !panel, |v| v.items_center())
            .when(panel, |v| v.flex_col().flex_none())
            .child(cover(
                edition.cover().or(album.cover()),
                if panel {
                    244.
                } else if wide {
                    280.
                } else {
                    112.
                },
                theme,
            ))
            .child(information);
        let songs = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(crate::widgets::PAGE_SECTION_GAP)
            .when(album.editions.len() > 1, |v| {
                v.child(
                    self.editions_view(album, edition_index, cx)
                        .when(panel, |view| view.px(crate::widgets::PAGE_INSET)),
                )
            })
            .child(self.tracks_view(album_id, edition_index, panel, cx));
        if panel {
            return div()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .justify_between()
                        .items_center()
                        .p(rems(0.75))
                        .child(
                            div()
                                .text_size(crate::widgets::TEXT_CAPTION)
                                .text_color(theme.muted)
                                .child(self.t(Text::SelectedAlbum)),
                        )
                        .child(
                            button("close-album-panel", "x", self.t(Text::CloseAlbum), theme)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.focus.focus(window);
                                    this.return_from_album(cx);
                                })),
                        ),
                )
                .child(
                    div()
                        .id("album-panel-summary")
                        .max_h(relative(0.65))
                        .min_h_0()
                        .overflow_y_scroll()
                        .px(crate::widgets::PAGE_INSET)
                        .pb(crate::widgets::PAGE_SECTION_GAP)
                        .child(record)
                        .child(
                            self.panel_album_actions(album, edition_index, cx)
                                .mt(crate::widgets::PAGE_SECTION_GAP),
                        ),
                )
                .child(songs)
                .into_any_element();
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(crate::widgets::PAGE_SECTION_GAP)
            .child(
                div()
                    .px(crate::widgets::PAGE_INSET)
                    .pt(crate::widgets::PAGE_TOP_PADDING)
                    .flex_none()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap(rems(0.5))
                    .child(
                        button(
                            "album-return",
                            "arrow-left",
                            self.t(Text::BackToLibrary),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.focus.focus(window);
                            this.return_from_album(cx);
                        })),
                    )
                    .child(self.panel_album_actions(album, edition_index, cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(rems(0.75))
                    .pl(crate::widgets::PAGE_INSET)
                    .pb(crate::widgets::PAGE_SECTION_GAP)
                    .when(!wide, |v| v.flex_col())
                    .child(record)
                    .child(songs),
            )
            .into_any_element()
    }

    /// Keep the main action visible; secondary album operations share one menu.
    fn panel_album_actions(
        &self,
        album: &Album,
        edition_index: usize,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = self.theme();
        let album_id = album.id;
        let edition = &album.editions[edition_index];
        let first = edition.tracks.iter().position(|t| t.available);
        let open = self.menu == Some(crate::filters::Menu::AlbumActions);
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(rems(0.5))
            .child(
                crate::widgets::accent_button(
                    "panel-play-album",
                    "play",
                    self.t(if first.is_some() {
                        Text::Play
                    } else {
                        Text::AlbumUnavailable
                    }),
                    theme,
                )
                .opacity(if first.is_some() { 1. } else { 0.5 })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(start) = first {
                        this.play_album(album_id, edition_index, start, cx);
                    }
                })),
            )
            .child(
                div()
                    .relative()
                    .child(
                        button(
                            "panel-album-actions",
                            "sliders-horizontal",
                            self.t(Text::Actions),
                            theme,
                        )
                        .child(crate::widgets::icon("chevron-down", theme))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.menu = if open {
                                None
                            } else {
                                Some(crate::filters::Menu::AlbumActions)
                            };
                            cx.notify();
                        })),
                    )
                    .when(open, |anchor| {
                        anchor.child(
                            deferred(
                                div().absolute().top_full().right_0().mt(rems(0.25)).child(
                                    crate::widgets::popover(theme)
                                        .flex()
                                        .flex_col()
                                        .gap(rems(0.25))
                                        .child(
                                            button(
                                                "panel-album-next",
                                                "skip-forward",
                                                self.t(Text::PlayNext),
                                                theme,
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.menu = None;
                                                    this.enqueue_album(
                                                        album_id,
                                                        edition_index,
                                                        true,
                                                        cx,
                                                    );
                                                }),
                                            ),
                                        )
                                        .child(
                                            button(
                                                "panel-album-append",
                                                "list-end",
                                                self.t(Text::AppendQueue),
                                                theme,
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.menu = None;
                                                    this.enqueue_album(
                                                        album_id,
                                                        edition_index,
                                                        false,
                                                        cx,
                                                    );
                                                }),
                                            ),
                                        )
                                        .child(self.playlist_tools(
                                            edition.tracks.clone(),
                                            "panel-album-to-playlist",
                                            cx,
                                        )),
                                ),
                            )
                            .with_priority(2),
                        )
                    }),
            )
    }

    /// One choice per edition, best first, shown only when several exist.
    pub(super) fn editions_view(
        &self,
        album: &Album,
        selected: usize,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = self.theme();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(rems(0.5))
            .child(div().text_color(theme.muted).child(self.t(Text::Editions)))
            .children(album.editions.iter().enumerate().map(|(index, edition)| {
                crate::widgets::toggle_button(
                    ("edition", index),
                    "disc-3",
                    self.edition_label(edition),
                    index == selected,
                    theme,
                )
                .when(!edition.available(), |v| v.text_color(theme.muted))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.selected_edition = index;
                    this.track_scroll.scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify();
                }))
            }))
    }

    /// Audio characteristics, then the folder that tells copies apart.
    fn edition_label(&self, edition: &Edition) -> String {
        let mut parts = vec![quality(&edition.tracks[0].metadata)];
        if let Some(folder) = edition.folder.file_name() {
            parts.push(folder.to_string_lossy().into_owned());
        }
        if !edition.available() {
            parts.push(self.t(Text::Unavailable).into());
        }
        parts.join(" · ")
    }

    pub(super) fn tracks_view(
        &self,
        album_id: i64,
        edition: usize,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let count = self
            .selected_album()
            .map_or(0, |(a, i)| a.editions[i].tracks.len());
        let list = uniform_list(
            "tracks",
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                let Some((album, index)) = this.selected_album() else {
                    return Vec::new();
                };
                let tracks = &album.editions[index].tracks;
                range
                    .filter_map(|i| tracks.get(i).map(|t| (i, t)))
                    .map(|(index, track)| {
                        this.album_selection_row(
                            crate::track_list::TrackRow {
                                selection: crate::track_list::Selection::Album {
                                    album: album_id,
                                    edition,
                                    track: track.id,
                                },
                                index,
                                metadata: &track.metadata,
                                cover: track.cover.as_ref(),
                                available: track.available,
                                current: this
                                    .playback
                                    .track
                                    .as_ref()
                                    .is_some_and(|t| t.id == track.id),
                                first: index == 0,
                                last: index + 1 == tracks.len(),
                            },
                            compact,
                            cx,
                        )
                    })
                    .collect()
            }),
        )
        .when(!compact, |list| list.pr(crate::widgets::PAGE_INSET))
        .track_scroll(self.track_scroll.clone())
        .flex_1()
        .min_w_0()
        .min_h_0();
        scroll_area(list, &self.track_bar, self.theme())
    }
}
