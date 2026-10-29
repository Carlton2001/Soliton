use super::*;
use windows::Win32::{
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits,
        ReleaseDC,
    },
    UI::WindowsAndMessaging::{GetIconInfo, ICONINFO},
};

/// Top-down BGRA pixels of an icon's color bitmap.
fn pixels(icon: &Icon) -> Vec<u8> {
    unsafe {
        let mut info = ICONINFO::default();
        GetIconInfo(icon.0, &mut info).unwrap();
        let mut header = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 32,
                biHeight: -32,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; 32 * 32 * 4];
        let dc = GetDC(None);
        let rows = GetDIBits(
            dc,
            info.hbmColor,
            0,
            32,
            Some(pixels.as_mut_ptr().cast()),
            &mut header,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, dc);
        let _ = DeleteObject(info.hbmColor.into());
        let _ = DeleteObject(info.hbmMask.into());
        assert_eq!(rows, 32);
        pixels
    }
}

#[test]
fn toolbar_icons_keep_their_orientation_and_straight_alpha() {
    // Opaque top half, half-transparent bottom half, in the accent color.
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
        <rect width="24" height="12" fill="currentColor"/>
        <rect y="12" width="24" height="12" fill="currentColor" fill-opacity="0.5"/>
    </svg>"#;
    let pixels = pixels(&Icon::new(svg, 0x3366cc).unwrap());
    let at = |row: usize| &pixels[row * 32 * 4..row * 32 * 4 + 4];
    assert_eq!(at(0), [0xcc, 0x66, 0x33, 0xff]);
    let bottom = at(31);
    assert!(bottom[3].abs_diff(128) <= 1, "{bottom:?}");
    // Straight alpha: the color is not darkened by its own transparency.
    assert!(bottom[0].abs_diff(0xcc) <= 2, "{bottom:?}");
}
