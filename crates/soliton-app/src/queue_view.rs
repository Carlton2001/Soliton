//! One named page for the current listening queue.
use crate::{
    app::Soliton,
    i18n::Text,
    playlists::Dialog,
    track_list::{Selection, TrackRow},
    widgets::{button, cover, explained_button, quality},
};
use gpui::{prelude::*, *};
use soliton_audio::Command;
impl Soliton {
    /// Labels stay while the whole row fits; all of them drop together otherwise.
    pub(crate) fn toolbar_density(
        &self,
        labels: &[Text],
        window: &Window,
    ) -> crate::widgets::Density {
        use crate::widgets::{CONTROL_GAP, Density, PAGE_INSET, label_width};
        // Padding, borders, icon and gap of a button, and slack for rounding.
        let button = 1.5 + 2. / 16. + 1. + 0.5;
        let needed = 2. * PAGE_INSET.0
            + 1.
            + labels
                .iter()
                .map(|&label| button + CONTROL_GAP.0 + label_width(self.t(label), window))
                .sum::<f32>();
        if crate::navigation::content_width(window) >= needed {
            Density::Full
        } else {
            Density::Tight
        }
    }

    pub fn queue_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let count = self.playback.queue.entries.len();
        let rows = uniform_list(
            "queue",
            count,
            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                let order = this.reorder_preview(Selection::Queue(0));
                range
                    .map(|i| order.as_ref().map_or(i, |order| order[i]))
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
                            },
                            cx,
                        )
                    })
                    .collect()
            }),
        )
        .track_scroll(self.queue_scroll.clone())
        .py(crate::widgets::PAGE_SECTION_GAP)
        .flex_1()
        .min_h_0();
        let track = self.playback.track.as_ref();
        let album = self.current_album().map(|a| a.id);
        let restorable = self.original_queue_order().is_some();
        let mut labels = vec![Text::SaveQueue, Text::ClearQueue];
        if restorable {
            labels.push(Text::RestoreOrder);
        }
        let density = self.toolbar_density(&labels, window);
        let toolbar = crate::widgets::with_density(density, || {
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
                .when(restorable, |toolbar| {
                    toolbar.child(
                        explained_button(
                            "restore-order",
                            "undo-2",
                            self.t(Text::RestoreOrder),
                            self.t(Text::RestoreOrderHint),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(ids) = this.original_queue_order() {
                                this.audio_command(
                                    Command::MoveQueuedBlock { ids, before: None },
                                    cx,
                                );
                            }
                        })),
                    )
                })
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
                )
        });
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(self.page_title(Text::Queue, cx))
            .child(toolbar)
            .when_some(track, |page, track| {
                // Queued tracks may mix qualities: the playing one shows its own.
                page.child(
                    crate::widgets::summary_band(
                        cover(track.cover.as_ref(), crate::widgets::SUMMARY_COVER, theme),
                        crate::widgets::summary_title()
                            .when(track.metadata.is_inferred("title"), |title| title.italic())
                            .child(track.metadata.title.clone()),
                        [
                            crate::widgets::summary_line(theme)
                                .child(self.track_subtitle(&track.metadata))
                                .into_any_element(),
                            crate::widgets::summary_line(theme)
                                .child(quality(&track.metadata))
                                .into_any_element(),
                        ],
                        theme,
                    )
                    .when_some(album, |band, id| {
                        band.child(
                            button("queue-album", "disc-3", self.t(Text::GoToAlbum), theme)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.focus.focus(window);
                                    this.open_album(id, cx);
                                })),
                        )
                    }),
                )
            })
            .child(
                crate::widgets::page_body()
                    .when(count == 0, |v| {
                        v.child(
                            crate::widgets::page_intro().child(
                                div()
                                    .px(crate::widgets::PAGE_INSET)
                                    .text_color(theme.muted)
                                    .child(self.t(Text::QueueEmpty)),
                            ),
                        )
                    })
                    .child(
                        self.reorder_area(
                            crate::views::scroll_area(rows, &self.queue_bar, theme),
                            cx,
                        ),
                    ),
            )
            .into_any_element()
    }
}
