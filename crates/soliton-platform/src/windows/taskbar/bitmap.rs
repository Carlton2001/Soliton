//! Owned 32-bit DIBs. Every selected GDI object is restored before destruction.
#[cfg(test)]
use anyhow::Context;
use anyhow::Result;
use windows::{
    Win32::{
        Foundation::{COLORREF, RECT},
        Graphics::Gdi::*,
    },
    core::w,
};

pub(super) struct Frame {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

impl Frame {
    pub fn bitmap(&self, max_width: i32, max_height: i32) -> Result<Surface> {
        let (width, height) = fitted(self.width, self.height, max_width, max_height);
        let surface = Surface::new(width, height, 0)?;
        unsafe {
            SetStretchBltMode(surface.dc, HALFTONE);
            let copied = StretchDIBits(
                surface.dc,
                0,
                0,
                width,
                height,
                0,
                0,
                self.width,
                self.height,
                Some(self.pixels.as_ptr().cast()),
                &info(self.width, self.height),
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            anyhow::ensure!(copied != 0 && copied != -1, "Cannot resize taskbar preview");
            GdiFlush().ok()?;
        }
        // GDI blits may clear the alpha channel; DWM requires opaque pixels.
        let mut surface = surface;
        for pixel in surface.pixels().as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
        Ok(surface)
    }
}

pub(super) fn fitted(width: i32, height: i32, max_width: i32, max_height: i32) -> (i32, i32) {
    let scale = (max_width.clamp(1, 4096) as f64 / width as f64)
        .min(max_height.clamp(1, 4096) as f64 / height as f64)
        .min(1.);
    (
        (width as f64 * scale).floor().max(1.) as i32,
        (height as f64 * scale).floor().max(1.) as i32,
    )
}

pub(super) struct Surface {
    pub dc: HDC,
    pub bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    width: i32,
    height: i32,
}

fn info(width: i32, height: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

impl Surface {
    pub fn new(width: i32, height: i32, color: u32) -> Result<Self> {
        anyhow::ensure!(
            (1..=4096).contains(&width) && (1..=4096).contains(&height),
            "Invalid preview size"
        );
        unsafe {
            let dc = CreateCompatibleDC(None);
            anyhow::ensure!(!dc.is_invalid(), "Cannot create preview DC");
            let mut bits = std::ptr::null_mut();
            let bitmap = match CreateDIBSection(
                Some(dc),
                &info(width, height),
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            ) {
                Ok(bitmap) => bitmap,
                Err(error) => {
                    let _ = DeleteDC(dc);
                    return Err(error.into());
                }
            };
            let old = SelectObject(dc, bitmap.into());
            let mut surface = Self {
                dc,
                bitmap,
                old,
                bits: bits.cast(),
                width,
                height,
            };
            surface.rect(0, 0, width, height, color);
            Ok(surface)
        }
    }

    fn pixels(&mut self) -> &mut [u8] {
        // SAFETY: this DIB owns width*height 32-bit, top-down pixels until Drop.
        unsafe {
            std::slice::from_raw_parts_mut(self.bits, (self.width * self.height * 4) as usize)
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: u32) {
        let stride = self.width as usize;
        let right = (x + width).min(self.width);
        let bottom = (y + height).min(self.height);
        let pixel = [color as u8, (color >> 8) as u8, (color >> 16) as u8, 255];
        let pixels = self.pixels();
        for row in y.max(0)..bottom {
            for column in x.max(0)..right {
                let start = (row as usize * stride + column as usize) * 4;
                pixels[start..start + 4].copy_from_slice(&pixel);
            }
        }
    }

    pub fn image(&mut self, image: &image::RgbaImage, x: i32, y: i32) {
        let stride = self.width as usize;
        let height = self.height;
        let pixels = self.pixels();
        for (ix, iy, rgba) in image.enumerate_pixels() {
            let (dx, dy) = (x + ix as i32, y + iy as i32);
            if dx < 0 || dx >= stride as i32 || dy < 0 || dy >= height {
                continue;
            }
            let start = (dy as usize * stride + dx as usize) * 4;
            for (channel, source) in [rgba[2], rgba[1], rgba[0]].into_iter().enumerate() {
                let alpha = u32::from(rgba[3]);
                pixels[start + channel] = ((u32::from(source) * alpha
                    + u32::from(pixels[start + channel]) * (255 - alpha))
                    / 255) as u8;
            }
        }
    }

    pub fn text(
        &self,
        text: &str,
        bounds: (i32, i32, i32, i32),
        size: i32,
        bold: bool,
        color: u32,
    ) -> Result<()> {
        let mut units: Vec<u16> = text
            .replace(['\r', '\n'], " ")
            .chars()
            .take(512)
            .collect::<String>()
            .encode_utf16()
            .collect();
        if units.is_empty() {
            return Ok(());
        }
        unsafe {
            let font = CreateFontW(
                -size,
                0,
                0,
                0,
                if bold { 600 } else { 400 },
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                DEFAULT_PITCH.0 as u32,
                w!("Segoe UI"),
            );
            anyhow::ensure!(!font.is_invalid(), "Cannot create preview font");
            let previous = SelectObject(self.dc, font.into());
            SetBkMode(self.dc, TRANSPARENT);
            SetTextColor(
                self.dc,
                COLORREF(((color & 0xff) << 16) | (color & 0xff00) | (color >> 16)),
            );
            let (left, top, width, height) = bounds;
            let mut rect = RECT {
                left,
                top,
                right: left + width,
                bottom: top + height,
            };
            DrawTextW(
                self.dc,
                &mut units,
                &mut rect,
                DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX | DT_VCENTER,
            );
            SelectObject(self.dc, previous);
            let _ = DeleteObject(font.into());
        }
        Ok(())
    }

    pub fn frame(mut self) -> Result<Frame> {
        unsafe {
            GdiFlush().ok()?;
        }
        let mut pixels = self.pixels().to_vec();
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
        anyhow::ensure!(!pixels.is_empty(), "Empty preview frame");
        Ok(Frame {
            width: self.width,
            height: self.height,
            pixels,
        })
    }

    #[cfg(test)]
    pub fn save(&mut self, path: &std::path::Path) -> Result<()> {
        unsafe {
            GdiFlush().ok()?;
        }
        let (width, height) = (self.width as u32, self.height as u32);
        let mut rgba = self.pixels().to_vec();
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        image::RgbaImage::from_raw(width, height, rgba)
            .context("Invalid preview pixels")?
            .save(path)?;
        Ok(())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}
