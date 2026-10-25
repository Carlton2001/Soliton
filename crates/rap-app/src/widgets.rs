//! Shared controls. Every dimension is in rems so the interface follows the
//! interface size chosen in the settings.
use crate::theme::Theme;
use gpui::{prelude::*, *};
use std::path::PathBuf;
use std::rc::Rc;
mod adaptive;
pub use adaptive::{Density, set_density};

/// Type scale at the 16 px reference: body text is 13 px.
pub const TEXT_CAPTION: Rems = Rems(0.6875);
pub const TEXT_SMALL: Rems = Rems(0.75);
pub const TEXT_BODY: Rems = Rems(0.8125);
pub const TEXT_SUBTITLE: Rems = Rems(0.9375);
pub const TEXT_TITLE: Rems = Rems(1.25);
pub const TEXT_DISPLAY: Rems = Rems(1.75);

/// Height shared by buttons, fields and segmented controls so they align.
pub const CONTROL_HEIGHT: Rems = Rems(2.);
/// Horizontal inset shared by page headings, controls and content.
pub const PAGE_INSET: Rems = Rems(0.75);
/// Space above controls, below a heading or the window title bar.
pub const PAGE_TOP_PADDING: Rems = Rems(0.75);
/// One owner for the gap between a page toolbar and its content or sections.
pub const PAGE_SECTION_GAP: Rems = Rems(0.75);
/// Related actions within a toolbar stay closer than separate content sections.
pub const CONTROL_GAP: Rems = Rems(0.5);
pub const ROW_HEIGHT: Rems = Rems(2.5);
pub const RADIUS: Rems = Rems(0.3125);
pub const RADIUS_LARGE: Rems = Rems(0.5);
const ICON: Rems = Rems(1.);

/// Content below a page toolbar: symmetric outer insets and one gap between blocks.
pub fn page_body() -> Div {
    div()
        .flex_1()
        .min_h_0()
        .min_w_0()
        .flex()
        .flex_col()
        .py(PAGE_SECTION_GAP)
        .gap(PAGE_SECTION_GAP)
}

/// First row of page actions, with one owner for its surrounding space.
pub fn page_toolbar(theme: Theme) -> Div {
    div()
        .flex_none()
        .px(PAGE_INSET)
        .pt(PAGE_TOP_PADDING)
        .pb(PAGE_SECTION_GAP)
        .border_b_1()
        .border_color(theme.border)
        .flex()
        .items_center()
        .gap(CONTROL_GAP)
}

/// Single-line input frame sharing the exact outer height of toolbar buttons.
pub fn input_field(input: Entity<crate::input::TextInput>, search: bool, theme: Theme) -> Div {
    div()
        .h(CONTROL_HEIGHT)
        .text_size(TEXT_BODY)
        .font_weight(FontWeight::NORMAL)
        .min_h(CONTROL_HEIGHT)
        .max_h(CONTROL_HEIGHT)
        .min_w_0()
        .flex()
        .items_center()
        .gap(rems(0.5))
        .px(rems(0.625))
        .rounded(RADIUS)
        .border_1()
        .border_color(theme.control_border)
        .bg(theme.control)
        .text_color(theme.text)
        .when(search, |field| field.child(icon("search", theme)))
        .child(div().flex_1().min_w_0().child(input))
}

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
    styled_button(id, name, label, theme, false)
}

fn styled_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    theme: Theme,
    primary: bool,
) -> Stateful<Div> {
    let tooltip_theme = theme;
    let theme = if primary {
        Theme {
            text: theme.on_accent,
            control: theme.accent,
            control_border: theme.accent,
            ..theme
        }
    } else {
        theme
    };
    let label = label.into();
    let control = div()
        .id(id)
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| {
            if primary {
                style.bg(theme.control).border_color(theme.on_accent)
            } else {
                style.bg(theme.selected).border_color(theme.accent)
            }
        })
        .h(CONTROL_HEIGHT)
        .text_size(TEXT_BODY)
        .font_weight(FontWeight::MEDIUM)
        .flex()
        .flex_none()
        .min_w(rems(2.5))
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
        .hover(move |style| {
            if primary {
                style.opacity(0.9)
            } else {
                style.bg(theme.control_hover)
            }
        })
        .active(move |style| {
            if primary {
                style.opacity(0.8)
            } else {
                style.bg(theme.control_pressed)
            }
        })
        .child(adaptive::Content {
            icon: name,
            label: label.clone(),
            color: theme.text,
            primary,
        });
    with_tooltip(control, label, tooltip_theme)
}

