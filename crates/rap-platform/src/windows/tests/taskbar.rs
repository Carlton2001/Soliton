use super::*;
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    UI::WindowsAndMessaging::*,
};
use windows::core::w;

#[test]
fn toolbar_commands_do_not_confuse_other_window_commands() {
    assert_eq!(command(PREVIOUS), Some(MediaCommand::Previous));
    assert_eq!(command(TOGGLE), Some(MediaCommand::Toggle));
    assert_eq!(command(NEXT), Some(MediaCommand::Next));
    assert_eq!(command(STOP), Some(MediaCommand::Stop));
    assert_eq!(command(1), None);
}

#[test]
#[ignore = "Requires an interactive Windows taskbar and DWM"]
fn native_toolbar_buttons_thumbnail_restart_and_detach() {
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED).unwrap();
    }
    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("RAP taskbar QA"),
            WS_OVERLAPPEDWINDOW,
            40,
            40,
            320,
            200,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    };
    let (events, received) = crossbeam_channel::bounded(64);
    let shared = install(window, events).unwrap();
    unsafe {
        let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
    }
    let mut taskbar = Taskbar::new(window, shared.clone()).unwrap();
    let mut state = MediaState::default();
    state.track = Some(crate::MediaTrack {
        title: "QA".into(),
        artist: "Artiste".into(),
        album: "Album".into(),
        cover: None,
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while !shared.toolbar_ready.load(Ordering::Acquire) && Instant::now() < deadline {
        taskbar.update(&state, Language::French).unwrap();
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        shared.toolbar_ready.load(Ordering::Acquire),
        "Windows did not accept the thumbnail toolbar"
    );
    for id in [PREVIOUS, TOGGLE, NEXT, STOP] {
        unsafe {
            SendMessageW(
                window,
                WM_COMMAND,
                Some(WPARAM(((THBN_CLICKED as usize) << 16) | id as usize)),
                Some(LPARAM(0)),
            );
        }
        assert!(
            matches!(received.recv_timeout(Duration::from_secs(1)).unwrap(), DesktopEvent::Media(value) if Some(value) == command(id))
        );
    }
    let thumbnails = shared.thumbnails.load(Ordering::Acquire);
    // Use a conservative maximum for this small test window. In production the
    // dimensions come from DWM's own request, not from a synthetic message.
    unsafe {
        SendMessageW(
            window,
            WM_DWMSENDICONICTHUMBNAIL,
            None,
            Some(LPARAM((64 << 16) | 36)),
        );
    }
    assert!(
        shared.thumbnails.load(Ordering::Acquire) > thumbnails,
        "DWM did not accept the custom thumbnail"
    );
    state.state = TransportState::Playing;
    taskbar.update(&state, Language::English).unwrap();
    let buttons = taskbar.buttons(true, true, Language::English);
    assert_eq!(String::from_utf16_lossy(&buttons[1].szTip[..5]), "Pause");
    assert!(
        taskbar
            .buttons(false, false, Language::French)
            .iter()
            .all(|button| button.dwFlags == THBF_DISABLED)
    );
    let before = shared.generation.load(Ordering::Acquire);
    unsafe {
        SendMessageW(window, shared.created, None, None);
    }
    assert!(shared.generation.load(Ordering::Acquire) > before);
    drop(taskbar);
    detach(window, &shared);
    assert!(!shared.enabled.load(Ordering::Acquire));
    assert_eq!(
        Arc::strong_count(&shared),
        1,
        "Subclass retained a reference after detach"
    );
    unsafe {
        let _ = DestroyWindow(window);
        RoUninitialize();
    }
}
