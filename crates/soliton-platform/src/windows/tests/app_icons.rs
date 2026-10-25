use super::*;
use std::sync::mpsc;
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
            w!("soliton icon test"),
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
    let original = [
        send(window.0, WM_GETICON, ICON_SMALL, 0).unwrap(),
        send(window.0, WM_GETICON, ICON_BIG, 0).unwrap(),
    ];
    let mut icons = WindowIcons::new(window.0).unwrap();
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

fn popup() -> HWND {
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("soliton icon test"),
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
    }
}

#[test]
fn a_window_busy_at_startup_receives_its_icon_once_it_answers() {
    let (created, handle) = mpsc::channel();
    let (stop, stopped) = mpsc::channel::<()>();
    // The owner thread stands for a UI thread still preparing its first frame.
    let owner = std::thread::spawn(move || {
        let window = TestWindow(popup());
        created.send(window.0.0 as isize).unwrap();
        std::thread::sleep(Duration::from_millis(600));
        while stopped.try_recv().is_err() {
            unsafe {
                let mut message = MSG::default();
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    DispatchMessageW(&message);
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    });
    let window = HWND(handle.recv().unwrap() as *mut _);
    let mut icons = WindowIcons::new(window).unwrap();
    // The first attempt times out without any warning, and is not final.
    icons.apply(AppIcon::Teal).unwrap();
    assert!(icons.applied.is_none());
    let deadline = Instant::now() + Duration::from_secs(5);
    while icons.applied.is_none() {
        assert!(Instant::now() < deadline, "Icon never applied");
        icons.apply(AppIcon::Teal).unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        send(window, WM_GETICON, ICON_BIG, 0).unwrap(),
        icons.palettes[AppIcon::Teal as usize].big.0.0 as usize
    );
    drop(icons);
    stop.send(()).unwrap();
    owner.join().unwrap();
}

#[test]
fn a_lasting_failure_is_reported_once() {
    let window = popup();
    unsafe {
        DestroyWindow(window).unwrap();
    }
    let mut icons = WindowIcons::new(window).unwrap();
    icons.apply(AppIcon::Blue).unwrap();
    // Pretend the window has not answered for longer than the reporting delay.
    icons.failing = Some((Instant::now() - REPORT_AFTER, false));
    icons.next_try = Instant::now();
    assert!(icons.apply(AppIcon::Blue).is_err());
    icons.next_try = Instant::now();
    icons.apply(AppIcon::Blue).unwrap();
    assert!(icons.applied.is_none());
}
