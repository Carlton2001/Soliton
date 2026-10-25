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
                                selection: Selection::Queue(e.id),
                                index,
                                metadata: &e.track.metadata,
                                cover: e.track.cover.as_ref(),
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
            .child(self.page_title(Text::Queue))
            .child(
                crate::widgets::page_toolbar(theme)
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
                            "arrow-down-from-line",
                            self.t(Text::ClearUpcoming),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.audio_command(Command::ClearUpcoming, cx)
                        })),
                    )
                    .child(
                        button("clear-queue", "eraser", self.t(Text::ClearQueue), theme)
                            .opacity(if count > 0 { 1. } else { 0.4 })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.audio_command(
                                    Command::Load {
                                        tracks: Vec::new(),
                                        start: 0,
                                        position_ms: 0,
                                    },
                                    cx,
                                );
                            })),
                    ),
            )
            .when_some(track, |v, track| {
                v.child(
                    div()
                        .mx(crate::widgets::PAGE_INSET)
                        .mb(crate::widgets::PAGE_SECTION_GAP)
                        .flex()
                        .items_center()
                        .gap(rems(0.75))
                        .child(crate::widgets::with_tooltip(
                            div().id("queue-track-details").flex_none().child(cover(
                                track.cover.as_ref(),
                                48.,
                                theme,
                            )),
                            output
                                .as_ref()
                                .map_or_else(
                                    || quality(&track.metadata),
                                    |output| {
                                        format!(
                                            "{} · {}",
                                            quality(&track.metadata),
                                            self.labelled(self.t(Text::Output), output)
                                        )
                                    },
                                )
                                .into(),
                            theme,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(rems(0.25))
                                .child(
                                    div()
                                        .text_size(crate::widgets::TEXT_BODY)
                                        .font_weight(FontWeight::MEDIUM)
                                        .w_full()
                                        .truncate()
                                        .whitespace_normal()
                                        .line_clamp(1)
                                        .child(track.metadata.title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(TEXT_SMALL)
                                        .text_color(theme.muted)
                                        .w_full()
                                        .truncate()
                                        .whitespace_normal()
                                        .line_clamp(1)
                                        .child(format!(
                                            "{} · {}",
                                            track.metadata.artist, track.metadata.album
                                        )),
                                ),
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
                        .px(crate::widgets::PAGE_INSET)
                        .pb(crate::widgets::PAGE_SECTION_GAP)
                        .text_color(theme.muted)
                        .child(self.t(Text::QueueEmpty)),
                )
            })
            .child(crate::views::scroll_area(rows, &self.queue_bar, theme))
            .into_any_element()
    }
}
