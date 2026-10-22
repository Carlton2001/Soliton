//! Combinable filters: one discreet button, removable chips for active filters,
//! and a menu listing the values of each kind. Also the album sort menu.
use crate::{
    app::{Rap, Section},
    catalog::{Period, Quality},
    i18n::{Text, decade},
    widgets::{
        CONTROL_HEIGHT, RADIUS, button, menu_item, nav_item, popover, tinted_icon, toggle_button,
    },
};
use gpui::{prelude::*, *};
use rap_core::{config::AlbumSort, text::fold};
use std::{
    collections::{BTreeSet, HashSet},
    rc::Rc,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    Genre,
    Period,
    Artist,
    Format,
    Quality,
}

impl FilterKind {
    const ALL: [FilterKind; 5] = [
        FilterKind::Genre,
        FilterKind::Period,
        FilterKind::Artist,
        FilterKind::Format,
        FilterKind::Quality,
    ];
    fn label(self) -> Text {
        match self {
            FilterKind::Genre => Text::Genre,
            FilterKind::Period => Text::Period,
            FilterKind::Artist => Text::Artist,
            FilterKind::Format => Text::Format,
            FilterKind::Quality => Text::Quality,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Filters(FilterKind),
    Sort,
}

#[derive(Clone)]
enum Value {
    Text(String),
    Period(Period),
    Quality(Quality),
}

pub const SORTS: [AlbumSort; 4] = [
    AlbumSort::Artist,
    AlbumSort::Title,
    AlbumSort::Year,
    AlbumSort::Added,
];

impl Rap {
    pub fn sort_label(&self, sort: AlbumSort) -> &'static str {
        self.t(match sort {
            AlbumSort::Artist => Text::Artist,
            AlbumSort::Title => Text::SortTitle,
            AlbumSort::Year => Text::Year,
            AlbumSort::Added => Text::SortAdded,
        })
    }

    fn quality_label(&self, quality: Quality) -> &'static str {
        self.t(match quality {
            Quality::HiRes => Text::HiRes,
            Quality::Cd => Text::CdQuality,
            Quality::Lossy => Text::Lossy,
        })
    }

    fn period_label(&self, period: Period) -> String {
        match period {
            Period::Decade(value) => decade(self.settings.language, value),
            Period::Year(value) => value.to_string(),
        }
    }

    fn current(&self, kind: FilterKind) -> Option<String> {
        let f = &self.filter;
        match kind {
            FilterKind::Genre => f.genre.clone(),
            FilterKind::Period => f.period.map(|p| self.period_label(p)),
            FilterKind::Artist => f.artist.clone(),
            FilterKind::Format => f.format.clone(),
            FilterKind::Quality => f.quality.map(|q| self.quality_label(q).into()),
        }
    }

    /// Values compatible with the other active filters.
    fn values(&self, kind: FilterKind) -> Vec<(String, Value)> {
        let facets = &self.facets;
        let options = &self.filter_options;
        let texts = |values: &[String], allowed: &HashSet<String>, folded: bool| {
            values
                .iter()
                .filter(|v| allowed.contains(&if folded { fold(v) } else { (*v).clone() }))
                .map(|v| (v.clone(), Value::Text(v.clone())))
                .collect()
        };
        match kind {
            FilterKind::Genre => texts(&facets.genres, &options.genres, true),
            FilterKind::Artist => texts(&facets.artists, &options.artists, true),
            FilterKind::Format => texts(&facets.formats, &options.formats, false),
            FilterKind::Quality => Quality::ALL
                .iter()
                .filter(|q| options.qualities.contains(q))
                .map(|&q| (self.quality_label(q).into(), Value::Quality(q)))
                .collect(),
            FilterKind::Period => {
                let decades: BTreeSet<u32> = options.years.iter().map(|y| y / 10 * 10).collect();
                decades
                    .into_iter()
                    .map(Period::Decade)
                    .chain(options.years.iter().map(|&y| Period::Year(y)))
                    .map(|p| (self.period_label(p), Value::Period(p)))
                    .collect()
            }
        }
    }

    fn apply(&mut self, kind: FilterKind, value: Option<Value>, cx: &mut Context<Self>) {
        let text = |value: Option<Value>| match value {
            Some(Value::Text(text)) => Some(text),
            _ => None,
        };
        let f = &mut self.filter;
        match kind {
            FilterKind::Genre => f.genre = text(value),
            FilterKind::Artist => f.artist = text(value),
            FilterKind::Format => f.format = text(value),
            FilterKind::Period => {
                f.period = match value {
                    Some(Value::Period(p)) => Some(p),
                    _ => None,
                }
            }
            FilterKind::Quality => {
                f.quality = match value {
                    Some(Value::Quality(q)) => Some(q),
                    _ => None,
                }
            }
        }
        self.selected = None;
        if self.section != Section::Settings {
            self.section = Section::Albums;
        }
        self.album_scroll.scroll_to_item(0, ScrollStrategy::Top);
        self.refresh(cx);
    }

    fn open_menu(&mut self, menu: Menu, cx: &mut Context<Self>) {
        self.menu = Some(menu);
        self.menu_search.update(cx, |input, cx| {
            input.reset();
            cx.notify();
        });
        self.menu_scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }

