//! Horizontal slider for playback position and volume, dragged with the left button.
use crate::{scale::scaled, theme::Theme};
use gpui::*;
use std::{cell::Cell, rc::Rc};

const RAIL: f32 = 4.;
const KNOB: f32 = 12.;

/// Called with the pointer fraction and whether the drag just ended.
pub type OnChange = Rc<dyn Fn(f32, bool, &mut Window, &mut App)>;

#[derive(Clone, Default)]
pub struct Slider {
    dragging: Rc<Cell<bool>>,
}

impl Slider {
    /// `value` in 0..=1. The whole element height is the hit area; the rail is
    /// drawn in the middle. Geometry is read at paint time, like the scrollbar.
    pub fn render(
        &self,
        value: f32,
        enabled: bool,
        theme: Theme,
        on_change: OnChange,
    ) -> Canvas<Hitbox> {
        let dragging = self.dragging.clone();
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |bounds, hitbox, window, _| {
                let (rail_height, knob) = (scaled(RAIL), scaled(KNOB));
                let active = enabled && (dragging.get() || hitbox.is_hovered(window));
                let width = f32::from(bounds.size.width);
                let rail = Bounds::new(
                    point(bounds.origin.x, bounds.center().y - px(rail_height / 2.)),
                    size(bounds.size.width, px(rail_height)),
                );
                let mut background = theme.muted;
                background.a = 0.3;
                window.paint_quad(fill(rail, background).corner_radii(px(rail_height / 2.)));
                let filled = px(width * value.clamp(0., 1.));
                let fill_color = if enabled { theme.accent } else { theme.muted };
                window.paint_quad(
                    fill(
                        Bounds::new(rail.origin, size(filled, px(rail_height))),
                        fill_color,
                    )
                    .corner_radii(px(rail_height / 2.)),
                );
                if active {
                    let thumb = Bounds::new(
                        point(
                            bounds.origin.x + filled - px(knob / 2.),
                            bounds.center().y - px(knob / 2.),
                        ),
                        size(px(knob), px(knob)),
                    );
                    window.paint_quad(fill(thumb, theme.text).corner_radii(px(knob / 2.)));
                }
                if !enabled {
                    return;
                }
                let fraction =
                    move |x: Pixels| (f32::from(x - bounds.origin.x) / width.max(1.)).clamp(0., 1.);
                window.on_mouse_event({
                    let (dragging, on_change, hitbox) =
                        (dragging.clone(), on_change.clone(), hitbox.clone());
                    move |event: &MouseDownEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble
                            && event.button == MouseButton::Left
                            && hitbox.is_hovered(window)
                        {
                            dragging.set(true);
                            on_change(fraction(event.position.x), false, window, cx);
                            cx.stop_propagation();
                            window.refresh();
                        }
                    }
                });
                window.on_mouse_event({
                    let (dragging, on_change) = (dragging.clone(), on_change.clone());
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase != DispatchPhase::Capture || !dragging.get() {
                            return;
                        }
                        if event.pressed_button == Some(MouseButton::Left) {
                            on_change(fraction(event.position.x), false, window, cx);
                        } else {
                            // The button was released outside the window.
                            dragging.set(false);
                            on_change(fraction(event.position.x), true, window, cx);
                        }
                        window.refresh();
                    }
                });
                window.on_mouse_event({
                    let (dragging, on_change) = (dragging.clone(), on_change.clone());
                    move |event: &MouseUpEvent, phase, window, cx| {
                        if phase == DispatchPhase::Capture && dragging.replace(false) {
                            on_change(fraction(event.position.x), true, window, cx);
                            window.refresh();
                        }
                    }
                });
            },
        )
    }
}
