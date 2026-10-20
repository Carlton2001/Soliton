use crate::{
    app::{Rap, Section},
    i18n::Text,
    widgets::button,
};
use gpui::{prelude::*, *};

impl Rap {
    pub fn filter_menu_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let id = self.filter_menu.unwrap_or("genre");
        let values = match id {
            "genre" => self.facets.genres.clone(),
            "year" => self.facets.years.clone(),
            _ => self.facets.formats.clone(),
        };
        let theme = self.theme();
        let mut entries = vec![None];
        entries.extend(values.into_iter().map(Some));
        div()
            .absolute()
            .top(px(120.))
            .right(px(24.))
            .w(px(300.))
            .h(px(330.))
            .p_2()
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .shadow_lg()
            .flex()
            .flex_col()
            .child(
                button("close-filter", "x", self.t(Text::Back), theme).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.filter_menu = None;
                        cx.notify();
                    },
                )),
            )
            .child(
                uniform_list(
                    "filter-options",
                    entries.len(),
                    cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|index| {
                                let value = entries[index].clone();
                                button(
                                    ("filter-option", index),
                                    "tag",
                                    value.clone().unwrap_or_else(|| this.t(Text::All).into()),
                                    theme,
                                )
                                .w_full()
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        match id {
                                            "genre" => this.filter.genre = value.clone(),
                                            "year" => this.filter.year = value.clone(),
                                            _ => this.filter.format = value.clone(),
                                        }
                                        this.filter_menu = None;
                                        this.selected = None;
                                        this.section = Section::Albums;
                                        this.refresh(cx);
                                    },
                                ))
                            })
                            .collect()
                    }),
                )
                .flex_1(),
            )
    }
}
