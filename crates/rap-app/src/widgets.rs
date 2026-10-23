//! Shared controls. Every dimension is in rems so the interface follows the
//! interface size chosen in the settings.
use crate::theme::Theme;
use gpui::{prelude::*, *};
use std::path::PathBuf;

/// Type scale at the 16 px reference: body text is 13 px.
pub const TEXT_CAPTION: Rems = Rems(0.6875);
pub const TEXT_SMALL: Rems = Rems(0.75);
pub const TEXT_BODY: Rems = Rems(0.8125);
pub const TEXT_SUBTITLE: Rems = Rems(0.9375);
pub const TEXT_TITLE: Rems = Rems(1.25);
pub const TEXT_DISPLAY: Rems = Rems(1.75);

/// Height shared by buttons, fields and segmented controls so they align.
pub const CONTROL_HEIGHT: Rems = Rems(2.);
pub const ROW_HEIGHT: Rems = Rems(2.5);
pub const RADIUS: Rems = Rems(0.3125);
pub const RADIUS_LARGE: Rems = Rems(0.5);
const ICON: Rems = Rems(1.);

/// Inter's tabular digits align durations without changing the UI typeface.
pub fn numeric_font() -> Font {
    static FEATURES: std::sync::OnceLock<FontFeatures> = std::sync::OnceLock::new();
    Font {
        family: "Inter".into(),
        features: FEATURES
            .get_or_init(|| FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])))
            .clone(),
        ..gpui::font("Inter")
    }
}

pub fn icon(name: &'static str, theme: Theme) -> Svg {
    tinted_icon(name, theme.muted)
}

pub fn tinted_icon(name: &'static str, color: Rgba) -> Svg {
    svg()
        .path(format!("icons/{name}.svg"))
        .size(ICON)
        .flex_none()
        .text_color(color)
}

/// Bordered push button with an icon and a label.
pub fn button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(CONTROL_HEIGHT)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(rems(0.5))
        .px(rems(0.75))
        .rounded(RADIUS)
        .border_1()
        .border_color(theme.control_border)
        .bg(theme.control)
        .text_color(theme.text)
        .whitespace_nowrap()
        .hover(move |style| style.bg(theme.control_hover))
        .active(move |style| style.bg(theme.control_pressed))
        .child(tinted_icon(name, theme.text))
        .child(label.into())
}

/// Familiar playback command, named by a tooltip and optionally highlighted.
pub fn icon_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    theme: Theme,
) -> Stateful<Div> {
    let color = if active { theme.accent } else { theme.text };
    with_tooltip(
        div()
            .id(id)
            .size(CONTROL_HEIGHT)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .rounded(RADIUS)
            .when(active, |v| v.bg(theme.selected))
            .hover(move |style| style.bg(theme.control_hover))
            .active(move |style| style.bg(theme.control_pressed))
            .child(tinted_icon(name, color)),
        label.into(),
        theme,
    )
}

/// Main action of a page, filled with the accent.
pub fn accent_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(CONTROL_HEIGHT)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(rems(0.5))
        .px(rems(0.875))
        .rounded(RADIUS)
        .bg(theme.accent)
        .text_color(theme.on_accent)
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .hover(|style| style.opacity(0.9))
        .active(|style| style.opacity(0.8))
        .child(tinted_icon(name, theme.on_accent))
        .child(label.into())
}

/// [`button`] that stays highlighted while what it opens or enables is active.
pub fn toggle_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    theme: Theme,
) -> Stateful<Div> {
    if !active {
        return button(id, name, label, theme);
    }
    div()
        .id(id)
        .h(CONTROL_HEIGHT)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .gap(rems(0.5))
        .px(rems(0.75))
        .rounded(RADIUS)
        .border_1()
        .border_color(theme.accent)
        .bg(theme.selected)
        .text_color(theme.accent)
        .whitespace_nowrap()
        .active(move |style| style.bg(theme.pressed))
        .child(tinted_icon(name, theme.accent))
        .child(label.into())
}

/// Entry of a drop-down menu; the current one carries a check mark.
pub fn menu_item(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(CONTROL_HEIGHT)
        .flex()
        .flex_none()
        .items_center()
        .gap(rems(0.5))
        .px(rems(0.5))
        .rounded(RADIUS)
        .when(selected, |item| item.text_color(theme.accent))
        .hover(move |style| style.bg(theme.hover))
        .active(move |style| style.bg(theme.pressed))
        .child(div().size(rems(1.)).flex_none().when(selected, |mark| {
            mark.child(tinted_icon("check", theme.accent))
        }))
        .child(div().flex_1().min_w_0().truncate().child(label.into()))
}

/// Floating surface of menus and dialogs.
pub fn popover(theme: Theme) -> Div {
    div()
        .p(rems(0.25))
        .rounded(RADIUS_LARGE)
        .border_1()
        .border_color(theme.control_border)
        .bg(theme.panel)
        .shadow_lg()
        .occlude()
}

/// Borderless action for toolbars and the sidebar.
pub fn ghost(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .h(CONTROL_HEIGHT)
        .flex()
        .flex_none()
        .items_center()
        .gap(rems(0.625))
        .px(rems(0.75))
        .rounded(RADIUS)
        .whitespace_nowrap()
        .hover(move |style| style.bg(theme.hover))
        .active(move |style| style.bg(theme.pressed))
        .child(icon(name, theme))
        .child(label.into())
}

