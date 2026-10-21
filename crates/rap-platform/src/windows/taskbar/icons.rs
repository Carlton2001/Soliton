use anyhow::{Context, Result};
use windows::Win32::UI::WindowsAndMessaging::{CreateIcon, DestroyIcon, HICON};

pub(super) struct Icon(pub HICON);

impl Icon {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let options = resvg::usvg::Options {
            // Neutral gray stays legible on Windows' light and dark flyouts.
            style_sheet: Some("svg { color: #808080; }".into()),
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
        let mut pixels = Vec::with_capacity(32 * 32 * 4);
        for row in image.data().chunks_exact(32 * 4).rev() {
            for rgba in row.chunks_exact(4) {
                pixels.extend_from_slice(&[rgba[2], rgba[1], rgba[0], rgba[3]]);
            }
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
