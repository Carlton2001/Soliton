//! Linux client decorations. Windows keeps GPUI's native non-client handling.
use crate::{placement, theme::Theme};
use gpui::{prelude::*, *};

/// Transparent margin holding the shadow and the resize zones.
const RESIZE_BORDER: Pixels = px(10.);
/// Radius of the window outline; content inside the 1px border uses one less.
const CORNER_RADIUS: Pixels = px(10.);

pub fn options(bounds: WindowBounds) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(bounds),
        window_min_size: Some(placement::MIN_SIZE),
        titlebar: Some(TitlebarOptions {
            appears_transparent: true,
            title: Some("Soliton".into()),
            ..Default::default()
        }),
        app_id: cfg!(target_os = "linux").then(|| soliton_platform::APP_ID.into()),
        window_decorations: cfg!(target_os = "linux").then_some(WindowDecorations::Client),
        ..Default::default()
    }
}

/// GPUI's Linux backends open `WindowBounds::Maximized` windows unmaximized;
/// Windows restores the state natively.
pub fn restore_maximized(bounds: WindowBounds, window: &mut Window) {
    if cfg!(target_os = "linux")
        && matches!(bounds, WindowBounds::Maximized(_))
        && !window.is_maximized()
    {
        window.zoom_window();
    }
}

/// Reused by the title bar and brand, preserving their native Windows hitboxes.
pub fn drag_region(id: &'static str, window: &mut Window, cx: &mut App) -> Stateful<Div> {
    let region = div()
        .id(id)
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Middle, |_, window, _| window.minimize_window());
    if !cfg!(target_os = "linux") {
        return region;
    }
    // Moving on press hands the pointer to the compositor and loses double-clicks.
    let pressed = window.use_keyed_state(id, cx, |_, _| false);
    region
        .on_mouse_down(MouseButton::Left, {
            let pressed = pressed.clone();
            move |_, _, cx| pressed.update(cx, |pressed, _| *pressed = true)
        })
        .on_mouse_move(move |event, window, cx| {
            if pressed.update(cx, |pressed, _| std::mem::take(pressed))
                && event.pressed_button == Some(MouseButton::Left)
            {
                window.start_window_move();
            }
        })
        .on_click(|event, window, _| {
            if event.standard_click() && event.click_count() == 2 {
                window.zoom_window();
            }
        })
}

/// Width taken by the resize margins of `frame` on the left and right.
pub fn horizontal_inset(window: &Window) -> Pixels {
    if !cfg!(target_os = "linux") || window.is_fullscreen() || window.is_maximized() {
        return px(0.);
    }
    let Decorations::Client { tiling } = window.window_decorations() else {
        return px(0.);
    };
    let side = |tiled: bool| if tiled { px(0.) } else { RESIZE_BORDER };
    side(tiling.left) + side(tiling.right)
}

/// Tiling of the client frame, `None` when no frame is drawn.
fn frame_tiling(window: &Window) -> Option<Tiling> {
    if !cfg!(target_os = "linux") || window.is_fullscreen() {
        return None;
    }
    let Decorations::Client { tiling } = window.window_decorations() else {
        return None;
    };
    Some(if window.is_maximized() {
        Tiling::tiled()
    } else {
        tiling
    })
}

/// A corner is rounded only when both of its sides float free.
fn rounded_corners(tiling: Tiling, radius: Pixels) -> Corners<Pixels> {
    let corner = |a: bool, b: bool| if a || b { px(0.) } else { radius };
    Corners {
        top_left: corner(tiling.top, tiling.left),
        top_right: corner(tiling.top, tiling.right),
        bottom_right: corner(tiling.bottom, tiling.right),
        bottom_left: corner(tiling.bottom, tiling.left),
    }
}

/// Radii for content touching the window corners. GPUI clips rectangles only,
/// so backgrounds reaching a corner must round it themselves.
pub fn inner_corners(window: &Window) -> Corners<Pixels> {
    frame_tiling(window)
        .map(|tiling| rounded_corners(tiling, CORNER_RADIUS - px(1.)))
        .unwrap_or_default()
}

/// Applies the `corners` radii that are selected by `mask` to `element`.
pub fn round<E: Styled>(mut element: E, corners: Corners<Pixels>, mask: Corners<bool>) -> E {
    let pick = |radius: Pixels, keep: bool| keep.then_some(radius.into());
    let style = &mut element.style().corner_radii;
    style.top_left = pick(corners.top_left, mask.top_left).or(style.top_left);
    style.top_right = pick(corners.top_right, mask.top_right).or(style.top_right);
    style.bottom_right = pick(corners.bottom_right, mask.bottom_right).or(style.bottom_right);
    style.bottom_left = pick(corners.bottom_left, mask.bottom_left).or(style.bottom_left);
    element
}

