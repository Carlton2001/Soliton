use crate::{
    app::{Rap, Section},
    i18n::Text,
    scrollbar::Scrollbar,
    theme::Theme,
    widgets::{button, cover, time},
};
use gpui::{prelude::*, *};
use rap_core::config::Language;

impl Rap {
    pub fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let entries = [
            (Section::Albums, "disc-3", Text::Albums),
            (Section::Artists, "users", Text::Artists),
            (Section::Genres, "tag", Text::Genres),
            (Section::Years, "calendar", Text::Years),
            (Section::Settings, "settings", Text::Settings),
        ];
        div()
            .w(px(205.))
            .flex_shrink_0()
            .h_full()
            .bg(theme.panel)
            .border_r_1()
            .border_color(theme.border)
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .py_5()
                    .px_3()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.accent)
                    .child("RAP"),
            )
            .children(
                entries
                    .into_iter()
                    .enumerate()
                    .map(|(index, (section, icon, label))| {
                        button(("nav", index), icon, self.t(label), theme)
                            .when(self.section == section, |v| v.bg(theme.card))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.focus.focus(window);
                                this.navigate(section, cx);
                            }))
                    }),
            )
            .child(div().flex_1())
            .child(
                button("add-source", "folder-plus", self.t(Text::AddSource), theme)
                    .opacity(if self.scanning { 0.5 } else { 1. })
                    .on_click(cx.listener(|this, _, _, cx| this.choose_source(cx))),
            )
            .child(
                button("rescan", "refresh-cw", self.t(Text::Rescan), theme)
                    .on_click(cx.listener(|this, _, _, cx| this.scan(Vec::new(), cx))),
            )
            .child(
                button("open-file", "play", self.t(Text::OpenFile), theme)
                    .on_click(cx.listener(|this, _, _, cx| this.open_file(cx))),
            )
            .child(
                div()
                    .px_3()
                    .text_xs()
                    .text_color(theme.muted)
                    .child(format!(
                        "{} · {}",
                        self.snapshot.albums.len(),
                        self.t(Text::Albums)
                    )),
            )
    }

    pub fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .p_5()
            .flex()
            .flex_col()
            .gap_3()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .bg(theme.panel)
                            .rounded_md()
                            .px_3()
                            .child(self.search.clone()),
                    )
                    .child(
                        button("grid", "layout-grid", self.t(Text::Grid), theme)
                            .when(self.settings.album_grid, |v| v.bg(theme.card))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings.album_grid = true;
                                this.save();
                                cx.notify();
                            })),
                    )
                    .child(
                        button("list", "list", self.t(Text::List), theme)
                            .when(!self.settings.album_grid, |v| v.bg(theme.card))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings.album_grid = false;
                                this.save();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(self.filter_chip(
                        "genre",
                        Text::Genre,
                        &self.filter.genre,
                        &self.facets.genres,
                        cx,
                    ))
                    .child(self.filter_chip(
                        "year",
                        Text::Year,
                        &self.filter.year,
                        &self.facets.years,
                        cx,
                    ))
                    .child(self.filter_chip(
                        "format",
                        Text::Format,
                        &self.filter.format,
                        &self.facets.formats,
                        cx,
                    ))
                    .child(
                        button("clear", "x", self.t(Text::Clear), theme)
                            .on_click(cx.listener(|this, _, _, cx| this.clear(cx))),
                    ),
            )
    }

    fn filter_chip(
        &self,
        id: &'static str,
        key: Text,
        selected: &Option<String>,
        _values: &[String],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        button(
            id,
            "tag",
            format!(
                "{} : {}",
                self.t(key),
                selected.as_deref().unwrap_or(self.t(Text::All))
            ),
            self.theme(),
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            this.filter_menu = Some(id);
            cx.notify();
        }))
    }

    pub fn albums_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        if self.snapshot.albums.is_empty() {
            return div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_4()
                .child(cover(None, 160., theme))
                .child(div().text_2xl().child(self.t(Text::Empty)))
                .child(div().text_color(theme.muted).child(self.t(Text::EmptyHint)))
                .child(
                    button("empty-add", "folder-plus", self.t(Text::AddSource), theme)
                        .on_click(cx.listener(|this, _, _, cx| this.choose_source(cx))),
                )
                .into_any_element();
        }
        if self.visible.is_empty() {
            return div()
                .p_8()
                .child(self.t(Text::NoResults))
                .into_any_element();
        }
        let columns = if self.settings.album_grid {
            self.columns
        } else {
            1
        };
        let rows = self.visible.len().div_ceil(columns);
        let list = uniform_list(
            "albums",
            rows,
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|row| {
                        div()
                            .id(("album-row", row))
                            .flex()
                            .w_full()
                            .px_5()
                            .gap_4()
                            .py_3()
                            .children(
                                (row * columns..((row + 1) * columns).min(this.visible.len())).map(
                                    |index| {
                                        let album = &this.snapshot.albums[this.visible[index]];
                                        let id = album.id;
                                        let frame = div()
                                            .id(("album", id as usize))
                                            .rounded_md()
                                            .when(index == this.keyboard_index, |view| {
                                                view.bg(theme.panel)
                                            })
                                            .p_2()
                                            .min_w_0()
                                            .hover(move |s| s.bg(theme.panel))
                                            .on_click(cx.listener(
                                                move |this, _: &ClickEvent, window, cx| {
                                                    this.focus.focus(window);
                                                    this.open_album(id, cx);
                                                },
                                            ));
                                        if this.settings.album_grid {
                                            frame
                                                .w(px(165.))
                                                .flex()
                                                .flex_col()
                                                .gap_2()
                                                .child(cover(album.cover(), 149., theme))
                                                .child(
                                                    div()
                                                        .truncate()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .child(album.title.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .truncate()
                                                        .text_color(theme.muted)
                                                        .child(this.album_artist(album)),
                                                )
                                                .into_any_element()
                                        } else {
                                            let edition = album.primary();
                                            let first = &edition.tracks[0].metadata;
                                            frame
                                                .w_full()
                                                .flex_1()
                                                .flex()
                                                .gap_4()
                                                .items_center()
                                                .child(cover(album.cover(), 48., theme))
                                                .child(
                                                    div()
                                                        .w(px(220.))
                                                        .truncate()
                                                        .child(album.title.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .truncate()
                                                        .text_color(theme.muted)
                                                        .child(this.album_artist(album)),
                                                )
                                                .child(
                                                    first
                                                        .year
                                                        .map(|y| y.to_string())
                                                        .unwrap_or_default(),
                                                )
                                                .child(
                                                    div()
                                                        .w(px(110.))
                                                        .truncate()
                                                        .child(first.genre.clone()),
                                                )
                                                .child(div().w(px(100.)).child(format!(
                                                    "{} {}",
                                                    first.format,
                                                    time(edition.duration_ms())
                                                )))
                                                .into_any_element()
                                        }
                                    },
                                ),
                            )
                    })
                    .collect()
            }),
        )
        .track_scroll(self.album_scroll.clone())
        .flex_1()
        .min_h_0();
        scroll_area(list, &self.album_bar, theme)
    }

    pub fn facets_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let values = match self.section {
            Section::Artists => self.facets.artists.clone(),
            Section::Genres => self.facets.genres.clone(),
            _ => self.facets.years.clone(),
        };
        let section = self.section;
        let theme = self.theme();
        let list = uniform_list(
            "facets",
            values.len(),
            cx.processor(move |_this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|index| {
                        let value = values[index].clone();
                        button(("facet", index), "disc-3", value.clone(), theme)
                            .w_full()
                            .py_4()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                match section {
                                    Section::Artists => this.filter.artist = Some(value.clone()),
                                    Section::Genres => this.filter.genre = Some(value.clone()),
                                    _ => this.filter.year = Some(value.clone()),
                                }
                                this.section = Section::Albums;
                                this.refresh(cx);
                            }))
                    })
                    .collect()
            }),
        )
        .track_scroll(self.facet_scroll.clone())
        .flex_1()
        .min_h_0();
        scroll_area(list, &self.facet_bar, theme)
    }

    pub fn settings_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let content = div()
            .id("settings-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.settings_bar.handle)
            .p_6()
            .flex()
            .flex_col()
            .gap_5()
            .child(div().text_2xl().child(self.t(Text::Settings)))
            .child(
                div()
                    .child(self.t(Text::Language))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(button("fr", "users", self.t(Text::French), theme).on_click(
                        cx.listener(|this, _, _, cx| this.set_language(Language::French, cx)),
                    ))
                    .child(
                        button("en", "users", self.t(Text::English), theme).on_click(
                            cx.listener(|this, _, _, cx| this.set_language(Language::English, cx)),
                        ),
                    ),
            )
            .child(
                div()
                    .child(self.t(Text::Theme))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        button("dark", "disc-3", self.t(Text::Dark), theme)
                            .on_click(cx.listener(|this, _, _, cx| this.set_dark(true, cx))),
                    )
                    .child(
                        button("light", "disc-3", self.t(Text::Light), theme)
                            .on_click(cx.listener(|this, _, _, cx| this.set_dark(false, cx))),
                    ),
            )
            .child(div().text_lg().child(self.t(Text::Sources)))
            .children(self.snapshot.sources.iter().map(|source| {
                let id = source.id;
                let confirming = self.pending_removal == Some(id);
                div()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(source.path.display().to_string())
                            .when(!source.available, |v| {
                                v.child(format!(" — {}", self.t(Text::Unavailable)))
                            }),
                    )
                    .when(confirming, |row| {
                        row.child(
                            div()
                                .text_color(theme.muted)
                                .child(self.t(Text::ConfirmRemove)),
                        )
                        .child(
                            button(
                                ("confirm-remove", id as usize),
                                "folder-minus",
                                self.t(Text::RemoveSource),
                                theme,
                            )
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.remove_source(id, cx)),
                            ),
                        )
                        .child(
                            button(
                                ("cancel-remove", id as usize),
                                "x",
                                self.t(Text::Cancel),
                                theme,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pending_removal = None;
                                cx.notify();
                            })),
                        )
                    })
                    .when(!confirming, |row| {
                        row.child(
                            button(
                                ("remove-source", id as usize),
                                "folder-minus",
                                self.t(Text::RemoveSource),
                                theme,
                            )
                            .opacity(if self.scanning { 0.5 } else { 1. })
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    if !this.scanning {
                                        this.pending_removal = Some(id);
                                        cx.notify();
                                    }
                                },
                            )),
                        )
                    })
            }))
            .child(
                button(
                    "settings-add",
                    "folder-plus",
                    self.t(Text::AddSource),
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.choose_source(cx))),
            );
        scroll_area(content, &self.settings_bar, theme)
    }

    pub fn footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .when(self.scanning, |view| {
                view.child(
                    div()
                        .px_5()
                        .py_1()
                        .bg(theme.card)
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(format!(
                            "{} · {} / {} · {} {}",
                            self.t(Text::Scan),
                            self.progress.indexed + self.progress.unchanged,
                            self.progress.discovered,
                            self.progress.errors,
                            self.t(Text::Errors)
                        ))
                        .child(button("cancel", "x", self.t(Text::Cancel), theme).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.cancellation.cancel();
                                cx.notify();
                            }),
                        )),
                )
            })
            .child(self.player(cx))
    }
}

/// Hosts a scrollable element with a draggable scrollbar on its right edge.
pub fn scroll_area(content: impl IntoElement, bar: &Scrollbar, theme: Theme) -> AnyElement {
    div()
        .relative()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(content)
        .child(bar.render(theme))
        .into_any_element()
}
