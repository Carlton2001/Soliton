//! Native placement of the styled tray menu, which GPUI's bounds cannot express
//! across monitors with different scales.
use crate::TrayAnchor;
use anyhow::Result;
use raw_window_handle::RawWindowHandle;
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::{
        Dwm::{
            DWM_WINDOW_CORNER_PREFERENCE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
            DwmSetWindowAttribute,
        },
        Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
    },
    UI::{
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::*,
    },
};

/// The shell's icon overflow flyout is topmost too and comes back above a popup
/// opened from it, so the popup goes back on top once it is active.
pub fn raise_tray_menu(handle: RawWindowHandle) -> Result<()> {
    let RawWindowHandle::Win32(handle) = handle else {
        anyhow::bail!("Expected a Win32 window");
    };
    let window = HWND(handle.hwnd.get() as *mut _);
    post_position(window, (0, 0), (0, 0), SWP_NOMOVE | SWP_NOSIZE);
    Ok(())
}

pub fn place_tray_menu(
    handle: RawWindowHandle,
    anchor: TrayAnchor,
    (width, height): (f32, f32),
) -> Result<()> {
    let RawWindowHandle::Win32(handle) = handle else {
        anyhow::bail!("Expected a Win32 window");
    };
    let window = HWND(handle.hwnd.get() as *mut _);
    unsafe {
        let point = POINT {
            x: anchor.x,
            y: anchor.y,
        };
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        anyhow::ensure!(
            GetMonitorInfoW(monitor, &mut info).as_bool(),
            "Cannot read the monitor work area"
        );
        let (mut dpi_x, mut dpi_y) = (96, 96);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y)?;
        let scale = dpi_x as f32 / 96.;
        let size = (
            (width * scale).round() as i32,
            (height * scale).round() as i32,
        );
        let (x, y) = origin(anchor, size, info.rcWork);
        let corners = DWMWCP_ROUND;
        // Windows 10 has no rounded corners and refuses the attribute.
        let _ = DwmSetWindowAttribute(
            window,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corners as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
            std::mem::size_of_val(&corners) as u32,
        );
        post_position(window, (x, y), size, SET_WINDOW_POS_FLAGS(0));
    }
    Ok(())
}

/// Called from the window's own thread, even SWP_ASYNCWINDOWPOS moves it at once,
/// and GPUI's move and resize callbacks then find the App already borrowed by the
/// caller. From another thread the request waits for the window's message loop.
fn post_position(
    window: HWND,
    (x, y): (i32, i32),
    (width, height): (i32, i32),
    flags: SET_WINDOW_POS_FLAGS,
) {
    let window = window.0 as isize;
    std::thread::spawn(move || unsafe {
        let _ = SetWindowPos(
            HWND(window as *mut _),
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | flags,
        );
    });
}

/// Centered above the pointer, then pushed back inside the work area.
fn origin(anchor: TrayAnchor, (width, height): (i32, i32), work: RECT) -> (i32, i32) {
    (
        (anchor.x - width / 2)
            .min(work.right - width)
            .max(work.left),
        (anchor.y - height).min(work.bottom - height).max(work.top),
    )
}

#[cfg(test)]
#[path = "tests/tray_popup.rs"]
mod tests;
