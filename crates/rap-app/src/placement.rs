//! Window size and position remembered between sessions.
use gpui::*;
use rap_core::config::WindowPlacement;

pub const DEFAULT_SIZE: Size<Pixels> = Size {
    width: px(1180.),
    height: px(780.),
};
pub const MIN_SIZE: Size<Pixels> = Size {
    width: px(640.),
    height: px(600.),
};
/// Part of the title bar that must stay on a screen to grab the window.
const VISIBLE: f32 = 48.;

/// The restored bounds, maximized or not; `None` while minimized.
pub fn capture(window: &Window) -> Option<WindowPlacement> {
    let (bounds, maximized) = match window.window_bounds() {
        WindowBounds::Windowed(bounds) | WindowBounds::Fullscreen(bounds) => (bounds, false),
        WindowBounds::Maximized(bounds) => (bounds, true),
    };
    // Windows parks minimized windows far outside every screen.
    let placement = WindowPlacement {
        x: f32::from(bounds.origin.x),
        y: f32::from(bounds.origin.y),
        width: f32::from(bounds.size.width),
        height: f32::from(bounds.size.height),
        maximized,
    };
    (placement.x > -10_000. && placement.y > -10_000.).then_some(placement)
}

/// Saved bounds when their title bar is still on a connected screen; centered
/// default bounds otherwise.
pub fn restore(saved: Option<WindowPlacement>, cx: &App) -> WindowBounds {
    let restored = saved.and_then(|p| {
        let bounds = Bounds::new(
            point(px(p.x), px(p.y)),
            size(
                px(p.width).max(MIN_SIZE.width),
                px(p.height).max(MIN_SIZE.height),
            ),
        );
        let grip = Bounds::new(bounds.origin, size(bounds.size.width, px(VISIBLE)));
        let visible = cx
            .displays()
            .iter()
            .any(|display| display.bounds().intersects(&grip));
        visible.then_some((bounds, p.maximized))
    });
    match restored {
        Some((bounds, true)) => WindowBounds::Maximized(bounds),
        Some((bounds, false)) => WindowBounds::Windowed(bounds),
        None => WindowBounds::Windowed(Bounds::centered(None, DEFAULT_SIZE, cx)),
    }
}
