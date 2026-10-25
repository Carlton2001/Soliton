//! Settings page: titled sections of bordered cards holding label/control rows.
use crate::{
    app::Rap,
    i18n::Text,
    theme::Theme,
    views::scroll_area,
    widgets::{
        TEXT_SMALL, TEXT_SUBTITLE, button, checkbox, display_path, list_box, list_row, segment,
        segments, tinted_icon, with_tooltip,
    },
};
use gpui::{prelude::*, *};
use rap_core::config::{Accent, Appearance, CoverSize, Language, Settings};

/// Section title with an optional action aligned on the right, above its content.
pub fn section(
    title: impl IntoElement,
    action: Option<AnyElement>,
    body: impl IntoElement,
) -> AnyElement {
    div()
        .w_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(rems(0.625))
        .child(
            div()
                .min_h(rems(2.))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(rems(0.75))
                .child(
                    div()
                        .text_size(TEXT_SUBTITLE)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .children(action),
        )
        .child(div().w_full().flex().flex_col().child(body))
        .into_any_element()
}

/// Label, optional explanation and the control acting on it, inside a card.
pub fn setting_row(
    label: &'static str,
    hint: Option<&'static str>,
    control: impl IntoElement,
    theme: Theme,
) -> AnyElement {
    div()
        .min_h(rems(3.25))
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(rems(0.75))
        .px(crate::widgets::PAGE_INSET)
        .py(rems(0.625))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(rems(0.125))
                .flex_basis(rems(14.))
                .flex_grow()
                .min_w_0()
                .child(label)
                .children(hint.map(|hint| {
                    div()
                        .text_size(TEXT_SMALL)
                        .text_color(theme.muted)
                        .child(hint)
                })),
        )
        .child(control)
        .into_any_element()
}

/// Secondary explanation shown under a card.
pub fn hint(text: impl Into<SharedString>, theme: Theme) -> Div {
    div()
        .w_full()
        .px(rems(0.25))
        .text_size(TEXT_SMALL)
        .text_color(theme.muted)
        .child(text.into())
}

