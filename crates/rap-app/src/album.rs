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
        let Some((album, edition_index)) = self.selected_album() else {
            return div().into_any_element();
        };
        let edition = &album.editions[edition_index];
        let theme = self.theme();
        let first = &edition.tracks[0].metadata;
        let album_id = album.id;
        let first_available = edition.tracks.iter().position(|t| t.available);
        let wide = f32::from(window.viewport_size().width) > crate::scale::scaled(1000.);
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
            .gap(rems(0.5))
            .child(
                div()
                    .text_size(crate::widgets::TEXT_SMALL)
                    .text_color(theme.muted)
                    .child(self.t(Text::Albums)),
            )
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
                    .text_size(crate::widgets::TEXT_SUBTITLE)
                    .text_color(theme.accent)
                    .rounded(crate::widgets::RADIUS)
                    .hover(move |s| s.bg(theme.hover))
                    .child(self.album_artist(album))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.show_artist(artist.clone(), cx)),
                    )
            })
            .child(
                div()
                    .mt(rems(0.5))
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
            .gap(rems(1.25))
            .min_w_0()
            .when(wide, |v| v.flex_col().w(rems(19.)).flex_none())
            .when(!wide, |v| v.items_center())
            .child(cover(
                edition.cover().or(album.cover()),
                if wide { 280. } else { 112. },
                theme,
            ))
            .child(information);
        let songs = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(rems(0.75))
            .when(album.editions.len() > 1, |v| {
                v.child(self.editions_view(album, edition_index, cx))
            })
            .child(self.tracks_view(album_id, edition_index, cx));
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(rems(1.25))
            .child(
                div()
                    .px(rems(1.5))
                    .pt(rems(1.))
                    .flex()
                    .items_center()
                    .gap(rems(0.5))
                    .flex_wrap()
                    .child(
                        button("back", "arrow-left", self.t(Text::Albums), theme).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.selected = None;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        crate::widgets::accent_button(
                            "play-album",
                            "play",
                            self.t(if first_available.is_some() {
                                Text::Play
                            } else {
                                Text::AlbumUnavailable
                            }),
                            theme,
                        )
                        .opacity(if first_available.is_some() { 1. } else { 0.5 })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(start) = first_available {
                                this.play_album(album_id, edition_index, start, cx);
                            }
                        })),
                    )
                    .child(
                        button("album-next", "skip-forward", self.t(Text::PlayNext), theme)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.enqueue_album(album_id, edition_index, true, cx)
                            })),
                    )
                    .child(
                        button("album-append", "list-end", self.t(Text::AppendQueue), theme)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.enqueue_album(album_id, edition_index, false, cx)
                            })),
                    )
                    .child(self.playlist_tools(edition.tracks.clone(), "album-to-playlist", cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(rems(1.5))
                    .px(rems(1.5))
                    .pb(rems(1.))
                    .when(!wide, |v| v.flex_col())
                    .child(record)
                    .child(songs),
            )
            .into_any_element()
    }

    /// One choice per edition, best first, shown only when several exist.
    fn editions_view(
        &self,
        album: &Album,
        selected: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
        div()
            .px(rems(1.25))
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

    fn tracks_view(&self, album_id: i64, edition: usize, cx: &mut Context<Self>) -> AnyElement {
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
                        this.selection_row(
                            crate::track_list::TrackRow {
                                selection: crate::track_list::Selection::Album {
                                    album: album_id,
                                    edition,
                                    track: track.id,
                                },
                                index,
                                metadata: &track.metadata,
                                available: track.available,
                                current: this
                                    .playback
                                    .track
                                    .as_ref()
                                    .is_some_and(|t| t.id == track.id),
                                first: index == 0,
                                last: index + 1 == tracks.len(),
                            },
                            cx,
                        )
                    })
                    .collect()
            }),
        )
        .track_scroll(self.track_scroll.clone())
        .flex_1()
        .min_h_0();
        scroll_area(list, &self.track_bar, self.theme())
    }
}
