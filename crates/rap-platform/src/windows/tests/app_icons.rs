use super::*;
use windows::core::w;

struct TestWindow(HWND);
impl Drop for TestWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

#[test]
fn palette_changes_update_both_window_icons_and_restore_borrowed_handles() {
    let window = TestWindow(unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("rap icon test"),
            WS_POPUP,
            0,
            0,
            32,
            32,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let mut icons = WindowIcons::new(window.0).unwrap();
    let original = icons.original;
    for palette in AppIcon::ALL {
        icons.apply(palette).unwrap();
        let pair = &icons.palettes[palette as usize];
        assert_eq!(
            send(window.0, WM_GETICON, ICON_SMALL, 0).unwrap(),
            pair.small.0.0 as usize
        );
        assert_eq!(
            send(window.0, WM_GETICON, ICON_BIG, 0).unwrap(),
            pair.big.0.0 as usize
        );
        // A no-op refresh keeps the same native handles.
        icons.apply(palette).unwrap();
    }
    drop(icons);
    assert_eq!(
        send(window.0, WM_GETICON, ICON_SMALL, 0).unwrap(),
        original[0]
    );
    assert_eq!(
        send(window.0, WM_GETICON, ICON_BIG, 0).unwrap(),
        original[1]
    );
}
