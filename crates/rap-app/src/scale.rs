//! Rem reference for the few APIs that need concrete pixels (canvas painting,
//! virtual list measurements, column counts). Layout itself uses `rems`, which
//! GPUI resolves with `Window::rem_size`. Adapted from Oxyde's `ui/scale.rs`.
//! Measured GPUI geometry (pointer positions, bounds) is already in pixels and
//! must never be scaled again.
use gpui::{Pixels, Window, px};
use rap_core::config::Settings;
use std::cell::Cell;

thread_local! {
    // Layout and painting run on the UI thread; synchronized before each render.
    static REM_SIZE: Cell<f32> = const { Cell::new(Settings::DEFAULT_REM_SIZE as f32) };
}

/// Applies the user's interface size to the window and to pixel conversions.
pub fn apply(rem_size: u32, window: &mut Window) {
    let size = Settings::clamp_rem_size(rem_size) as f32;
    REM_SIZE.with(|rem| rem.set(size));
    if window.rem_size() != px(size) {
        window.set_rem_size(px(size));
    }
}

/// Converts a design length given at the 16 px reference into current pixels.
pub fn scaled(value: f32) -> f32 {
    REM_SIZE.with(|rem| value * rem.get() / Settings::DEFAULT_REM_SIZE as f32)
}

pub fn scaled_px(value: f32) -> Pixels {
    px(scaled(value))
}
