use crate::{
    app::{Rap, Section},
    i18n::Text,
    scrollbar::Scrollbar,
    theme::Theme,
    widgets::{
        RADIUS, TEXT_SMALL, TEXT_TITLE, accent_button, button, cover, segment, segments, time,
    },
};
use gpui::{prelude::*, *};
use rap_core::{config::CoverSize, text::fold};

/// Horizontal padding of the album list and grid, in rems.
pub const CONTENT_PADDING: f32 = crate::widgets::PAGE_INSET.0;
/// Gap between grid tiles, in rems.
pub const TILE_GAP: f32 = 1.;

/// Side of album covers in the grid, in pixels at the 16 px reference.
pub fn cover_side(size: CoverSize) -> f32 {
    match size {
        CoverSize::Small => 120.,
        CoverSize::Medium => 149.,
        CoverSize::Large => 196.,
    }
}

impl Rap {
    pub fn search_field(&self) -> Div {
        let theme = self.theme();
        crate::widgets::input_field(self.search.clone(), true, theme)
    }

    pub fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let albums = self.section == Section::Albums;
        let grid = match self.section {
            Section::Albums => Some(self.settings.album_grid),
            Section::Artists => Some(self.settings.artist_grid),
            _ => None,
        };
        div()
            .flex_none()
            .px(rems(CONTENT_PADDING))
            .pt(crate::widgets::PAGE_TOP_PADDING)
            .pb(crate::widgets::PAGE_SECTION_GAP)
            .flex()
            .flex_col()
            .gap(crate::widgets::PAGE_SECTION_GAP)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(crate::widgets::CONTROL_GAP)
                    .when(albums, |row| {
                        let playable = !self.visible.is_empty();
                        row.child(
                            accent_button("play-all", "play", self.t(Text::PlayAll), theme)
                                .when(self.compact_controls, |button| {
                                    crate::widgets::compact_button(button, false)
                                })
                                .opacity(if playable { 1. } else { 0.5 })
                                .on_click(cx.listener(|this, _, _, cx| this.play_all(cx))),
                        )
                    })
                    .child(self.search_field().w(rems(19.)).max_w_full())
                    .when(albums, |row| row.child(self.filter_button(cx)))
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
            .when(albums && self.filter.active(), |view| {
                view.child(self.filter_bar(cx))
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
            .gap(rems(0.75))
            .p(rems(0.75))
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
                            .px(rems(CONTENT_PADDING))
                            .gap(rems(TILE_GAP))
                            .when(columns == 1, |row| row.py(rems(0.0625)))
                            .when(columns > 1, |row| row.pb(crate::widgets::PAGE_SECTION_GAP))
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
            .tab_index(0)
            .border_2()
            .border_color(gpui::transparent_black())
            .capture_any_mouse_down(crate::widgets::keyboard_focus_only)
            .focus(move |s| s.bg(theme.selected).border_2().border_color(theme.accent))
            .rounded(RADIUS)
            .when(Some(id) == self.selected, |view| view.bg(theme.selected))
            .when(
                index == self.keyboard_index && Some(id) != self.selected,
                |view| view.bg(theme.hover),
            )
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
                        .w_full()
                        .truncate()
                        .whitespace_normal()
                        .line_clamp(1)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(album.title.clone()),
                )
                .child(
                    div()
                        .w_full()
                        .truncate()
                        .whitespace_normal()
                        .line_clamp(1)
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
            .gap(rems(0.75))
            .items_center()
            .child(cover(album.cover(), 40., theme))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(rems(0.25))
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .whitespace_normal()
                            .line_clamp(1)
                            .font_weight(FontWeight::MEDIUM)
                            .child(album.title.clone()),
                    )
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .whitespace_normal()
                            .line_clamp(1)
                            .text_size(TEXT_SMALL)
                            .text_color(theme.muted)
                            .child(self.album_artist(album)),
                    )
                    .when(!available, |v| {
                        v.child(
                            div()
                                .text_size(TEXT_SMALL)
                                .text_color(theme.muted)
                                .child(self.t(Text::Unavailable)),
                        )
                    }),
            )
            .child(
                div()
                    .max_w(rems(11.))
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(rems(0.25))
                    .text_right()
                    .text_size(TEXT_SMALL)
                    .text_color(theme.muted)
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .whitespace_normal()
                            .line_clamp(1)
                            .child(
                                first
                                    .year
                                    .map(|y| y.to_string())
                                    .into_iter()
                                    .chain((!first.genre.is_empty()).then(|| first.genre.clone()))
                                    .collect::<Vec<_>>()
                                    .join(" · "),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .truncate()
                            .whitespace_normal()
                            .line_clamp(1)
                            .font(crate::widgets::numeric_font())
                            .child(format!(
                                "{} · {}",
                                first.format,
                                time(edition.duration_ms())
                            )),
                    ),
            )
            .into_any_element()
    }

    /// Artists as tiles sharing the album grid's size and columns.
    pub fn artists_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self.facet_query(cx);
        let cards: Vec<_> = self
            .facets
            .artist_cards
            .iter()
            .filter(|card| query.is_empty() || fold(&card.name).contains(&query))
            .cloned()
            .collect();
        if cards.is_empty() {
            if self.snapshot.albums.is_empty() {
                let add = button(
                    "artists-empty-add",
                    "folder-plus",
                    self.t(Text::AddSource),
                    self.theme(),
                )
                .on_click(cx.listener(|this, _, _, cx| this.choose_source(cx)));
                return self.empty_state(
                    self.t(Text::Empty),
                    self.t(Text::EmptyHint),
                    Some(add.into_any_element()),
                );
            }
            return self.facet_empty(&query);
        }
        let theme = self.theme();
        let grid = self.settings.artist_grid;
        let columns = if grid { self.columns.max(1) } else { 1 };
        let side = cover_side(self.settings.cover_size);
        let list = uniform_list(
            "artists",
            cards.len().div_ceil(columns),
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|row| {
                        div()
                            .id(("artist-row", row))
                            .flex()
                            .w_full()
                            .px(rems(CONTENT_PADDING))
                            .when(grid, |row| row.pb(crate::widgets::PAGE_SECTION_GAP))
                            .when(!grid, |row| row.py(rems(0.0625)))
                            .gap(rems(TILE_GAP))
                            .children((row * columns..((row + 1) * columns).min(cards.len())).map(
                                |index| {
                                    let card = &cards[index];
                                    let name = card.name.clone();
                                    if !grid {
                                        return div()
                                            .id(("artist", index))
                                            .when(index == this.keyboard_index, |v| {
                                                v.bg(theme.hover)
                                            })
                                            .tab_index(0)
                                            .border_2()
                                            .border_color(gpui::transparent_black())
                                            .capture_any_mouse_down(
                                                crate::widgets::keyboard_focus_only,
                                            )
                                            .focus(move |s| {
                                                s.bg(theme.selected)
                                                    .border_2()
                                                    .border_color(theme.accent)
                                            })
                                            .flex_1()
                                            .p(rems(0.5))
                                            .rounded(RADIUS)
                                            .flex()
                                            .items_center()
                                            .gap(rems(0.75))
                                            .hover(move |s| s.bg(theme.hover))
                                            .active(move |s| s.bg(theme.pressed))
                                            .child(cover(card.cover.as_ref(), 40., theme))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .w_full()
                                                    .truncate()
                                                    .whitespace_normal()
                                                    .line_clamp(1)
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(card.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_color(theme.muted)
                                                    .child(crate::i18n::album_count(card.albums)),
                                            )
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.focus.focus(window);
                                                this.show_artist(name.clone(), cx)
                                            }));
                                    }
                                    div()
                                        .id(("artist", index))
                                        .when(index == this.keyboard_index, |v| v.bg(theme.hover))
                                        .tab_index(0)
                                        .border_2()
                                        .border_color(gpui::transparent_black())
                                        .capture_any_mouse_down(crate::widgets::keyboard_focus_only)
                                        .focus(move |s| {
                                            s.bg(theme.selected)
                                                .border_2()
                                                .border_color(theme.accent)
                                        })
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
                                                .w_full()
                                                .truncate()
                                                .whitespace_normal()
                                                .line_clamp(1)
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(card.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(TEXT_SMALL)
                                                .text_color(theme.muted)
                                                .child(crate::i18n::album_count(card.albums)),
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.focus.focus(window);
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

    pub fn footer(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                        .gap(rems(0.75))
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
            .child(self.player(window, cx))
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
        .pr(bar.gutter())
        .child(content)
        .child(bar.render(theme))
        .into_any_element()
}