/// Familiar playback command, named by a tooltip and optionally highlighted.
pub fn icon_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    theme: Theme,
) -> Stateful<Div> {
    with_tooltip(icon_control(id, name, active, theme), label.into(), theme)
}

/// [`icon_button`] filled with the accent, as the Play/Pause command. Its
/// tooltip reads `label` at each frame, so it follows the command while shown.
pub fn accent_icon_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Fn(&App) -> SharedString + 'static,
    theme: Theme,
) -> Stateful<Div> {
    let accent = Theme {
        text: theme.on_accent,
        accent: theme.on_accent,
        control_hover: theme.accent,
        control_pressed: theme.accent,
        selected: theme.accent,
        ..theme
    };
    // The tooltip keeps the regular colours: on_accent text is unreadable on a panel.
    with_live_tooltip(icon_control(id, name, true, accent), label, theme)
        .rounded_full()
        .border_1()
        .border_color(theme.accent)
}

fn icon_control(
    id: impl Into<ElementId>,
    name: &'static str,
    active: bool,
    theme: Theme,
) -> Stateful<Div> {
    let color = theme.text;
    div()
        .id(id)
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.bg(theme.selected).border_color(theme.accent))
        .size(CONTROL_HEIGHT)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(RADIUS)
        .when(active, |v| v.bg(theme.selected))
        .hover(move |style| style.bg(theme.control_hover))
        .active(move |style| style.bg(theme.control_pressed))
        .child(tinted_icon(name, color))
}

/// Main action of a page, filled with the accent.
pub fn accent_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    theme: Theme,
) -> Stateful<Div> {
    styled_button(id, name, label, theme, true)
}
/// [`button`] that stays highlighted while what it opens or enables is active.
pub fn toggle_button(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    theme: Theme,
) -> Stateful<Div> {
    button(id, name, label, theme).when(active, |control| control.bg(theme.selected))
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
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.bg(theme.selected).border_color(theme.accent))
        .h(CONTROL_HEIGHT)
        .text_size(TEXT_BODY)
        .font_weight(FontWeight::MEDIUM)
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
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .whitespace_normal()
                .line_clamp(1)
                .child(label.into()),
        )
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
    let label = label.into();
    let control = div()
        .id(id)
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.bg(theme.selected).border_color(theme.accent))
        .h(CONTROL_HEIGHT)
        .text_size(TEXT_BODY)
        .font_weight(FontWeight::MEDIUM)
        .flex()
        .flex_none()
        .min_w(rems(2.5))
        .items_center()
        .gap(rems(0.625))
        .px(rems(0.75))
        .rounded(RADIUS)
        .whitespace_nowrap()
        .hover(move |style| style.bg(theme.hover))
        .active(move |style| style.bg(theme.pressed))
        .child(adaptive::Content {
            icon: name,
            label: label.clone(),
            color: theme.muted,
            primary: false,
        });
    with_tooltip(control, label, theme)
}

/// Icon-only navigation entry of the section rail; its label is the tooltip.
pub fn nav_icon(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    let entry = div()
        .id(id)
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.bg(theme.selected).border_color(theme.accent))
        .size(rems(2.5))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(RADIUS)
        .active(move |style| style.bg(theme.pressed));
    let entry = if selected {
        entry.bg(theme.selected)
    } else {
        entry.hover(move |style| style.bg(theme.hover))
    };
    let color = if selected { theme.text } else { theme.muted };
    with_tooltip(entry.child(tinted_icon(name, color)), label.into(), theme)
}

