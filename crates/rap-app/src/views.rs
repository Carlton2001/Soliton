use crate::{
    app::{Rap, Section},
    catalog::Period,
    i18n::Text,
    scrollbar::Scrollbar,
    theme::Theme,
    widgets::{button, choice, cover, time},
};
use gpui::{prelude::*, *};
use rap_core::config::CoverSize;

/// Side of album covers in the grid.
pub fn cover_side(size: CoverSize) -> f32 {
    match size {
        CoverSize::Small => 120.,
        CoverSize::Medium => 149.,
        CoverSize::Large => 196.,
    }
}

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
                        choice(
                            ("nav", index),
                            icon,
                            self.t(label),
                            self.section == section,
                            theme,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.focus.focus(window);
                                this.navigate(section, cx);
                            },
                        ))
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
                    .opacity(if self.scanning { 0.5 } else { 1. })
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
        let albums = self.section == Section::Albums && self.selected.is_none();
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
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .mr_2()
                            .bg(theme.panel)
                            .rounded_md()
                            .px_3()
                            .child(self.search.clone()),
                    )
                    .when(albums, |row| {
                        row.child(self.sort_button(cx))
                            .child(
                                choice(
                                    "grid",
                                    "layout-grid",
                                    self.t(Text::Grid),
                                    self.settings.album_grid,
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.set_grid(true, cx))),
                            )
                            .child(
                                choice(
                                    "list",
                                    "list",
                                    self.t(Text::List),
                                    !self.settings.album_grid,
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.set_grid(false, cx))),
                            )
                    }),
            )
            .when(self.section != Section::Settings, |header| {
                header.child(self.filter_bar(cx))
            })
    }

    fn empty_state(
        &self,
        title: &'static str,
        hint: &'static str,
        action: Option<AnyElement>,
    ) -> AnyElement {
        let theme = self.theme();
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .child(cover(None, 120., theme))
            .child(div().text_xl().child(title))
            .child(div().text_color(theme.muted).child(hint))
            .children(action)
            .into_any_element()
    }

    pub fn albums_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        if self.snapshot.albums.is_empty() {
            let add = button("empty-add", "folder-plus", self.t(Text::AddSource), theme)
                .bg(theme.card)
                .on_click(cx.listener(|this, _, _, cx| this.choose_source(cx)));
            return self.empty_state(
                self.t(Text::Empty),
                self.t(Text::EmptyHint),
                Some(add.into_any_element()),
            );
        }
        if self.visible.is_empty() {
            let clear = button("empty-clear", "x", self.t(Text::Clear), theme)
                .bg(theme.card)
                .on_click(cx.listener(|this, _, _, cx| this.clear(cx)));
            return self.empty_state(
                self.t(Text::NoResults),
                self.t(Text::NoResultsHint),
                Some(clear.into_any_element()),
            );
        }
        let columns = if self.settings.album_grid {
            self.columns
        } else {
            1
        };
        let side = cover_side(self.settings.cover_size);
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
                                (row * columns..((row + 1) * columns).min(this.visible.len()))
                                    .map(|index| this.album_tile(index, side, cx)),
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

    fn album_tile(&self, index: usize, side: f32, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let album = &self.snapshot.albums[self.visible[index]];
        let id = album.id;
        let available = album.editions.iter().any(|e| e.available());
        let frame = div()
            .id(("album", id as usize))
            .rounded_md()
            .when(index == self.keyboard_index, |view| view.bg(theme.panel))
            .when(!available, |view| view.opacity(0.55))
            .p_2()
            .min_w_0()
            .hover(move |s| s.bg(theme.panel))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.focus.focus(window);
                this.keyboard_index = index;
                this.open_album(id, cx);
            }));
        if self.settings.album_grid {
            return frame
                .w(px(side + 16.))
                .flex()
                .flex_col()
                .gap_2()
                .child(cover(album.cover(), side, theme))
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
                        .child(self.album_artist(album)),
                )
                .into_any_element();
        }
        let edition = album.primary();
        let first = &edition.tracks[0].metadata;
        frame
            .w_full()
            .flex_1()
            .flex()
            .gap_4()
            .items_center()
            .child(cover(album.cover(), 48., theme))
            .child(div().w(px(220.)).truncate().child(album.title.clone()))
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .text_color(theme.muted)
                    .child(self.album_artist(album)),
            )
            .child(
                div()
                    .w(px(48.))
                    .child(first.year.map(|y| y.to_string()).unwrap_or_default()),
            )
            .child(div().w(px(110.)).truncate().child(first.genre.clone()))
            .child(div().w(px(110.)).child(format!(
                "{} {}",
                first.format,
                time(edition.duration_ms())
            )))
            .when(!available, |row| {
                row.child(
                    div()
                        .text_color(theme.muted)
                        .child(self.t(Text::Unavailable)),
                )
            })
            .into_any_element()
    }

    pub fn facets_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let values: Vec<String> = match self.section {
            Section::Artists => self.facets.artists.clone(),
            Section::Genres => self.facets.genres.clone(),
            _ => self.facets.years.iter().map(u32::to_string).collect(),
        };
        if values.is_empty() {
            return self.empty_state(self.t(Text::EmptyFacet), "", None);
        }
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
                                    Section::Artists => return this.show_artist(value.clone(), cx),
                                    Section::Genres => this.filter.genre = Some(value.clone()),
                                    _ => this.filter.period = value.parse().ok().map(Period::Year),
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
            .when(self.preparing.is_some(), |view| {
                view.child(
                    div()
                        .px_5()
                        .py_1()
                        .bg(theme.card)
                        .child(self.t(Text::Preparing)),
                )
            })
            .child(self.player(cx))
    }

    /// Visible only while files are dragged over the window.
    pub fn drop_overlay(&self) -> impl IntoElement {
        let theme = self.theme();
        div()
            .id("drop-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .opacity(0.)
            .drag_over::<ExternalPaths>(move |style, _, _, _| style.opacity(1.).bg(theme.scrim))
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.accent)
                    .child(self.t(Text::DropToPlay)),
            )
            .child(div().text_color(theme.muted).child(self.t(Text::DropHint)))
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
