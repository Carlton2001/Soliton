//! Vertical scrollbar overlay for pointers without a wheel.
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
}

impl Scrollbar {
    pub fn new(handle: ScrollHandle) -> Self {
        Self {
            handle,
            grab: Rc::default(),
        }
    }

    /// Absolute overlay on the right edge of a `relative` parent. Geometry is read at
    /// paint time so wheel scrolling never leaves the thumb one frame behind.
    pub fn render(&self, theme: Theme) -> impl IntoElement {
        let handle = self.handle.clone();
        let grab = self.grab.clone();
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |bounds, hitbox, window, _| {
                let Some(thumb) = thumb_bounds(&handle, bounds) else {
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
                let inset = (TRACK - THUMB) / 2.;
                window.paint_quad(
                    fill(
                        Bounds::new(
                            point(bounds.origin.x + px(inset), thumb.origin.y),
                            size(px(THUMB), thumb.size.height),
                        ),
                        color,
                    )
                    .corner_radii(px(THUMB / 2.)),
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
                        let Some(thumb) = thumb_bounds(&handle, bounds) else {
                            return;
                        };
                        // Clicking the rail centers the thumb under the pointer.
                        let offset = if thumb.contains(&event.position) {
                            f32::from(event.position.y - thumb.origin.y)
                        } else {
                            f32::from(thumb.size.height) / 2.
                        };
                        grab.set(Some(offset));
                        scroll_to(&handle, bounds, event.position.y, offset);
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
                            scroll_to(&handle, bounds, event.position.y, offset);
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
        .top_0()
        .right_0()
        .bottom_0()
        .w(px(TRACK))
    }
}

fn thumb_bounds(handle: &ScrollHandle, track: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
    let (height, max) = (
        f32::from(track.size.height),
        f32::from(handle.max_offset().height),
    );
    if max <= 0.5 || height <= 0. {
        return None;
    }
    let length = (height * height / (height + max)).clamp(MIN_THUMB.min(height), height);
    let ratio = (-f32::from(handle.offset().y) / max).clamp(0., 1.);
    Some(Bounds::new(
        point(
            track.origin.x,
            track.origin.y + px(ratio * (height - length)),
        ),
        size(track.size.width, px(length)),
    ))
}

fn scroll_to(handle: &ScrollHandle, track: Bounds<Pixels>, y: Pixels, grab: f32) {
    let Some(thumb) = thumb_bounds(handle, track) else {
        return;
    };
    let free = f32::from(track.size.height - thumb.size.height);
    let max = f32::from(handle.max_offset().height);
    let ratio = if free > 0. {
        (f32::from(y - track.origin.y) - grab) / free
    } else {
        0.
    };
    let offset = handle.offset();
    handle.set_offset(point(offset.x, px(-ratio.clamp(0., 1.) * max)));
}
