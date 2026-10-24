//! Application icon resources and native ownership, used on the desktop worker.
use anyhow::{Context, Result};
use rap_core::config::AppIcon;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
    core::PCWSTR,
};

pub(super) struct Icon(pub HICON);

impl Icon {
    pub fn load(palette: AppIcon, size: i32) -> Result<Self> {
        // Resource 1 is the executable default; 2..7 follow AppIcon::ALL.
        let resource = 2 + palette as usize;
        if let Ok(module) = unsafe { GetModuleHandleW(None) }
            && let Ok(handle) = unsafe {
                LoadImageW(
                    Some(module.into()),
                    PCWSTR(resource as _),
                    IMAGE_ICON,
                    size,
                    size,
                    LR_DEFAULTCOLOR,
                )
            }
        {
            return Ok(Self(HICON(handle.0)));
        }
        // Standalone platform tests have no executable resources. The embedded
        // SVG fallback is also bounded and runs only on the desktop worker.
        let bytes: &[u8] = match palette {
            AppIcon::Blue => include_bytes!("../../../../assets/logos/blue.svg"),
            AppIcon::Teal => include_bytes!("../../../../assets/logos/teal.svg"),
            AppIcon::Violet => include_bytes!("../../../../assets/logos/violet.svg"),
            AppIcon::Copper => include_bytes!("../../../../assets/logos/copper.svg"),
            AppIcon::GraphiteMint => include_bytes!("../../../../assets/logos/graphite-mint.svg"),
            AppIcon::IvoryInk => include_bytes!("../../../../assets/logos/ivory-ink.svg"),
        };
        let tree = resvg::usvg::Tree::from_data(bytes, &Default::default())?;
        let size = size.max(1) as u32;
        let mut image = resvg::tiny_skia::Pixmap::new(size, size)
            .context("Cannot allocate application icon")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(
                size as f32 / tree.size().width(),
                size as f32 / tree.size().height(),
            ),
            &mut image.as_mut(),
        );
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for row in image.pixels().chunks_exact(size as usize).rev() {
            for pixel in row {
                let color = pixel.demultiply();
                pixels.extend_from_slice(&[
                    color.blue(),
                    color.green(),
                    color.red(),
                    color.alpha(),
                ]);
            }
        }
        let mask = vec![0u8; (size.div_ceil(32) * 4 * size) as usize];
        Ok(Self(unsafe {
            CreateIcon(
                None,
                size as i32,
                size as i32,
                1,
                32,
                mask.as_ptr(),
                pixels.as_ptr(),
            )?
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

pub(super) struct IconPair {
    pub small: Icon,
    pub big: Icon,
}

impl IconPair {
    pub fn load(palette: AppIcon) -> Result<Self> {
        Ok(Self {
            small: Icon::load(palette, unsafe { GetSystemMetrics(SM_CXSMICON) })?,
            big: Icon::load(palette, unsafe { GetSystemMetrics(SM_CXICON) })?,
        })
    }

    pub fn apply(&self, window: HWND) -> Result<()> {
        send(window, WM_SETICON, ICON_SMALL, self.small.0.0 as isize)?;
        send(window, WM_SETICON, ICON_BIG, self.big.0.0 as isize)?;
        Ok(())
    }
}

fn send(window: HWND, message: u32, kind: u32, value: isize) -> Result<usize> {
    let mut result = 0;
    let delivered = unsafe {
        SendMessageTimeoutW(
            window,
            message,
            WPARAM(kind as usize),
            LPARAM(value),
            SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
            200,
            Some(&mut result),
        )
    };
    anyhow::ensure!(
        delivered.0 != 0,
        "Window icon update timed out or the window closed"
    );
    Ok(result)
}

/// Keep all palettes alive while Windows can reference their HICON handles.
pub(super) struct WindowIcons {
    window: HWND,
    original: [usize; 2],
    palettes: Vec<IconPair>,
    attempted: Option<AppIcon>,
}

impl WindowIcons {
    pub fn new(window: HWND) -> Result<Self> {
        let palettes = AppIcon::ALL
            .into_iter()
            .map(IconPair::load)
            .collect::<Result<_>>()?;
        let original = [
            send(window, WM_GETICON, ICON_SMALL, 0)?,
            send(window, WM_GETICON, ICON_BIG, 0)?,
        ];
        Ok(Self {
            window,
            original,
            palettes,
            attempted: None,
        })
    }

    pub fn apply(&mut self, palette: AppIcon) -> Result<()> {
        if self.attempted == Some(palette) {
            return Ok(());
        }
        // Report a failed choice once, rather than on every playback tick.
        self.attempted = Some(palette);
        self.palettes[palette as usize].apply(self.window)
    }
}

impl Drop for WindowIcons {
    fn drop(&mut self) {
        let small = send(
            self.window,
            WM_SETICON,
            ICON_SMALL,
            self.original[0] as isize,
        );
        let big = send(self.window, WM_SETICON, ICON_BIG, self.original[1] as isize);
        if (small.is_err() || big.is_err()) && unsafe { IsWindow(Some(self.window)) }.as_bool() {
            // An unresponsive borrowed window may still consume a timed-out
            // message. Retain this bounded cache rather than free live handles.
            std::mem::forget(std::mem::take(&mut self.palettes));
        }
    }
}

#[cfg(test)]
#[path = "tests/app_icons.rs"]
mod tests;
