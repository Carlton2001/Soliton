//! Linux client decorations. Windows keeps GPUI's native non-client handling.
use crate::{placement, theme::Theme};
use gpui::{prelude::*, *};

const RESIZE_BORDER: Pixels = px(4.);

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
pub fn drag_region<E: InteractiveElement + FluentBuilder>(region: E) -> E {
    region
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Middle, |_, window, _| window.minimize_window())
        .when(cfg!(target_os = "linux"), |region| {
            region.on_mouse_down(MouseButton::Left, |event, window, cx| {
                if event.click_count == 2 {
                    window.zoom_window();
                } else {
                    window.start_window_move();
                }
                cx.stop_propagation();
            })
        })
}

/// No wrapper or new handlers on Windows. Linux resizes through the compositor;
/// the border deliberately leaves the cursor unchanged.
pub fn frame(content: Div, window: &mut Window, theme: Theme) -> Div {
    if !cfg!(target_os = "linux") {
        return content;
    }
    let Decorations::Client { mut tiling } = window.window_decorations() else {
        window.set_client_inset(px(0.));
        return content;
    };
    if window.is_fullscreen() {
        window.set_client_inset(px(0.));
        return content;
    }
    if window.is_maximized() {
        tiling = Tiling::tiled();
    }
    window.set_client_inset(RESIZE_BORDER);
    div()
        .relative()
        .size_full()
        .bg(theme.border)
        .pt(if tiling.top { px(0.) } else { RESIZE_BORDER })
        .pr(if tiling.right { px(0.) } else { RESIZE_BORDER })
        .pb(if tiling.bottom { px(0.) } else { RESIZE_BORDER })
        .pl(if tiling.left { px(0.) } else { RESIZE_BORDER })
        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            if !window.is_maximized()
                && !window.is_fullscreen()
                && let Some(edge) = resize_edge(event.position, window.viewport_size(), tiling)
            {
                window.start_window_resize(edge);
                cx.stop_propagation();
            }
        })
        .child(content)
}

fn resize_edge(position: Point<Pixels>, size: Size<Pixels>, tiling: Tiling) -> Option<ResizeEdge> {
    if position.x < px(0.)
        || position.y < px(0.)
        || position.x >= size.width
        || position.y >= size.height
    {
        return None;
    }
    let top = !tiling.top && position.y < RESIZE_BORDER;
    let bottom = !tiling.bottom && position.y >= size.height - RESIZE_BORDER;
    let left = !tiling.left && position.x < RESIZE_BORDER;
    let right = !tiling.right && position.x >= size.width - RESIZE_BORDER;
    match (top, bottom, left, right) {
        (true, _, true, _) => Some(ResizeEdge::TopLeft),
        (true, _, _, true) => Some(ResizeEdge::TopRight),
        (_, true, true, _) => Some(ResizeEdge::BottomLeft),
        (_, true, _, true) => Some(ResizeEdge::BottomRight),
        (true, _, _, _) => Some(ResizeEdge::Top),
        (_, true, _, _) => Some(ResizeEdge::Bottom),
        (_, _, true, _) => Some(ResizeEdge::Left),
        (_, _, _, true) => Some(ResizeEdge::Right),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/window_chrome.rs"]
mod tests;