/// Status or problem that needs the user's attention, under a card.
pub fn notice(text: impl Into<SharedString>, theme: Theme) -> Div {
    hint(text, theme).text_color(theme.accent)
}

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
            Accent::System => Text::System,
        })
    }

    pub fn settings_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let content = div()
            .id("settings-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.settings_bar.handle)
            .flex()
            .flex_col()
            .child(self.page_title(Text::Settings, cx))
            .child(
                // Resolve the available width before measuring wrapped rows, and
                // preserve every section's height in the scrollable content.
                div()
                    .w_full()
                    .flex_shrink_0()
                    .max_w(rems(52.))
                    .px(crate::widgets::PAGE_INSET)
                    .pt(crate::widgets::PAGE_TOP_PADDING)
                    .pb(crate::widgets::PAGE_SECTION_GAP)
                    .flex()
                    .flex_col()
                    .gap(crate::widgets::PAGE_SECTION_GAP)
                    .child(self.general_section(theme, cx))
                    .child(self.display_section(theme, cx))
                    .child(self.audio_preferences(cx))
                    .children(self.bluetooth_preferences(cx))
                    .child(self.network_preferences(cx))
                    .children(self.system_preferences(cx))
                    .child(self.sources_section(theme, cx))
                    .child(self.update_preferences(cx)),
            );
        scroll_area(content, &self.settings_bar, theme)
    }

    fn general_section(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let languages = [
            (Language::French, Text::French),
            (Language::English, Text::English),
        ];
        let language = segments(
            languages.map(|(language, label)| {
                segment(
                    ("language", language as usize),
                    None,
                    self.t(label),
                    self.settings.language == language,
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_language(language, cx)))
            }),
            theme,
        );
        let size = self.settings.ui_rem_size;
        let stepper = div()
            .flex()
            .items_center()
            .gap(rems(0.5))
            .child(segments(
                [
                    with_tooltip(
                        segment("ui-smaller", Some("minus"), "", false, theme)
                            .gap(rems(0.))
                            .when(size <= Settings::MIN_REM_SIZE, |tool| tool.opacity(0.4))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_ui_rem_size(size.saturating_sub(1), cx)
                            })),
                        self.t(Text::Smaller).into(),
                        theme,
                    ),
                    div()
                        .id("ui-size-value")
                        .w(rems(3.5))
                        .text_center()
                        .font(crate::widgets::numeric_font())
                        .child(format!("{} %", size * 100 / Settings::DEFAULT_REM_SIZE)),
                    with_tooltip(
                        segment("ui-larger", Some("plus"), "", false, theme)
                            .gap(rems(0.))
                            .when(size >= Settings::MAX_REM_SIZE, |tool| tool.opacity(0.4))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.set_ui_rem_size(size + 1, cx)
                            })),
                        self.t(Text::Larger).into(),
                        theme,
                    ),
                ],
                theme,
            ))
            .child(
                button(
                    "ui-default-size",
                    "refresh-cw",
                    self.t(Text::DefaultSize),
                    theme,
                )
                .when(size == Settings::DEFAULT_REM_SIZE, |button| {
                    button.opacity(0.5)
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_ui_rem_size(Settings::DEFAULT_REM_SIZE, cx)
                })),
            );
        section(
            self.t(Text::General),
            None,
            list_box(
                [
                    setting_row(self.t(Text::Language), None, language, theme),
                    setting_row(
                        self.t(Text::InterfaceSize),
                        Some(self.t(Text::InterfaceSizeHint)),
                        stepper,
                        theme,
                    ),
                ],
                theme,
            ),
        )
    }

    fn display_section(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let s = &self.settings;
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
        let appearance = segments(
            appearances.map(|(appearance, label)| {
                segment(
                    ("appearance", appearance as usize),
                    None,
                    self.t(label),
                    s.appearance == appearance,
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_appearance(appearance, cx)))
            }),
            theme,
        );
        let system_accent = segment(
            "accent-system",
            None,
            self.t(Text::System),
            s.accent == Accent::System,
            theme,
        )
        .on_click(cx.listener(|this, _, _, cx| this.set_accent(Accent::System, cx)));
        let accents = segments(
            std::iter::once(system_accent).chain(
                Accent::ALL
                    .into_iter()
                    .filter(|accent| *accent != Accent::System)
                    .map(|accent| self.swatch(accent, s.accent == accent, cx)),
            ),
            theme,
        );
        let cover_size = segments(
            sizes.map(|(size, label)| {
                segment(
                    ("cover-size", size as usize),
                    None,
                    self.t(label),
                    s.cover_size == size,
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_cover_size(size, cx)))
            }),
            theme,
        );
        let hide = div().px(crate::widgets::PAGE_INSET).py(rems(0.625)).child(
            checkbox(
                "hide-unavailable",
                s.hide_unavailable,
                self.t(Text::HideUnavailable),
                theme,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.set_hide_unavailable(!this.settings.hide_unavailable, cx)
            })),
        );
        section(
            self.t(Text::Display),
            None,
            list_box(
                [
                    setting_row(self.t(Text::Theme), None, appearance, theme),
                    setting_row(self.t(Text::AccentColor), None, accents, theme),
                    self.icon_preferences(cx),
                    setting_row(self.t(Text::CoverSize), None, cover_size, theme),
                    hide.into_any_element(),
                ],
                theme,
            ),
        )
    }

    fn swatch(&self, accent: Accent, selected: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = self.theme();
        let color = crate::theme::accent_swatch(accent);
        with_tooltip(
            segment(("accent", accent as usize), None, "", selected, theme)
                .px(rems(0.375))
                .gap_0()
                .justify_center()
                .child(div().size(rems(1.)).rounded_full().bg(color))
                .on_click(cx.listener(move |this, _, _, cx| this.set_accent(accent, cx))),
            self.accent_name(accent).into(),
            theme,
        )
    }

    fn sources_section(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let add = button(
            "settings-add",
            "folder-plus",
            self.t(Text::AddSource),
            theme,
        )
        .when(self.scanning, |button| button.opacity(0.5))
        .on_click(cx.listener(|this, _, _, cx| {
            if !this.scanning {
                this.choose_source(cx)
            }
        }));
        let rescan = button("settings-rescan", "refresh-cw", self.t(Text::Rescan), theme)
            .when(self.scanning, |button| button.opacity(0.5))
            .on_click(cx.listener(|this, _, _, cx| {
                if !this.scanning {
                    this.scan(Vec::new(), cx)
                }
            }));
        let actions = div()
            .flex()
            .flex_wrap()
            .gap(rems(0.5))
            .child(rescan)
            .child(add);
        let rows: Vec<AnyElement> = if self.snapshot.sources.is_empty() {
            vec![
                list_row()
                    .text_color(theme.muted)
                    .child(self.t(Text::EmptyHint))
                    .into_any_element(),
            ]
        } else {
            self.sources_rows(theme, cx)
        };
        let title = div()
            .flex()
            .items_baseline()
            .gap(rems(0.75))
            .child(self.t(Text::Sources))
            .child(
                div()
                    .text_size(TEXT_SMALL)
                    .font_weight(FontWeight::NORMAL)
                    .text_color(theme.muted)
                    .child(crate::i18n::album_count(self.snapshot.albums.len())),
            );
        section(
            title,
            Some(actions.into_any_element()),
            list_box(rows, theme),
        )
    }

    fn sources_rows(&self, theme: Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        self.snapshot
            .sources
            .iter()
            .map(|source| {
                let id = source.id;
                let confirming = self.pending_removal == Some(id);
                list_row()
                    .child(tinted_icon(
                        "folder-plus",
                        if source.available {
                            theme.muted
                        } else {
                            theme.accent
                        },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .truncate()
                                    .whitespace_normal()
                                    .line_clamp(1)
                                    .child(display_path(&source.path)),
                            )
                            .when(!source.available, |v| {
                                v.child(
                                    div()
                                        .text_size(TEXT_SMALL)
                                        .text_color(theme.accent)
                                        .child(self.t(Text::Unavailable)),
                                )
                            }),
                    )
                    .when(confirming, |row| {
                        row.child(
                            div()
                                .text_size(TEXT_SMALL)
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
                            .text_color(theme.danger)
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
                            .when(self.scanning, |button| button.opacity(0.5))
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
