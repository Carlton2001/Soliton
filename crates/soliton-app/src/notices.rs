//! Errors and warnings collected while Soliton runs, shown above the playback bar.
use crate::{
    app::Soliton,
    i18n::Text,
    widgets::{button, plain_paths, tinted_icon},
};
use gpui::{prelude::*, *};
use soliton_core::failure::Failure;

/// Older messages are dropped beyond this count.
const KEPT: usize = 50;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Warning,
    Error,
}

pub struct Notice {
    pub level: Level,
    /// Where it happened: playback, library, an output…
    pub label: Text,
    pub text: String,
}

#[derive(Default)]
pub struct Notices {
    items: Vec<Notice>,
    pub expanded: bool,
}

impl Notices {
    pub fn push(&mut self, level: Level, label: Text, text: impl Into<String>) {
        let text = text.into();
        // Repeated messages, such as a failing action retried, are shown once.
        if self
            .items
            .last()
            .is_some_and(|last| last.label == label && last.text == text)
        {
            return;
        }
        self.items.push(Notice { level, label, text });
        if self.items.len() > KEPT {
            self.items.remove(0);
        }
    }
    pub fn clear(&mut self) {
        self.items.clear();
        self.expanded = false;
    }
}

impl Soliton {
    pub fn report_error(&mut self, text: impl Into<String>) {
        self.notices.push(Level::Error, Text::Error, text);
    }
    pub fn report_warning(&mut self, text: impl Into<String>) {
        self.notices.push(Level::Warning, Text::Warning, text);
    }
    /// Shows the failure in the interface language, under `label`.
    pub fn report_failure(&mut self, level: Level, label: Text, failure: &Failure) {
        let text = crate::i18n::describe(self.settings.language, failure);
        self.notices.push(level, label, text);
    }

    pub fn notices_view(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let latest = self.notices.items.last()?;
        let theme = self.theme();
        let count = self.notices.items.len();
        let line = |notice: &Notice| {
            let name = match notice.level {
                Level::Error => "circle-alert",
                Level::Warning => "triangle-alert",
            };
            let label = notice.label;
            div()
                .flex()
                .items_center()
                .gap(rems(0.5))
                .min_w_0()
                .child(tinted_icon(name, theme.accent))
                .child(div().flex_shrink_0().child(self.t(label)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child(plain_paths(&notice.text)),
                )
        };
        let expanded = self.notices.expanded && count > 1;
        Some(
            div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .bg(theme.panel)
                .border_t_1()
                .border_color(theme.border)
                .when(expanded, |view| {
                    view.child(
                        div()
                            .id("notices-list")
                            .max_h(rems(11.25))
                            .overflow_y_scroll()
                            .px(rems(0.75))
                            .pt(rems(0.5))
                            .flex()
                            .flex_col()
                            .gap(rems(0.25))
                            // Lines keep their height so the list scrolls instead.
                            .children(
                                self.notices
                                    .items
                                    .iter()
                                    .rev()
                                    .skip(1)
                                    .map(|notice| line(notice).flex_shrink_0()),
                            ),
                    )
                })
                .child(
                    div()
                        .px(rems(0.75))
                        .py(rems(0.375))
                        .flex()
                        .items_center()
                        .gap(rems(0.75))
                        .child(line(latest).flex_1())
                        .when(count > 1, |row| {
                            row.child(
                                button(
                                    "notices-toggle",
                                    "chevron-down",
                                    format!(
                                        "{} ({count})",
                                        self.t(if expanded { Text::Hide } else { Text::ShowAll })
                                    ),
                                    theme,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.notices.expanded = !this.notices.expanded;
                                        cx.notify();
                                    },
                                )),
                            )
                        })
                        .child(
                            button("notices-dismiss", "x", self.t(Text::DismissAll), theme)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.notices.clear();
                                    cx.notify();
                                })),
                        ),
                )
                .into_any_element(),
        )
    }
}