    /// The filter button, one removable chip per active filter, and a reset.
    pub fn filter_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let active = self.filter.active();
        let open = matches!(self.menu, Some(Menu::Filters(_)));
        div()
            .flex()
            .flex_wrap()
            .gap(rems(0.5))
            .items_center()
            .child(
                div()
                    .relative()
                    .child(
                        toggle_button(
                            "filters",
                            "sliders-horizontal",
                            self.t(Text::Filters),
                            open,
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            let kind = FilterKind::ALL
                                .into_iter()
                                .find(|&k| this.current(k).is_some())
                                .unwrap_or(FilterKind::Genre);
                            this.open_menu(Menu::Filters(kind), cx)
                        })),
                    )
                    .children(match self.menu {
                        Some(Menu::Filters(kind)) => {
                            Some(dropdown(self.filters_menu(kind, cx), false))
                        }
                        _ => None,
                    }),
            )
            .children(FilterKind::ALL.into_iter().filter_map(|kind| {
                let value = self.current(kind)?;
                Some(
                    div()
                        .id(("chip", kind as usize))
                        .h(CONTROL_HEIGHT)
                        .flex()
                        .items_center()
                        .gap(rems(0.375))
                        .pl(rems(0.75))
                        .pr(rems(0.5))
                        .rounded(RADIUS)
                        .border_1()
                        .border_color(theme.accent)
                        .bg(theme.selected)
                        .text_color(theme.accent)
                        .hover(move |s| s.bg(theme.hover))
                        .active(move |s| s.bg(theme.pressed))
                        .child(self.labelled(self.t(kind.label()), &value))
                        .child(tinted_icon("x", theme.accent))
                        .on_click(cx.listener(move |this, _, _, cx| this.apply(kind, None, cx))),
                )
            }))
            .when(active, |bar| {
                bar.child(
                    button("clear", "x", self.t(Text::Clear), theme)
                        .on_click(cx.listener(|this, _, _, cx| this.clear(cx))),
                )
            })
    }

    pub fn sort_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .child(
                toggle_button(
                    "sort",
                    "arrow-up-down",
                    self.labelled(
                        self.t(Text::Sort),
                        self.sort_label(self.settings.album_sort),
                    ),
                    self.menu == Some(Menu::Sort),
                    self.theme(),
                )
                .on_click(cx.listener(|this, _, _, cx| this.open_menu(Menu::Sort, cx))),
            )
            .when(self.menu == Some(Menu::Sort), |anchor| {
                anchor.child(dropdown(self.sort_menu(cx), true))
            })
    }

    fn sort_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        popover(theme)
            .w(rems(15.))
            .flex()
            .flex_col()
            .children(SORTS.iter().map(|&sort| {
                menu_item(
                    ("sort-option", sort as usize),
                    self.sort_label(sort),
                    self.settings.album_sort == sort,
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.menu = None;
                    this.set_sort(sort, cx);
                }))
            }))
            .into_any_element()
    }

    fn filters_menu(&self, kind: FilterKind, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        popover(theme)
            .w(rems(36.))
            .h(rems(24.))
            .flex()
            .gap(rems(0.25))
            .child(self.kinds_column(kind, cx))
            .child(div().w(px(1.)).h_full().bg(theme.border))
            .child(self.values_column(kind, cx))
            .into_any_element()
    }

    /// Transparent layer closing the open menu when clicking outside of it.
    pub fn menu_view(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.menu?;
        Some(
            div()
                .id("menu-backdrop")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .occlude()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.menu = None;
                        cx.notify();
                    }),
                )
                .into_any_element(),
        )
    }

    fn kinds_column(&self, selected: FilterKind, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .w(rems(12.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(rems(0.125))
            .children(FilterKind::ALL.into_iter().map(|kind| {
                let current = self.current(kind);
                let label = match &current {
                    Some(value) => self.labelled(self.t(kind.label()), value),
                    None => self.t(kind.label()).into(),
                };
                nav_item(
                    ("filter-kind", kind as usize),
                    if current.is_some() { "check" } else { "tag" },
                    label,
                    kind == selected,
                    theme,
                )
                .h(rems(2.))
                .min_w_0()
                .overflow_hidden()
                .on_click(
                    cx.listener(move |this, _, _, cx| this.open_menu(Menu::Filters(kind), cx)),
                )
            }))
    }

    fn values_column(&self, kind: FilterKind, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let query = fold(&self.menu_search.read(cx).content);
        let current = self.current(kind);
        let mut entries: Vec<(String, Option<Value>)> = vec![(self.t(Text::All).into(), None)];
        entries.extend(
            self.values(kind)
                .into_iter()
                .filter(|(label, _)| query.is_empty() || fold(label).contains(&query))
                .map(|(label, value)| (label, Some(value))),
        );
        let entries = Rc::new(entries);
        let searchable = matches!(kind, FilterKind::Genre | FilterKind::Artist);
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(rems(0.25))
            .when(searchable, |column| {
                column.child(
                    div()
                        .rounded(RADIUS)
                        .border_1()
                        .border_color(theme.control_border)
                        .bg(theme.background)
                        .px(rems(0.5))
                        .child(self.menu_search.clone()),
                )
            })
            .child(
                uniform_list(
                    "filter-values",
                    entries.len(),
                    cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|index| {
                                let (label, value) = entries[index].clone();
                                let selected = match &value {
                                    None => current.is_none(),
                                    Some(_) => current.as_deref() == Some(label.as_str()),
                                };
                                menu_item(("filter-value", index), label, selected, this.theme())
                                    .w_full()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.apply(kind, value.clone(), cx)
                                    }))
                            })
                            .collect()
                    }),
                )
                .track_scroll(self.menu_scroll.clone())
                .flex_1(),
            )
    }
}

/// Places a menu under its button, painted above the rest of the window.
fn dropdown(menu: AnyElement, align_right: bool) -> AnyElement {
    deferred(
        div()
            .absolute()
            .top_full()
            .mt(rems(0.25))
            .when(align_right, |anchor| anchor.right_0())
            .when(!align_right, |anchor| anchor.left_0())
            .child(menu),
    )
    .with_priority(1)
    .into_any_element()
}
