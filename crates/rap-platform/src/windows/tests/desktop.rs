//! Exercises the real Windows services, without touching music or startup settings.
use super::*;
use crate::{Desktop, MediaCommand, MediaTrack, TransportState};
use raw_window_handle::Win32WindowHandle;
use std::{num::NonZeroIsize, time::Instant};
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    },
    core::w,
};

struct TestWindow(HWND);
impl Drop for TestWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

fn wait_event(desktop: &Desktop, predicate: impl Fn(&DesktopEvent) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(event) = desktop.events.recv_timeout(Duration::from_millis(100)) {
            assert!(!matches!(event, DesktopEvent::Warning(_)), "{event:?}");
            if predicate(&event) {
                return;
            }
        }
    }
    panic!("Native event timed out");
}

fn find_session(
    manager: &GlobalSystemMediaTransportControlsSessionManager,
    title: &str,
) -> GlobalSystemMediaTransportControlsSession {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        for session in manager.GetSessions().unwrap() {
            if let Ok(properties) = session.TryGetMediaPropertiesAsync().unwrap().get()
                && properties.Title().unwrap() == title
            {
                return session;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("RAP SMTC session not published");
}

fn tray_window() -> Option<HWND> {
    let mut after = None;
    loop {
        let window =
            unsafe { FindWindowExW(None, after, w!("RAP.DesktopIntegration"), None).ok()? };
        let mut process = 0;
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut process));
        }
        if process == std::process::id() {
            return Some(window);
        }
        after = Some(window);
    }
}

#[test]
#[ignore = "Requires an interactive Windows desktop and SMTC services"]
fn native_tray_smtc_artwork_commands_and_cleanup() {
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED).unwrap();
    }
    let window = TestWindow(unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("RAP native integration test"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            100,
            100,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let raw = RawWindowHandle::Win32(Win32WindowHandle::new(
        NonZeroIsize::new(window.0.0 as isize).unwrap(),
    ));
    let desktop = Desktop::start(raw, Language::English, Default::default()).unwrap();
    wait_event(&desktop, |event| {
        matches!(event, DesktopEvent::Autostart(Ok(_)))
    });
    assert!(desktop.can_hide(), "Tray icon was not registered");
    assert!(desktop.hide());
    assert!(!unsafe { IsWindowVisible(window.0) }.as_bool());

    let directory = tempfile::tempdir().unwrap();
    let cover = directory.path().join("cover.png");
    let mut image = resvg::tiny_skia::Pixmap::new(16, 16).unwrap();
    image.fill(resvg::tiny_skia::Color::from_rgba8(45, 212, 191, 255));
    image.save_png(&cover).unwrap();
    let title = format!("RAP native QA {}", std::process::id());
    let state = MediaState {
        track: Some(MediaTrack {
            title: title.clone(),
            artist: "Artiste été".into(),
            album: "Album QA".into(),
            // Library cache paths are canonical Windows paths, including \\?\.
            cover: Some(cover.canonicalize().unwrap()),
        }),
        state: TransportState::Playing,
        position_ms: 1200,
        duration_ms: 60_000,
        ..Default::default()
    };
    desktop.publish(state.clone(), Language::English, Default::default());
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .unwrap()
        .get()
        .unwrap();
    let session = find_session(&manager, &title);
    for event in desktop.events.try_iter() {
        assert!(!matches!(event, DesktopEvent::Warning(_)), "{event:?}");
    }
    let properties = session.TryGetMediaPropertiesAsync().unwrap().get().unwrap();
    assert_eq!(properties.Artist().unwrap(), "Artiste été");
    assert!(
        properties
            .Thumbnail()
            .unwrap()
            .OpenReadAsync()
            .unwrap()
            .get()
            .unwrap()
            .Size()
            .unwrap()
            > 0
    );
    // Long canonical cache paths also work, without stripping their prefix or
    // depending on the WinRT StorageFile path parser.
    let long_directory = directory
        .path()
        .join("Pochettes été")
        .join("a".repeat(120))
        .join("b".repeat(120));
    std::fs::create_dir_all(&long_directory).unwrap();
    let long_cover = long_directory.join("pochette.png");
    image.save_png(&long_cover).unwrap();
    let long_cover = long_cover.canonicalize().unwrap();
    assert!(long_cover.as_os_str().len() > 260);
    let mut long_state = state.clone();
    let long_title = format!("{title} long cache path");
    let track = long_state.track.as_mut().unwrap();
    track.title = long_title.clone();
    track.cover = Some(long_cover.clone());
    desktop.publish(long_state, Language::French, Default::default());
    let session = find_session(&manager, &long_title);
    for event in desktop.events.try_iter() {
        assert!(!matches!(event, DesktopEvent::Warning(_)), "{event:?}");
    }
    assert_eq!(
        session
            .TryGetMediaPropertiesAsync()
            .unwrap()
            .get()
            .unwrap()
            .Thumbnail()
            .unwrap()
            .OpenReadAsync()
            .unwrap()
            .get()
            .unwrap()
            .Size()
            .unwrap(),
        std::fs::metadata(&long_cover).unwrap().len()
    );
    assert!(session.TryPauseAsync().unwrap().get().unwrap());
    wait_event(&desktop, |event| {
        matches!(event, DesktopEvent::Media(MediaCommand::Pause))
    });
    assert!(session.TrySkipNextAsync().unwrap().get().unwrap());
    wait_event(&desktop, |event| {
        matches!(event, DesktopEvent::Media(MediaCommand::Next))
    });
    assert!(
        session
            .TryChangePlaybackPositionAsync(25_000 * 10_000)
            .unwrap()
            .get()
            .unwrap()
    );
    wait_event(&desktop, |event| {
        matches!(event, DesktopEvent::Media(MediaCommand::Seek(25_000)))
    });

    // Track changes clear old artwork, including during paused playback.
    let mut without_cover = state.clone();
    without_cover.state = TransportState::Paused;
    let next_title = format!("{title} without artwork");
    let track = without_cover.track.as_mut().unwrap();
    track.title = next_title.clone();
    track.cover = None;
    desktop.publish(without_cover, Language::French, Default::default());
    let session = find_session(&manager, &next_title);
    assert!(
        session
            .TryGetMediaPropertiesAsync()
            .unwrap()
            .get()
            .unwrap()
            .Thumbnail()
            .is_err()
    );

    // Simulate Explorer announcing recreation: the icon is re-added, or Show is
    // sent as a safe fallback (without restarting the user's Explorer process).
    let tray = tray_window().expect("Tray window missing in the test process");
    let restart = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    unsafe {
        PostMessageW(
            Some(tray),
            restart,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        )
        .unwrap();
    }
    std::thread::sleep(Duration::from_millis(150));
    // Real tray left-click callback restores the app via the portable event.
    unsafe {
        PostMessageW(
            Some(tray),
            WM_APP + 6,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(windows::Win32::UI::Shell::NIN_SELECT as isize),
        )
        .unwrap();
    }
    wait_event(&desktop, |event| matches!(event, DesktopEvent::Show));
    // Keep the owner thread pumping while the background worker detaches.
    let cleanup = std::thread::spawn(move || desktop.shutdown());
    while !cleanup.is_finished() {
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    cleanup.join().unwrap();
    assert!(tray_window().is_none());
    drop(session);
    drop(manager);
    drop(window);
    unsafe {
        RoUninitialize();
    }
}
