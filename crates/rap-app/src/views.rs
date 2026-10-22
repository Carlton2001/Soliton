use crate::{
    app::{Rap, Section},
    catalog::Period,
    i18n::Text,
    scrollbar::Scrollbar,
    theme::Theme,
    widgets::{
        CONTROL_HEIGHT, RADIUS, TEXT_CAPTION, TEXT_SMALL, TEXT_TITLE, button, cover, ghost,
        nav_item, segment, segments, time,
    },
};
use gpui::{prelude::*, *};
use rap_core::{config::CoverSize, text::fold};

/// Width of the sidebar, in rems.
pub const SIDEBAR_WIDTH: f32 = 13.;
/// Horizontal padding of the album list and grid, in rems.
pub const CONTENT_PADDING: f32 = 1.25;
/// Gap between grid tiles, in rems.
pub const TILE_GAP: f32 = 0.75;

/// Side of album covers in the grid, in pixels at the 16 px reference.
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
            (Section::Artists, "users", Text::Artists),
            (Section::Albums, "disc-3", Text::Albums),
            (Section::Genres, "tag", Text::Genres),
            (Section::Years, "calendar", Text::Years),
            (Section::Settings, "settings", Text::Settings),
        ];
        div()
            .w(rems(SIDEBAR_WIDTH))
            .flex_shrink_0()
            .h_full()
            .bg(theme.panel)
            .border_r_1()
            .border_color(theme.border)
            .px(rems(0.75))
            .pt(rems(1.))
            .pb(rems(0.75))
            .flex()
            .flex_col()
            .gap(rems(0.125))
            .children(
                entries
                    .into_iter()
                    .enumerate()
                    .map(|(index, (section, icon, label))| {
                        nav_item(
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
                ghost("add-source", "folder-plus", self.t(Text::AddSource), theme)
                    .opacity(if self.scanning { 0.5 } else { 1. })
                    .on_click(cx.listener(|this, _, _, cx| this.choose_source(cx))),
            )
            .child(
                ghost("rescan", "refresh-cw", self.t(Text::Rescan), theme)
                    .opacity(if self.scanning { 0.5 } else { 1. })
                    .on_click(cx.listener(|this, _, _, cx| this.scan(Vec::new(), cx))),
            )
            .child(
                ghost("open-file", "play", self.t(Text::OpenFile), theme)
                    .on_click(cx.listener(|this, _, _, cx| this.open_file(cx))),
            )
            .child(
                div()
                    .px(rems(0.75))
                    .pt(rems(0.5))
                    .text_size(TEXT_CAPTION)
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
        let grid = match self.section {
            Section::Albums if albums => Some(self.settings.album_grid),
            Section::Artists => Some(self.settings.artist_grid),
            _ => None,
        };
        div()
            .px(rems(CONTENT_PADDING))
            .py(rems(0.875))
            .flex()
            .flex_col()
            .gap(rems(0.625))
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(rems(0.5))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .mr(rems(0.5))
                            .flex()
                            .items_center()
                            .gap(rems(0.5))
                            .h(CONTROL_HEIGHT)
                            .bg(theme.panel)
                            .rounded(RADIUS)
                            .border_1()
                            .border_color(theme.control_border)
                            .px(rems(0.625))
                            .child(crate::widgets::icon("search", theme))
                            .child(div().flex_1().min_w_0().child(self.search.clone())),
                    )
                    .when(albums, |row| row.child(self.sort_button(cx)))
                    .when_some(grid, |row, grid| {
                        row.child(segments(
                            [
                                segment(
                                    "grid",
                                    Some("layout-grid"),
                                    self.t(Text::Grid),
                                    grid,
                                    theme,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.set_grid(true, cx))),
                                segment("list", Some("list"), self.t(Text::List), !grid, theme)
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.set_grid(false, cx)),
                                    ),
                            ],
                            theme,
                        ))
                    }),
            )
            .child(self.filter_bar(cx))
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
            .gap(rems(0.75))
            .p(rems(2.))
            .child(cover(None, 96., theme))
            .child(
                div()
                    .mt(rems(0.5))
                    .text_size(TEXT_TITLE)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(div().text_color(theme.muted).child(hint))
            .children(action)
            .into_any_element()
    }

    pub fn albums_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        if self.snapshot.albums.is_empty() {
            let add = button("empty-add", "folder-plus", self.t(Text::AddSource), theme)
                .mt(rems(0.5))
                .on_click(cx.listener(|this, _, _, cx| this.choose_source(cx)));
            return self.empty_state(
                self.t(Text::Empty),
                self.t(Text::EmptyHint),
                Some(add.into_any_element()),
            );
        }
        if self.visible.is_empty() {
            let clear = button("empty-clear", "x", self.t(Text::Clear), theme)
                .mt(rems(0.5))
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
                            .px(rems(CONTENT_PADDING - 0.5))
                            .gap(rems(TILE_GAP))
                            .when(columns == 1, |row| row.py(rems(0.0625)))
                            .when(columns > 1, |row| row.py(rems(0.375)))
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
            .rounded(RADIUS)
            .when(index == self.keyboard_index, |view| view.bg(theme.hover))
            .when(!available, |view| view.opacity(0.55))
            .p(rems(0.5))
            .min_w_0()
            .hover(move |s| s.bg(theme.hover))
            .active(move |s| s.bg(theme.pressed))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.focus.focus(window);
                this.keyboard_index = index;
                this.open_album(id, cx);
            }));
        if self.settings.album_grid {
            return frame
                .w(rems(side / 16. + 1.))
                .flex()
                .flex_col()
                .gap(rems(0.25))
                .child(cover(album.cover(), side, theme))
                .child(
                    div()
                        .mt(rems(0.25))
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(album.title.clone()),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(TEXT_SMALL)
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
            .gap(rems(1.))
            .items_center()
            .child(cover(album.cover(), 40., theme))
            .child(
                div()
                    .w(rems(14.))
                    .truncate()
                    .font_weight(FontWeight::MEDIUM)
                    .child(album.title.clone()),
            )
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .text_color(theme.muted)
                    .child(self.album_artist(album)),
            )
            .child(
                div()
                    .w(rems(3.))
                    .text_color(theme.muted)
                    .child(first.year.map(|y| y.to_string()).unwrap_or_default()),
            )
            .child(
                div()
                    .w(rems(7.))
                    .truncate()
                    .text_color(theme.muted)
                    .child(first.genre.clone()),
            )
            .child(div().w(rems(7.)).text_color(theme.muted).child(format!(
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

    /// Artists as tiles sharing the album grid's size and columns.
    fn artists_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self.facet_query(cx);
        let cards: Vec<_> = self
            .facets
            .artist_cards
            .iter()
            .filter(|card| query.is_empty() || fold(&card.name).contains(&query))
            .cloned()
            .collect();
        if cards.is_empty() {
            return self.facet_empty(&query);
        }
        let theme = self.theme();
        let grid = self.settings.artist_grid;
        let columns = if grid { self.columns.max(1) } else { 1 };
        let side = cover_side(self.settings.cover_size);
        let list = uniform_list(
            "artists",
            cards.len().div_ceil(columns),
            cx.processor(move |_this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|row| {
                        div()
                            .id(("artist-row", row))
                            .flex()
                            .w_full()
                            .px(rems(CONTENT_PADDING - 0.5))
                            .when(grid, |row| row.py(rems(0.375)))
                            .when(!grid, |row| row.py(rems(0.0625)))
                            .gap(rems(TILE_GAP))
                            .children((row * columns..((row + 1) * columns).min(cards.len())).map(
                                |index| {
                                    let card = &cards[index];
                                    let name = card.name.clone();
                                    if !grid {
                                        return div()
                                            .id(("artist", index))
                                            .flex_1()
                                            .p(rems(0.5))
                                            .rounded(RADIUS)
                                            .flex()
                                            .items_center()
                                            .gap(rems(1.))
                                            .hover(move |s| s.bg(theme.hover))
                                            .active(move |s| s.bg(theme.pressed))
                                            .child(cover(card.cover.as_ref(), 40., theme))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(card.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_color(theme.muted)
                                                    .child(crate::i18n::album_count(card.albums)),
                                            )
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.show_artist(name.clone(), cx)
                                            }));
                                    }
                                    div()
                                        .id(("artist", index))
                                        .w(rems(side / 16. + 1.))
                                        .min_w_0()
                                        .p(rems(0.5))
                                        .rounded(RADIUS)
                                        .flex()
                                        .flex_col()
                                        .gap(rems(0.25))
                                        .hover(move |s| s.bg(theme.hover))
                                        .active(move |s| s.bg(theme.pressed))
                                        .child(cover(card.cover.as_ref(), side, theme))
                                        .child(
                                            div()
                                                .mt(rems(0.25))
                                                .truncate()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(card.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(TEXT_SMALL)
                                                .text_color(theme.muted)
                                                .child(crate::i18n::album_count(card.albums)),
                                        )
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.show_artist(name.clone(), cx)
                                        }))
                                },
                            ))
                    })
                    .collect()
            }),
        )
        .track_scroll(self.facet_scroll.clone())
        .flex_1()
        .min_h_0();
        scroll_area(list, &self.facet_bar, theme)
    }

    /// Search text narrowing the artist, genre or year list.
    fn facet_query(&self, cx: &App) -> String {
        fold(&self.search.read(cx).content)
    }

    fn facet_empty(&self, query: &str) -> AnyElement {
        if query.is_empty() {
            self.empty_state(self.t(Text::EmptyFacet), "", None)
        } else {
            self.empty_state(self.t(Text::NoFacetMatch), "", None)
        }
    }

    pub fn facets_view(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.section == Section::Artists {
            return self.artists_view(cx);
        }
        let query = self.facet_query(cx);
        let values: Vec<String> = match self.section {
            Section::Genres => self.facets.genres.clone(),
            _ => self.facets.years.iter().map(u32::to_string).collect(),
        }
        .into_iter()
        .filter(|value| query.is_empty() || fold(value).contains(&query))
        .collect();
        if values.is_empty() {
            return self.facet_empty(&query);
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
                        ghost(("facet", index), "disc-3", value.clone(), theme)
                            .mx(rems(CONTENT_PADDING - 0.5))
                            .h(rems(2.25))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                match section {
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
                        .px(rems(CONTENT_PADDING))
                        .py(rems(0.375))
                        .gap(rems(1.))
                        .bg(theme.panel)
                        .border_t_1()
                        .border_color(theme.border)
                        .text_size(TEXT_SMALL)
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
                        .px(rems(CONTENT_PADDING))
                        .py(rems(0.5))
                        .bg(theme.panel)
                        .border_t_1()
                        .border_color(theme.border)
                        .text_size(TEXT_SMALL)
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
            .gap(rems(0.75))
            .opacity(0.)
            .drag_over::<ExternalPaths>(move |style, _, _, _| style.opacity(1.).bg(theme.scrim))
            .child(
                div()
                    .text_size(crate::widgets::TEXT_DISPLAY)
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
