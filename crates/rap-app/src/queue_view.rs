//! One named page for the current listening queue.
use crate::{
    app::Rap,
    i18n::Text,
    playlists::Dialog,
    track_list::{Selection, TrackRow},
    widgets::{TEXT_SMALL, button, cover, quality},
};
use gpui::{prelude::*, *};
use rap_audio::Command;
impl Rap {
    pub fn queue_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let count = self.playback.queue.entries.len();
        let rows = uniform_list(
            "queue",
            count,
            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                range
                    .filter_map(|i| this.playback.queue.entries.get(i).map(|e| (i, e)))
                    .map(|(index, e)| {
                        this.selection_row(
                            TrackRow {
                                multi_disc: false,
                                selection: Selection::Queue(e.id),
                                index,
                                metadata: &e.track.metadata,
                                available: if e.track.id > 0 {
                                    this.tracks.get(&e.track.id).is_some_and(|t| t.available)
                                } else {
                                    e.track.available
                                },
                                current: this.playback.queue.current == Some(e.id),
                                first: index == 0,
                                last: index + 1 == this.playback.queue.entries.len(),
                            },
                            cx,
                        )
                    })
                    .collect()
            }),
        )
        .track_scroll(self.queue_scroll.clone())
        .flex_1()
        .min_h_0();
        let track = self.playback.track.as_ref();
        let album = self.current_album().map(|a| a.id);
        let output = self.output_description();
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
                    .gap(rems(0.75))
                    .flex_wrap()
                    .child(
                        button("save-queue", "list-plus", self.t(Text::SaveQueue), theme)
                            .opacity(if count > 0 { 1. } else { 0.4 })
                            .on_click(cx.listener(|this, _, window, cx| {
                                let tracks = this
                                    .playback
                                    .queue
                                    .entries
                                    .iter()
                                    .map(|e| e.track.clone())
                                    .filter(|t| t.id > 0)
                                    .collect::<Vec<_>>();
                                if !tracks.is_empty() {
                                    this.open_playlist_dialog(Dialog::New(tracks), window, cx);
                                }
                            })),
                    )
                    .child(
                        button(
                            "clear-upcoming",
                            "minus",
                            self.t(Text::ClearUpcoming),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.audio_command(Command::ClearUpcoming, cx)
                        })),
                    ),
            )
            .when_some(track, |v, track| {
                v.child(
                    div()
                        .mx(rems(0.75))
                        .mb(rems(0.75))
                        .pb(rems(0.75))
                        .border_b_1()
                        .border_color(theme.border)
                        .flex()
                        .items_center()
                        .gap(rems(0.75))
                        .child(cover(track.cover.as_ref(), 64., theme))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(TEXT_SMALL)
                                        .text_color(theme.muted)
                                        .child(self.t(Text::NowPlaying)),
                                )
                                .child(
                                    div()
                                        .text_size(crate::widgets::TEXT_SUBTITLE)
                                        .truncate()
                                        .child(track.metadata.title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(TEXT_SMALL)
                                        .text_color(theme.muted)
                                        .truncate()
                                        .child(format!(
                                            "{} · {}",
                                            track.metadata.artist,
                                            quality(&track.metadata)
                                        )),
                                )
                                .when_some(output, |v, output| {
                                    v.child(
                                        div()
                                            .text_size(TEXT_SMALL)
                                            .text_color(theme.muted)
                                            .truncate()
                                            .child(self.labelled(self.t(Text::Output), &output)),
                                    )
                                }),
                        )
                        .when_some(album, |v, id| {
                            v.child(
                                button("queue-album", "disc-3", self.t(Text::GoToAlbum), theme)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.focus.focus(window);
                                        this.open_album(id, cx);
                                    })),
                            )
                        }),
                )
            })
            .when(count == 0, |v| {
                v.child(
                    div()
                        .p(rems(0.75))
                        .text_color(theme.muted)
                        .child(self.t(Text::QueueEmpty)),
                )
            })
            .child(crate::views::scroll_area(rows, &self.queue_bar, theme))
            .into_any_element()
    }
}
