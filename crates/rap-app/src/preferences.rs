//! Settings page: language, appearance, display preferences and source folders.
use crate::{
    app::Rap,
    i18n::Text,
    theme::Theme,
    views::scroll_area,
    widgets::{button, choice, display_path, with_tooltip},
};
use gpui::{prelude::*, *};
use rap_core::config::{Accent, Appearance, CoverSize, Language};

impl Rap {
    fn accent_name(&self, accent: Accent) -> &'static str {
        self.t(match accent {
            Accent::Teal => Text::Teal,
            Accent::Blue => Text::Blue,
            Accent::Indigo => Text::Indigo,
            Accent::Violet => Text::Violet,
            Accent::Pink => Text::Pink,
            Accent::Red => Text::Red,
            Accent::Orange => Text::Orange,
            Accent::Green => Text::Green,
        })
    }

    pub fn settings_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let s = &self.settings;
        let row = |label: Text| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(div().w(px(200.)).child(self.t(label)))
        };
        let languages = [
            (Language::French, Text::French),
            (Language::English, Text::English),
        ];
        let appearances = [
            (Appearance::System, Text::System),
            (Appearance::Light, Text::Light),
            (Appearance::Dark, Text::Dark),
        ];
        let sizes = [
            (CoverSize::Small, Text::Small),
            (CoverSize::Medium, Text::Medium),
            (CoverSize::Large, Text::Large),
        ];
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
                row(Text::Language).children(languages.map(|(language, label)| {
                    choice(
                        ("language", language as usize),
                        "users",
                        self.t(label),
                        s.language == language,
                        theme,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.set_language(language, cx)))
                })),
            )
            .child(self.audio_preferences(cx))
            .child(div().text_lg().child(self.t(Text::Display)))
            .child(
                row(Text::Theme).children(appearances.map(|(appearance, label)| {
                    choice(
                        ("appearance", appearance as usize),
                        "disc-3",
                        self.t(label),
                        s.appearance == appearance,
                        theme,
                    )
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.set_appearance(appearance, cx)),
                    )
                })),
            )
            .child(
                row(Text::AccentColor).children(
                    Accent::ALL.map(|accent| self.swatch(accent, s.accent == accent, cx)),
                ),
            )
            .child(row(Text::CoverSize).children(sizes.map(|(size, label)| {
                choice(
                    ("cover-size", size as usize),
                    "layout-grid",
                    self.t(label),
                    s.cover_size == size,
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_cover_size(size, cx)))
            })))
            .child(
                choice(
                    "hide-unavailable",
                    if s.hide_unavailable {
                        "check"
                    } else {
                        "square"
                    },
                    self.t(Text::HideUnavailable),
                    s.hide_unavailable,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_hide_unavailable(!this.settings.hide_unavailable, cx)
                })),
            )
            .child(self.system_preferences(cx))
            .child(div().text_lg().child(self.t(Text::Sources)))
            .children(self.sources_rows(theme, cx))
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

    fn swatch(&self, accent: Accent, selected: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = self.theme();
        let color = Theme::new(self.dark(), accent).accent;
        with_tooltip(
            div()
                .id(("accent", accent as usize))
                .size(px(32.))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .border_2()
                .border_color(if selected {
                    theme.text
                } else {
                    theme.background
                })
                .child(div().size(px(22.)).rounded_full().bg(color))
                .on_click(cx.listener(move |this, _, _, cx| this.set_accent(accent, cx))),
            self.accent_name(accent).into(),
            theme,
        )
    }

    fn sources_rows(&self, theme: Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        self.snapshot
            .sources
            .iter()
            .map(|source| {
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
                            .child(display_path(&source.path))
                            .when(!source.available, |v| {
                                v.text_color(theme.muted)
                                    .child(format!(" — {}", self.t(Text::Unavailable)))
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
                    .into_any_element()
            })
            .collect()
    }
}
