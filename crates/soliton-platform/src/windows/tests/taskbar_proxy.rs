use super::*;
use crate::MediaCommand;

struct MainWindow(HWND);

impl MainWindow {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Soliton native Peek test"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                320,
                200,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
}

impl Drop for MainWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

#[test]
fn thumbnail_proxy_stays_hidden_and_releases_its_window_and_state() {
    let main = MainWindow::new();
    let (events, _received) = crossbeam_channel::bounded(8);
    let shared = super::super::install(main.0, events).unwrap();
    let proxy = Proxy::new(main.0, shared.clone()).unwrap();
    assert!(!unsafe { IsWindowVisible(proxy.window()) }.as_bool());
    let proxy_window = proxy.window();
    drop(proxy);
    assert!(!unsafe { IsWindow(Some(proxy_window)) }.as_bool());
    super::super::detach(main.0, &shared);
    assert_eq!(Arc::strong_count(&shared), 1);
}

#[test]
fn proxy_routes_playback_activation_and_close_to_soliton() {
    let main = MainWindow::new();
    let (events, received) = crossbeam_channel::bounded(8);
    let shared = super::super::install(main.0, events).unwrap();
    let proxy = Proxy::new(main.0, shared.clone()).unwrap();
    for (id, expected) in [
        (super::super::PREVIOUS, MediaCommand::Previous),
        (super::super::TOGGLE, MediaCommand::Toggle),
        (super::super::NEXT, MediaCommand::Next),
        (super::super::STOP, MediaCommand::Stop),
    ] {
        unsafe {
            SendMessageW(
                proxy.window(),
                WM_COMMAND,
                Some(WPARAM(((THBN_CLICKED as usize) << 16) | id as usize)),
                None,
            );
        }
        assert!(
            matches!(received.try_recv().unwrap(), DesktopEvent::Media(value) if value == expected)
        );
    }
    unsafe {
        SendMessageW(
            proxy.window(),
            WM_ACTIVATE,
            Some(WPARAM(WA_ACTIVE as usize)),
            None,
        );
    }
    assert!(matches!(received.try_recv().unwrap(), DesktopEvent::Show));
    unsafe {
        SendMessageW(
            proxy.window(),
            WM_SYSCOMMAND,
            Some(WPARAM(SC_RESTORE as usize)),
            None,
        );
    }
    assert!(matches!(received.try_recv().unwrap(), DesktopEvent::Show));
    for (message, wparam) in [(WM_CLOSE, 0), (WM_SYSCOMMAND, SC_CLOSE as usize)] {
        unsafe {
            SendMessageW(proxy.window(), message, Some(WPARAM(wparam)), None);
            let mut posted = MSG::default();
            assert!(
                PeekMessageW(&mut posted, Some(main.0), WM_CLOSE, WM_CLOSE, PM_REMOVE).as_bool()
            );
            assert!(IsWindow(Some(proxy.window())).as_bool());
        }
    }
    super::super::detach(main.0, &shared);
    unsafe {
        SendMessageW(
            proxy.window(),
            WM_ACTIVATE,
            Some(WPARAM(WA_ACTIVE as usize)),
            None,
        );
    }
    assert!(
        received.is_empty(),
        "Detached proxy must stop sending commands"
    );
    drop(proxy);
    assert_eq!(Arc::strong_count(&shared), 1);
}