/// Navigation entry; the current one is tinted with the accent.
pub fn nav_item(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    let entry = div()
        .id(id)
        .h(rems(2.25))
        .flex()
        .flex_none()
        .items_center()
        .gap(rems(0.625))
        .px(rems(0.75))
        .rounded(RADIUS)
        .whitespace_nowrap();
    if selected {
        entry
            .bg(theme.selected)
            .text_color(theme.accent)
            .font_weight(FontWeight::MEDIUM)
            .child(tinted_icon(name, theme.accent))
            .child(label.into())
    } else {
        entry
            .hover(move |style| style.bg(theme.hover))
            .active(move |style| style.bg(theme.pressed))
            .child(icon(name, theme))
            .child(label.into())
    }
}

/// Mutually exclusive options drawn as one bordered control.
pub fn segments(options: impl IntoIterator<Item = Stateful<Div>>, theme: Theme) -> Div {
    div()
        .h(CONTROL_HEIGHT)
        .flex()
        .flex_none()
        .items_center()
        .gap(rems(0.125))
        .px(rems(0.1875))
        .rounded(RADIUS)
        .border_1()
        .border_color(theme.control_border)
        .bg(theme.background)
        .children(options)
}

/// One option of [`segments`], with an optional leading icon.
pub fn segment(
    id: impl Into<ElementId>,
    name: Option<&'static str>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    let color = if selected { theme.accent } else { theme.muted };
    let option = div()
        .id(id)
        .h(rems(1.5))
        .flex()
        .items_center()
        .gap(rems(0.375))
        .px(rems(0.625))
        .rounded(rems(0.1875))
        .whitespace_nowrap()
        .children(name.map(|name| tinted_icon(name, color)))
        .child(label.into());
    if selected {
        option
            .bg(theme.selected)
            .text_color(theme.accent)
            .font_weight(FontWeight::MEDIUM)
    } else {
        option
            .hover(move |style| style.bg(theme.hover))
            .active(move |style| style.bg(theme.pressed))
    }
}

/// Labelled check box; the whole row toggles it.
pub fn checkbox(
    id: impl Into<ElementId>,
    checked: bool,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Stateful<Div> {
    let mark = div()
        .size(rems(1.))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(rems(0.25))
        .border_1();
    let mark = if checked {
        mark.bg(theme.accent).border_color(theme.accent).child(
            svg()
                .path("icons/check.svg")
                .size(rems(0.75))
                .text_color(theme.on_accent),
        )
    } else {
        mark.bg(theme.control)
            .border_color(theme.control_border)
            .group_hover("checkbox", move |style| style.border_color(theme.muted))
    };
    div()
        .id(id)
        .group("checkbox")
        .min_h(CONTROL_HEIGHT)
        .flex()
        .items_center()
        .gap(rems(0.625))
        .child(mark)
        .child(label.into())
}

/// Row of a [`list_box`] choosing one item of a set, with a radio mark.
pub fn radio_row(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    let mark = div()
        .size(rems(1.))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1();
    let mark = if selected {
        mark.border_color(theme.accent)
            .child(div().size(rems(0.5)).rounded_full().bg(theme.accent))
    } else {
        mark.border_color(theme.control_border).bg(theme.control)
    };
    let color = if selected { theme.accent } else { theme.muted };
    let row = div()
        .id(id)
        .min_h(ROW_HEIGHT)
        .flex()
        .items_center()
        .gap(rems(0.75))
        .px(rems(0.875))
        .child(mark)
        .child(tinted_icon(name, color))
        .child(div().flex_1().min_w_0().truncate().child(label.into()));
    if selected {
        row.bg(theme.selected)
    } else {
        row.hover(move |style| style.bg(theme.hover))
            .active(move |style| style.bg(theme.pressed))
    }
}

/// Plain row of a [`list_box`].
pub fn list_row() -> Div {
    div()
        .min_h(ROW_HEIGHT)
        .flex()
        .items_center()
        .gap(rems(0.75))
        .px(rems(0.875))
        .py(rems(0.375))
}

/// Bordered container whose rows are separated by dividers.
pub fn list_box(rows: impl IntoIterator<Item = AnyElement>, theme: Theme) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .rounded(RADIUS_LARGE)
        .border_1()
        .border_color(theme.control_border)
        .bg(theme.panel)
        .overflow_hidden()
        .children(rows.into_iter().enumerate().map(move |(index, row)| {
            div()
                .w_full()
                .flex()
                .flex_col()
                .when(index > 0, |divided| {
                    divided.border_t_1().border_color(theme.border)
                })
                .child(row)
        }))
}

struct Tip(SharedString, Theme);
impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(rems(0.5))
            .py(rems(0.25))
            .rounded(RADIUS)
            .border_1()
            .border_color(self.1.control_border)
            .bg(self.1.panel)
            .text_color(self.1.text)
            .text_size(TEXT_SMALL)
            .child(self.0.clone())
    }
}

pub fn with_tooltip(element: Stateful<Div>, label: SharedString, theme: Theme) -> Stateful<Div> {
    element.tooltip(move |_, cx| cx.new(|_| Tip(label.clone(), theme)).into())
}

pub fn cover(path: Option<&PathBuf>, side: f32, theme: Theme) -> AnyElement {
    let frame = div()
        .size(rems(side / 16.))
        .flex_shrink_0()
        .rounded(RADIUS)
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
                    .size(rems(side * 0.38 / 16.))
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