/// Navigation entry; selection uses a tinted background and body foreground.
pub fn nav_item(
    id: impl Into<ElementId>,
    name: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    let label = label.into();
    let entry = div()
        .id(id)
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.bg(theme.selected).border_color(theme.accent))
        .h(rems(2.25))
        .font_weight(FontWeight::MEDIUM)
        .flex()
        .flex_none()
        .min_w(rems(2.5))
        .items_center()
        .gap(rems(0.625))
        .px(rems(0.75))
        .rounded(RADIUS)
        .active(move |style| style.bg(theme.pressed))
        .whitespace_nowrap();
    let color = if selected { theme.text } else { theme.muted };
    let entry = if selected {
        entry.bg(theme.selected).text_color(theme.text)
    } else {
        entry.hover(move |style| style.bg(theme.hover))
    };
    // Menu entries keep their label at every density.
    with_tooltip(
        entry
            .child(tinted_icon(name, color))
            .child(div().min_w_0().truncate().child(label.clone())),
        label,
        theme,
    )
}
/// Mutually exclusive options drawn as one bordered control.
pub fn segments(options: impl IntoIterator<Item = Stateful<Div>>, theme: Theme) -> Div {
    div()
        .h(CONTROL_HEIGHT)
        .text_size(TEXT_BODY)
        .font_weight(FontWeight::MEDIUM)
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
    let label = label.into();
    let color = if selected { theme.text } else { theme.muted };
    let option = div()
        .id(id)
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.bg(theme.selected).border_color(theme.accent))
        .h(rems(1.5))
        .flex()
        .items_center()
        .gap(rems(0.375))
        .px(rems(0.625))
        .rounded(rems(0.1875))
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .min_w_0();
    let option = if let Some(name) = name {
        let content = option.min_w(rems(2.25)).child(adaptive::Content {
            icon: name,
            label: label.clone(),
            color,
            primary: false,
        });
        // Icon-only steppers supply their meaningful tooltip at the call site.
        if label.is_empty() {
            content
        } else {
            with_tooltip(content, label, theme)
        }
    } else {
        option.child(label)
    };
    if selected {
        option
            .bg(theme.selected)
            .text_color(theme.text)
            .active(move |style| style.bg(theme.pressed))
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
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.bg(theme.selected).border_color(theme.accent))
        .hover(move |style| style.bg(theme.hover))
        .active(move |style| style.bg(theme.pressed))
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
    let color = if selected { theme.text } else { theme.muted };
    let row = div()
        .id(id)
        .tab_index(0)
        .capture_any_mouse_down(keyboard_focus_only)
        .border_1()
        .border_color(gpui::transparent_black())
        .focus(move |style| style.bg(theme.selected).border_color(theme.accent))
        .min_h(ROW_HEIGHT)
        .flex()
        .items_center()
        .gap(rems(0.75))
        .px(rems(0.75))
        .child(mark)
        .child(tinted_icon(name, color))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .whitespace_normal()
                .line_clamp(1)
                .child(label.into()),
        );
    if selected {
        row.bg(theme.selected)
            .active(move |style| style.bg(theme.pressed))
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
        .flex_wrap()
        .items_center()
        .gap(rems(0.75))
        .px(rems(0.75))
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

/// GPUI keeps the tooltip view while the pointer stays, so its text is read at render.
struct Tip(Rc<dyn Fn(&App) -> SharedString>, Theme);
impl Render for Tip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(rems(0.5))
            .py(rems(0.25))
            .rounded(RADIUS)
            .border_1()
            .border_color(self.1.control_border)
            .bg(self.1.panel)
            .text_color(self.1.text)
            .text_size(TEXT_SMALL)
            .child((self.0)(cx))
    }
}

pub fn with_tooltip(element: Stateful<Div>, label: SharedString, theme: Theme) -> Stateful<Div> {
    with_live_tooltip(element, move |_| label.clone(), theme)
}

pub fn with_live_tooltip(
    element: Stateful<Div>,
    label: impl Fn(&App) -> SharedString + 'static,
    theme: Theme,
) -> Stateful<Div> {
    let label: Rc<dyn Fn(&App) -> SharedString> = Rc::new(label);
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

/// Keep click feedback separate from the keyboard focus ring on actions.
pub fn keyboard_focus_only(event: &MouseDownEvent, window: &mut Window, _: &mut App) {
    if event.button == MouseButton::Left {
        window.prevent_default();
    }
}

#[cfg(test)]
#[path = "tests/widgets.rs"]
mod tests;
