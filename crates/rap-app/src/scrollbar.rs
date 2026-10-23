//! Scrollbar overlays for pointers without a wheel.
use crate::theme::Theme;
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

const TRACK: f32 = 12.;
const THUMB: f32 = 6.;
const MIN_THUMB: f32 = 32.;

/// Scroll state plus the pointer offset inside the thumb while dragging.
#[derive(Clone, Default)]
pub struct Scrollbar {
    pub handle: ScrollHandle,
    grab: Rc<Cell<Option<f32>>>,
    horizontal_grab: Rc<Cell<Option<f32>>>,
}

impl Scrollbar {
    pub fn new(handle: ScrollHandle) -> Self {
        Self {
            handle,
            grab: Rc::default(),
            horizontal_grab: Rc::default(),
        }
    }

    /// Absolute overlay on the right edge of a `relative` parent. Geometry is read at
    /// paint time so wheel scrolling never leaves the thumb one frame behind.
    pub fn render(&self, theme: Theme) -> impl IntoElement {
        self.render_axis(theme, false)
    }

    pub fn render_horizontal(&self, theme: Theme) -> impl IntoElement {
        self.render_axis(theme, true)
    }

    fn render_axis(&self, theme: Theme, horizontal: bool) -> impl IntoElement {
        let handle = self.handle.clone();
        let grab = if horizontal {
            self.horizontal_grab.clone()
        } else {
            self.grab.clone()
        };
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |bounds, hitbox, window, _| {
                let Some(thumb) = thumb_bounds(&handle, bounds, horizontal) else {
                    grab.set(None);
                    return;
                };
                let dragging = grab.get().is_some();
                let mut track = theme.card;
                track.a = 0.6;
                let mut color = theme.muted;
                color.a = if dragging || hitbox.is_hovered(window) {
                    0.8
                } else {
                    0.45
                };
                window.paint_quad(fill(bounds, track));
                let inset = crate::scale::scaled((TRACK - THUMB) / 2.);
                window.paint_quad(
                    fill(
                        Bounds::new(
                            if horizontal {
                                point(thumb.origin.x, bounds.origin.y + px(inset))
                            } else {
                                point(bounds.origin.x + px(inset), thumb.origin.y)
                            },
                            if horizontal {
                                size(thumb.size.width, crate::scale::scaled_px(THUMB))
                            } else {
                                size(crate::scale::scaled_px(THUMB), thumb.size.height)
                            },
                        ),
                        color,
                    )
                    .corner_radii(crate::scale::scaled_px(THUMB / 2.)),
                );
                window.on_mouse_event({
                    let (handle, grab, hitbox) = (handle.clone(), grab.clone(), hitbox.clone());
                    move |event: &MouseDownEvent, phase, window, cx| {
                        if phase != DispatchPhase::Capture
                            || event.button != MouseButton::Left
                            || !hitbox.is_hovered(window)
                        {
                            return;
                        }
                        let Some(thumb) = thumb_bounds(&handle, bounds, horizontal) else {
                            return;
                        };
                        // Clicking the rail centers the thumb under the pointer.
                        let offset = if thumb.contains(&event.position) {
                            if horizontal {
                                f32::from(event.position.x - thumb.origin.x)
                            } else {
                                f32::from(event.position.y - thumb.origin.y)
                            }
                        } else {
                            if horizontal {
                                f32::from(thumb.size.width) / 2.
                            } else {
                                f32::from(thumb.size.height) / 2.
                            }
                        };
                        grab.set(Some(offset));
                        scroll_to(&handle, bounds, event.position, offset, horizontal);
                        cx.stop_propagation();
                        window.refresh();
                    }
                });
                window.on_mouse_event({
                    let (handle, grab) = (handle.clone(), grab.clone());
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        let Some(offset) = grab.get() else { return };
                        if phase != DispatchPhase::Capture {
                            return;
                        }
                        if event.pressed_button != Some(MouseButton::Left) {
                            grab.set(None);
                        } else {
                            scroll_to(&handle, bounds, event.position, offset, horizontal);
                            cx.stop_propagation();
                        }
                        window.refresh();
                    }
                });
                window.on_mouse_event({
                    let grab = grab.clone();
                    move |_: &MouseUpEvent, phase, window, _| {
                        if phase == DispatchPhase::Capture && grab.take().is_some() {
                            window.refresh();
                        }
                    }
                });
            },
        )
        .absolute()
        .when(horizontal, |rail| {
            rail.left_0().right_0().bottom_0().h(rems(TRACK / 16.))
        })
        .when(!horizontal, |rail| {
            rail.top_0().right_0().bottom_0().w(rems(TRACK / 16.))
        })
    }
}

fn thumb_geometry(viewport: f32, maximum: f32, offset: f32) -> Option<(f32, f32)> {
    if maximum <= 0.5 || viewport <= 0. {
        return None;
    }
    let length = (viewport * viewport / (viewport + maximum))
        .clamp(crate::scale::scaled(MIN_THUMB).min(viewport), viewport);
    let position = (-offset / maximum).clamp(0., 1.) * (viewport - length);
    Some((position, length))
}

fn thumb_bounds(
    handle: &ScrollHandle,
    track: Bounds<Pixels>,
    horizontal: bool,
) -> Option<Bounds<Pixels>> {
    let (viewport, maximum, offset) = if horizontal {
        (
            track.size.width,
            handle.max_offset().width,
            handle.offset().x,
        )
    } else {
        (
            track.size.height,
            handle.max_offset().height,
            handle.offset().y,
        )
    };
    let (position, length) =
        thumb_geometry(f32::from(viewport), f32::from(maximum), f32::from(offset))?;
    Some(if horizontal {
        Bounds::new(
            point(track.origin.x + px(position), track.origin.y),
            size(px(length), track.size.height),
        )
    } else {
        Bounds::new(
            point(track.origin.x, track.origin.y + px(position)),
            size(track.size.width, px(length)),
        )
    })
}

fn scroll_to(
    handle: &ScrollHandle,
    track: Bounds<Pixels>,
    pointer: Point<Pixels>,
    grab: f32,
    horizontal: bool,
) {
    let Some(thumb) = thumb_bounds(handle, track, horizontal) else {
        return;
    };
    let (free, maximum, position) = if horizontal {
        (
            track.size.width - thumb.size.width,
            handle.max_offset().width,
            pointer.x - track.origin.x,
        )
    } else {
        (
            track.size.height - thumb.size.height,
            handle.max_offset().height,
            pointer.y - track.origin.y,
        )
    };
    let free = f32::from(free);
    let ratio = if free > 0. {
        (f32::from(position) - grab) / free
    } else {
        0.
    };
    let value = px(-ratio.clamp(0., 1.) * f32::from(maximum));
    let offset = handle.offset();
    handle.set_offset(if horizontal {
        point(value, offset.y)
    } else {
        point(offset.x, value)
    });
}

#[cfg(test)]
#[path = "tests/scrollbar.rs"]
mod tests;
