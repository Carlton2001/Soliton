//! Application icon resources and native ownership, used on the desktop worker.
use anyhow::{Context, Result};
use soliton_core::config::AppIcon;
use std::time::{Duration, Instant};
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
        // CreateIcon takes top-down rows, unlike the bottom-up DIBs of icon files.
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for pixel in image.pixels() {
            let color = pixel.demultiply();
            pixels.extend_from_slice(&[color.blue(), color.green(), color.red(), color.alpha()]);
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

/// The UI thread may be busy past the message timeout, notably while the first
/// frame is prepared: the icon is tried again at this pace.
const RETRY_EVERY: Duration = Duration::from_millis(250);
/// A failure is reported once the window stayed unresponsive this long.
const REPORT_AFTER: Duration = Duration::from_secs(10);

/// Keep all palettes alive while Windows can reference their HICON handles.
pub(super) struct WindowIcons {
    window: HWND,
    /// Read before the first change, restored on drop.
    original: Option<[usize; 2]>,
    palettes: Vec<IconPair>,
    applied: Option<AppIcon>,
    /// When the shown palette first failed to change, and whether that was reported.
    failing: Option<(Instant, bool)>,
    next_try: Instant,
}

impl WindowIcons {
    pub fn new(window: HWND) -> Result<Self> {
        let palettes = AppIcon::ALL
            .into_iter()
            .map(IconPair::load)
            .collect::<Result<_>>()?;
        Ok(Self {
            window,
            original: None,
            palettes,
            applied: None,
            failing: None,
            next_try: Instant::now(),
        })
    }

    /// Shows `palette`; cheap once shown, so it can be called on every worker turn.
    /// A window that does not answer is tried again later, and its failure is
    /// returned once, rather than on every playback tick.
    pub fn apply(&mut self, palette: AppIcon) -> Result<()> {
        if self.applied == Some(palette) || Instant::now() < self.next_try {
            return Ok(());
        }
        match self.change(palette) {
            Ok(()) => {
                self.applied = Some(palette);
                self.failing = None;
                Ok(())
            }
            Err(error) => {
                self.next_try = Instant::now() + RETRY_EVERY;
                let (since, reported) = self.failing.get_or_insert((Instant::now(), false));
                if *reported || since.elapsed() < REPORT_AFTER {
                    return Ok(());
                }
                *reported = true;
                Err(error)
            }
        }
    }

    fn change(&mut self, palette: AppIcon) -> Result<()> {
        if self.original.is_none() {
            self.original = Some([
                send(self.window, WM_GETICON, ICON_SMALL, 0)?,
                send(self.window, WM_GETICON, ICON_BIG, 0)?,
            ]);
        }
        self.palettes[palette as usize].apply(self.window)
    }
}

impl Drop for WindowIcons {
    fn drop(&mut self) {
        // Icons are only sent once the originals are known: none was handed over.
        let Some(original) = self.original else {
            return;
        };
        let small = send(self.window, WM_SETICON, ICON_SMALL, original[0] as isize);
        let big = send(self.window, WM_SETICON, ICON_BIG, original[1] as isize);
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
