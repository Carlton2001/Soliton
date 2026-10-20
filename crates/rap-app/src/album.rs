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

    pub fn album_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some((album, edition_index)) = self.selected_album() else {
            return div().into_any_element();
        };
        let edition = &album.editions[edition_index];
        let theme = self.theme();
        let first = &edition.tracks[0].metadata;
        let album_id = album.id;
        let first_available = edition.tracks.iter().position(|t| t.available);
        let summary = [
            first.year.map(|year| year.to_string()),
            (!first.genre.is_empty()).then(|| first.genre.clone()),
            Some(format!("{} {}", edition.tracks.len(), self.t(Text::Tracks))),
            Some(time(edition.duration_ms())),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .p_5()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        button("back", "arrow-left", self.t(Text::Back), theme).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.selected = None;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        button(
                            "play-album",
                            "play",
                            self.t(if first_available.is_some() {
                                Text::Play
                            } else {
                                Text::AlbumUnavailable
                            }),
                            theme,
                        )
                        .bg(theme.card)
                        .opacity(if first_available.is_some() { 1. } else { 0.5 })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(start) = first_available {
                                this.play_album(album_id, edition_index, start, cx);
                            }
                        })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_6()
                    .items_center()
                    .child(cover(edition.cover().or(album.cover()), 180., theme))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .min_w_0()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child(album.title.clone()),
                            )
                            .child({
                                let artist = album.artist.clone();
                                div()
                                    .id("album-artist")
                                    .text_lg()
                                    .text_color(theme.accent)
                                    .rounded_md()
                                    .when(!artist.is_empty(), |link| {
                                        link.hover(move |s| s.bg(theme.card)).on_click(cx.listener(
                                            move |this, _, _, cx| {
                                                this.show_artist(artist.clone(), cx)
                                            },
                                        ))
                                    })
                                    .child(self.album_artist(album))
                            })
                            .child(summary)
                            .child(quality(first)),
                    ),
            )
            .when(album.editions.len() > 1, |view| {
                view.child(self.editions_view(album, edition_index, cx))
            })
            .child(self.tracks_view(album_id, edition_index, cx))
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
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(div().text_color(theme.muted).child(self.t(Text::Editions)))
            .children(album.editions.iter().enumerate().map(|(index, edition)| {
                button(
                    ("edition", index),
                    "disc-3",
                    self.edition_label(edition),
                    theme,
                )
                .when(index == selected, |v| v.bg(theme.card))
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
        let theme = self.theme();
        let count = self
            .selected_album()
            .map_or(0, |(album, index)| album.editions[index].tracks.len());
        scroll_area(
            uniform_list(
                "tracks",
                count,
                cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                    let Some((album, index)) = this.selected_album() else {
                        return Vec::new();
                    };
                    let tracks = &album.editions[index].tracks;
                    range
                        .filter_map(|index| tracks.get(index).map(|track| (index, track)))
                        .map(|(index, track)| {
                            let m = &track.metadata;
                            let available = track.available;
                            div()
                                .id(("track", index))
                                .w_full()
                                .h(px(48.))
                                .flex()
                                .items_center()
                                .gap_4()
                                .px_3()
                                .border_b_1()
                                .border_color(theme.border)
                                .text_color(if track.available {
                                    theme.text
                                } else {
                                    theme.muted
                                })
                                .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                    if available && event.click_count() == 2 {
                                        this.play_album(album_id, edition, index, cx);
                                    }
                                }))
                                .child(div().w(px(40.)).child(format!(
                                    "{}.{}",
                                    m.disc.unwrap_or(1),
                                    m.track.unwrap_or((index + 1) as u32)
                                )))
                                .child(div().flex_1().truncate().child(m.title.clone()))
                                .child(div().w(px(150.)).truncate().child(m.artist.clone()))
                                .child(quality(m))
                                .when(!track.available, |v| v.child(this.t(Text::Unavailable)))
                                .child(time(m.duration_ms))
                        })
                        .collect()
                }),
            )
            .track_scroll(self.track_scroll.clone())
            .flex_1()
            .min_h_0(),
            &self.track_bar,
            theme,
        )
    }
}
