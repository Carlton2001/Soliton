use anyhow::{Context, Result};
use windows::Win32::UI::WindowsAndMessaging::{CreateIcon, DestroyIcon, HICON};

pub(super) struct Icon(pub HICON);

impl Icon {
    pub fn new(bytes: &[u8], color: u32) -> Result<Self> {
        let options = resvg::usvg::Options {
            // Style the official Lucide paths only for the native toolbar.
            style_sheet: Some(format!(
                "svg {{ color: #{:06x}; stroke-width: 2; }}",
                color & 0xffffff
            )),
            ..Default::default()
        };
        let tree = resvg::usvg::Tree::from_data(bytes, &options)?;
        let mut image =
            resvg::tiny_skia::Pixmap::new(32, 32).context("Cannot allocate thumbnail button")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(32. / 24., 32. / 24.),
            &mut image.as_mut(),
        );
        // CreateIcon takes top-down rows of straight-alpha BGRA, unlike icon
        // file DIBs; tiny-skia stores premultiplied RGBA.
        let mut pixels = Vec::with_capacity(32 * 32 * 4);
        for pixel in image.pixels() {
            let color = pixel.demultiply();
            pixels.extend_from_slice(&[color.blue(), color.green(), color.red(), color.alpha()]);
        }
        let mask = [0u8; 128];
        Ok(Self(unsafe {
            CreateIcon(None, 32, 32, 1, 32, mask.as_ptr(), pixels.as_ptr())?
        }))
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

#[cfg(test)]
#[path = "../tests/taskbar_icons.rs"]
mod tests;

/// Follows the taskbar (system) theme, which also styles the thumbnail flyout.
pub(super) fn system_light_theme() -> bool {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("SystemUsesLightTheme"))
        .is_ok_and(|value| value != 0)
}

/// The Windows accent, in the shade the system uses for its own theme.
pub(super) fn system_accent(light: bool) -> Option<u32> {
    use windows::UI::ViewManagement::{UIColorType, UISettings};
    let kind = if light {
        UIColorType::AccentDark1
    } else {
        UIColorType::AccentLight2
    };
    let color = UISettings::new().ok()?.GetColorValue(kind).ok()?;
    Some((u32::from(color.R) << 16) | (u32::from(color.G) << 8) | u32::from(color.B))
}

/// Preview colors matching the system flyout that hosts the thumbnail.
pub(super) fn system_preview(light: bool) -> crate::PreviewColors {
    let (background, text, muted, progress_background, fallback) = if light {
        (0xf3f3f3, 0x1b1b1b, 0x5f5f5f, 0xc8c8c8, 0x005fb8)
    } else {
        (0x2c2c2c, 0xffffff, 0xc5c5c5, 0x4a4a4a, 0x60cdff)
    };
    crate::PreviewColors {
        background,
        text,
        muted,
        accent: system_accent(light).unwrap_or(fallback),
        progress_background,
    }
}

pub(super) fn toolbar(light: bool) -> Result<[Icon; 5]> {
    let color = if light { 0x1b1b1b } else { 0xffffff };
    Ok([
        Icon::new(
            include_bytes!("../../../../../assets/icons/skip-back.svg"),
            color,
        )?,
        Icon::new(
            include_bytes!("../../../../../assets/icons/play.svg"),
            color,
        )?,
        Icon::new(
            include_bytes!("../../../../../assets/icons/pause.svg"),
            color,
        )?,
        Icon::new(
            include_bytes!("../../../../../assets/icons/skip-forward.svg"),
            color,
        )?,
        Icon::new(
            include_bytes!("../../../../../assets/icons/square.svg"),
            color,
        )?,
    ])
}
