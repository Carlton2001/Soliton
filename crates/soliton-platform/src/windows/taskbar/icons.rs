use anyhow::{Context, Result};
use windows::Win32::UI::WindowsAndMessaging::{CreateIcon, DestroyIcon, HICON};

pub(super) struct Icon(pub HICON);

impl Icon {
    pub fn new(bytes: &[u8], accent: u32) -> Result<Self> {
        let options = resvg::usvg::Options {
            // Style the official Lucide paths only for the native toolbar.
            style_sheet: Some(format!(
                "svg {{ color: #{:06x}; stroke-width: 3; }}",
                accent & 0xffffff
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

pub(super) fn toolbar(accent: u32) -> Result<[Icon; 5]> {
    Ok([
        Icon::new(
            include_bytes!("../../../../../assets/icons/skip-back.svg"),
            accent,
        )?,
        Icon::new(
            include_bytes!("../../../../../assets/icons/play.svg"),
            accent,
        )?,
        Icon::new(
            include_bytes!("../../../../../assets/icons/pause.svg"),
            accent,
        )?,
        Icon::new(
            include_bytes!("../../../../../assets/icons/skip-forward.svg"),
            accent,
        )?,
        Icon::new(
            include_bytes!("../../../../../assets/icons/square.svg"),
            accent,
        )?,
    ])
}
