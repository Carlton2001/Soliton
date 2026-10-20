use crate::theme::Theme;
use gpui::{prelude::*, *};
use std::path::PathBuf;

pub fn icon(name: &'static str, theme: Theme) -> Svg {
    tinted_icon(name, theme.muted)
}

pub fn tinted_icon(name: &'static str, color: Rgba) -> Svg {
    svg()
        .path(format!("icons/{name}.svg"))
        .size_4()
        .text_color(color)
}

pub fn button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_md()
        .hover(move |style| style.bg(theme.card))
        .child(icon(name, theme))
        .child(label.into())
}

/// One option of a set; the selected one is highlighted with the accent color.
pub fn choice(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    let label = label.into();
    if !selected {
        return button(id, name, label, theme);
    }
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(theme.card)
        .text_color(theme.accent)
        .child(tinted_icon(name, theme.accent))
        .child(label)
}

struct Tip(SharedString, Theme);
impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_2()
            .rounded_md()
            .bg(self.1.card)
            .text_color(self.1.text)
            .text_sm()
            .child(self.0.clone())
    }
}

pub fn with_tooltip(element: Stateful<Div>, label: SharedString, theme: Theme) -> Stateful<Div> {
    element.tooltip(move |_, cx| cx.new(|_| Tip(label.clone(), theme)).into())
}

pub fn icon_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Stateful<Div> {
    tool(id, name, label, false, theme)
}

/// Icon-only control with a tooltip; `active` tints it with the accent color.
pub fn tool(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    theme: Theme,
) -> Stateful<Div> {
    let color = if active { theme.accent } else { theme.muted };
    with_tooltip(
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .p_2()
            .rounded_md()
            .hover(move |style| style.bg(theme.card))
            .child(tinted_icon(name, color)),
        label.into(),
        theme,
    )
}

pub fn cover(path: Option<&PathBuf>, side: f32, theme: Theme) -> AnyElement {
    let frame = div()
        .size(px(side))
        .flex_shrink_0()
        .rounded_md()
        .overflow_hidden()
        .bg(theme.card)
        .flex()
        .items_center()
        .justify_center();
    if let Some(path) = path {
        let path = if side <= 170. {
            path.with_file_name(format!(
                "{}.small.png",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ))
        } else {
            path.clone()
        };
        frame
            .child(img(path).size_full().object_fit(ObjectFit::Contain))
            .into_any_element()
    } else {
        frame
            .child(
                svg()
                    .path("icons/disc-3.svg")
                    .size(px(side * 0.38))
                    .text_color(theme.muted),
            )
            .into_any_element()
    }
}

/// Path as users write it, without the extended-length prefix of canonical paths.
pub fn display_path(path: &std::path::Path) -> String {
    plain_paths(&path.display().to_string())
}

/// Removes extended-length prefixes from paths embedded in a text.
pub fn plain_paths(text: &str) -> String {
    text.replace(r"\\?\UNC\", r"\\").replace(r"\\?\", "")
}

pub fn time(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub fn quality(metadata: &rap_core::model::Metadata) -> String {
    let mut values = vec![metadata.format.clone()];
    if let Some(rate) = metadata.sample_rate {
        values.push(format!("{rate} Hz"));
    }
    if let Some(depth) = metadata.bit_depth {
        values.push(format!("{depth} bits"));
    }
    values.join(" · ")
}
