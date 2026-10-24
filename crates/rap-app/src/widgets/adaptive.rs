//! Button content measured against its actual allocation, including in panels.
use gpui::{prelude::*, *};

pub(super) struct Content {
    pub icon: &'static str,
    pub label: SharedString,
    pub color: Rgba,
}

impl IntoElement for Content {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Content {
    type RequestLayoutState = Pixels;
    type PrepaintState = AnyElement;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, Pixels) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let run = TextRun {
            len: self.label.len(),
            font: style.font(),
            color: self.color.into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let text = window
            .text_system()
            .shape_line(self.label.clone(), font_size, &[run], None);
        let icon = window.rem_size();
        let full = icon
            + if self.label.is_empty() {
                px(0.)
            } else {
                icon * 0.5 + text.width
            };
        let height = window.line_height().max(icon);
        let mut layout = Style {
            flex_shrink: 1.,
            flex_grow: 1.,
            ..Default::default()
        };
        layout.min_size.width = px(0.).into();
        let id = window.request_measured_layout(layout, move |known, available, _, _| {
            let width = known.width.unwrap_or_else(|| match available.width {
                AvailableSpace::Definite(width) => full.min(width).max(icon),
                AvailableSpace::MinContent => icon,
                AvailableSpace::MaxContent => full,
            });
            size(width, height)
        });
        (id, full)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        full: &mut Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let show_label = bounds.size.width + px(2.) >= *full;
        let mut child = div()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(rems(0.5))
            .whitespace_nowrap()
            .child(super::tinted_icon(self.icon, self.color))
            .when(show_label && !self.label.is_empty(), |view| {
                view.child(self.label.clone())
            })
            .into_any_element();
        child.layout_as_root(
            size(
                AvailableSpace::Definite(bounds.size.width),
                AvailableSpace::Definite(bounds.size.height),
            ),
            window,
            cx,
        );
        child.prepaint_at(bounds.origin, window, cx);
        child
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Pixels,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        child.paint(window, cx);
    }
}