/// No wrapper or new handlers on Windows. On Linux, a transparent margin carries
/// the window shadow and the compositor-driven resize zones with their cursors.
pub fn frame(content: Div, window: &mut Window, theme: Theme) -> Div {
    if !cfg!(target_os = "linux") {
        return content;
    }
    let Some(tiling) = frame_tiling(window) else {
        window.set_client_inset(px(0.));
        return content;
    };
    window.set_client_inset(RESIZE_BORDER);
    div()
        .relative()
        .size_full()
        .pt(if tiling.top { px(0.) } else { RESIZE_BORDER })
        .pr(if tiling.right { px(0.) } else { RESIZE_BORDER })
        .pb(if tiling.bottom { px(0.) } else { RESIZE_BORDER })
        .pl(if tiling.left { px(0.) } else { RESIZE_BORDER })
        .child(
            canvas(
                move |_, window, _| {
                    resize_zones(window.viewport_size(), tiling)
                        .map(|(edge, zone)| {
                            (edge, window.insert_hitbox(zone, HitboxBehavior::Normal))
                        })
                        .collect::<Vec<_>>()
                },
                |_, zones, window, _| {
                    for (edge, hitbox) in zones {
                        window.set_cursor_style(resize_cursor(edge), &hitbox);
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            if !window.is_maximized()
                && !window.is_fullscreen()
                && let Some(edge) = resize_edge(event.position, window.viewport_size(), tiling)
            {
                window.start_window_resize(edge);
                cx.stop_propagation();
            }
        })
        .child(
            round(
                content,
                rounded_corners(tiling, CORNER_RADIUS),
                Corners::all(true),
            )
            .border_color(theme.border)
            .when(!tiling.top, |content| content.border_t_1())
            .when(!tiling.right, |content| content.border_r_1())
            .when(!tiling.bottom, |content| content.border_b_1())
            .when(!tiling.left, |content| content.border_l_1())
            .when(!tiling.is_tiled(), |content| {
                content.shadow(vec![BoxShadow {
                    color: hsla(0., 0., 0., 0.4),
                    offset: point(px(0.), px(0.)),
                    blur_radius: RESIZE_BORDER / 2.,
                    spread_radius: px(0.),
                }])
            }),
        )
}

/// Non-overlapping zones of the margin, corners first; tiled sides have none.
fn resize_zones(
    size: Size<Pixels>,
    tiling: Tiling,
) -> impl Iterator<Item = (ResizeEdge, Bounds<Pixels>)> {
    let side = |tiled: bool| if tiled { px(0.) } else { RESIZE_BORDER };
    let (top, right, bottom, left) = (
        side(tiling.top),
        side(tiling.right),
        side(tiling.bottom),
        side(tiling.left),
    );
    let (width, height) = (size.width, size.height);
    let (inner_width, inner_height) = (width - left - right, height - top - bottom);
    let zone = |x, y, w, h| Bounds::new(point(x, y), gpui::size(w, h));
    [
        (ResizeEdge::TopLeft, zone(px(0.), px(0.), left, top)),
        (
            ResizeEdge::TopRight,
            zone(width - right, px(0.), right, top),
        ),
        (
            ResizeEdge::BottomLeft,
            zone(px(0.), height - bottom, left, bottom),
        ),
        (
            ResizeEdge::BottomRight,
            zone(width - right, height - bottom, right, bottom),
        ),
        (ResizeEdge::Top, zone(left, px(0.), inner_width, top)),
        (
            ResizeEdge::Bottom,
            zone(left, height - bottom, inner_width, bottom),
        ),
        (ResizeEdge::Left, zone(px(0.), top, left, inner_height)),
        (
            ResizeEdge::Right,
            zone(width - right, top, right, inner_height),
        ),
    ]
    .into_iter()
    .filter(|(_, zone)| !zone.is_empty())
}

fn resize_edge(position: Point<Pixels>, size: Size<Pixels>, tiling: Tiling) -> Option<ResizeEdge> {
    // Half-open, unlike `Bounds::contains`, so adjacent zones never share a pixel.
    resize_zones(size, tiling)
        .find(|(_, zone)| {
            let end = zone.bottom_right();
            (zone.origin.x..end.x).contains(&position.x)
                && (zone.origin.y..end.y).contains(&position.y)
        })
        .map(|(edge, _)| edge)
}

fn resize_cursor(edge: ResizeEdge) -> CursorStyle {
    match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}

#[cfg(test)]
#[path = "tests/window_chrome.rs"]
mod tests;
