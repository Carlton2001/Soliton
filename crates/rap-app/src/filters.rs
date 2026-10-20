//! Combinable filters: one discreet button, removable chips for active filters,
//! and a menu listing the values of each kind. Also the album sort menu.
use crate::{
    app::{Rap, Section},
    catalog::{Period, Quality},
    i18n::{Text, decade},
    widgets::{button, choice, tinted_icon},
};
use gpui::{prelude::*, *};
use rap_core::{config::AlbumSort, text::fold};
use std::rc::Rc;

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

    fn values(&self, kind: FilterKind) -> Vec<(String, Value)> {
        let facets = &self.facets;
        let texts = |values: &[String]| {
            values
                .iter()
                .map(|v| (v.clone(), Value::Text(v.clone())))
                .collect()
        };
        match kind {
            FilterKind::Genre => texts(&facets.genres),
            FilterKind::Artist => texts(&facets.artists),
            FilterKind::Format => texts(&facets.formats),
            FilterKind::Quality => Quality::ALL
                .iter()
                .map(|&q| (self.quality_label(q).into(), Value::Quality(q)))
                .collect(),
            FilterKind::Period => facets
                .decades
                .iter()
                .map(|&d| Period::Decade(d))
                .chain(facets.years.iter().map(|&y| Period::Year(y)))
                .map(|p| (self.period_label(p), Value::Period(p)))
                .collect(),
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
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .items_center()
            .child(
                choice(
                    "filters",
                    "sliders-horizontal",
                    self.t(Text::Filters),
                    matches!(self.menu, Some(Menu::Filters(_))),
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
            .children(FilterKind::ALL.into_iter().filter_map(|kind| {
                let value = self.current(kind)?;
                Some(
                    div()
                        .id(("chip", kind as usize))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_1()
                        .rounded_full()
                        .border_1()
                        .border_color(theme.accent)
                        .text_color(theme.accent)
                        .hover(move |s| s.bg(theme.card))
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
        choice(
            "sort",
            "arrow-up-down",
            self.labelled(
                self.t(Text::Sort),
                self.sort_label(self.settings.album_sort),
            ),
            self.menu == Some(Menu::Sort),
            self.theme(),
        )
        .on_click(cx.listener(|this, _, _, cx| this.open_menu(Menu::Sort, cx)))
    }

    /// The open menu above a backdrop that closes it.
    pub fn menu_view(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.menu?;
        let theme = self.theme();
        let panel = div()
            .absolute()
            .p_2()
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .shadow_lg()
            .occlude()
            .flex();
        let panel = match menu {
            Menu::Sort => panel
                .top(px(72.))
                .right(px(160.))
                .w(px(240.))
                .flex_col()
                .children(SORTS.iter().map(|&sort| {
                    choice(
                        ("sort-option", sort as usize),
                        "arrow-up-down",
                        self.sort_label(sort),
                        self.settings.album_sort == sort,
                        theme,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.menu = None;
                        this.set_sort(sort, cx);
                    }))
                }))
                .into_any_element(),
            Menu::Filters(kind) => panel
                .top(px(132.))
                .left(px(225.))
                .w(px(560.))
                .h(px(380.))
                .gap_2()
                .child(self.kinds_column(kind, cx))
                .child(self.values_column(kind, cx))
                .into_any_element(),
        };
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(
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
                        ),
                )
                .child(panel)
                .into_any_element(),
        )
    }

    fn kinds_column(&self, selected: FilterKind, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .w(px(190.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .children(FilterKind::ALL.into_iter().map(|kind| {
                let current = self.current(kind);
                let label = match &current {
                    Some(value) => self.labelled(self.t(kind.label()), value),
                    None => self.t(kind.label()).into(),
                };
                choice(
                    ("filter-kind", kind as usize),
                    if current.is_some() { "check" } else { "tag" },
                    label,
                    kind == selected,
                    theme,
                )
                .min_w_0()
                .truncate()
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
            .gap_2()
            .when(searchable, |column| {
                column.child(
                    div()
                        .bg(theme.background)
                        .rounded_md()
                        .px_3()
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
                                choice(
                                    ("filter-value", index),
                                    if selected { "check" } else { "tag" },
                                    label,
                                    selected,
                                    this.theme(),
                                )
                                .w_full()
                                .on_click(cx.listener(
                                    move |this, _, _, cx| this.apply(kind, value.clone(), cx),
                                ))
                            })
                            .collect()
                    }),
                )
                .track_scroll(self.menu_scroll.clone())
                .flex_1(),
            )
    }
}
