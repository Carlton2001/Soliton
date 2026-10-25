use super::*;

#[test]
fn icon_defaults_for_older_settings_and_all_palettes_persist_independently() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    assert_eq!(
        Settings::load(&path).unwrap().app_icon,
        AppIcon::GraphiteMint
    );
    fs::write(&path, br#"{"accent":"Red","appearance":"Light"}"#).unwrap();
    let mut settings = Settings::load(&path).unwrap();
    assert_eq!(settings.app_icon, AppIcon::GraphiteMint);
    for app_icon in AppIcon::ALL {
        settings.app_icon = app_icon;
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.app_icon, app_icon);
        assert_eq!(loaded.accent, Accent::Red);
        assert_eq!(loaded.appearance, Appearance::Light);
    }
}

#[test]
fn settings_roundtrip_and_missing_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    assert_eq!(Settings::load(&path).unwrap().language, Language::French);
    let settings = Settings {
        language: Language::English,
        volume: 0.3,
        ..Settings::default()
    };
    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path).unwrap().language, Language::English);
    assert_eq!(Settings::load(&path).unwrap().volume, 0.3);
}

#[test]
fn output_preference_roundtrips_and_older_settings_use_system_default() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, br#"{"volume":0.4}"#).unwrap();
    assert_eq!(Settings::load(&path).unwrap().output_device, None);
    let mut settings = Settings::load(&path).unwrap();
    settings.output_device = Some("DAC USB — Casque".into());
    settings.save(&path).unwrap();
    assert_eq!(
        Settings::load(&path).unwrap().output_device,
        settings.output_device
    );
    settings.output_device = None;
    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path).unwrap().output_device, None);
}

#[test]
fn playback_modes_round_trip_and_older_settings_turn_them_off() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, br#"{"volume":0.4}"#).unwrap();
    let mut settings = Settings::load(&path).unwrap();
    assert!(!settings.shuffle);
    assert_eq!(settings.repeat, Repeat::Off);
    for repeat in [Repeat::Track, Repeat::All] {
        settings.shuffle = true;
        settings.repeat = repeat;
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert!(loaded.shuffle);
        assert_eq!(loaded.repeat, repeat);
    }
}

#[test]
fn invalid_settings_are_reported() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, b"broken").unwrap();
    assert!(Settings::load(&path).is_err());
}

#[test]
fn closing_defaults_to_quit_and_all_choices_persist() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, br#"{"volume":0.4}"#).unwrap();
    assert_eq!(
        Settings::load(&path).unwrap().close_behavior,
        CloseBehavior::Quit
    );
    for close_behavior in [CloseBehavior::Quit, CloseBehavior::Hide, CloseBehavior::Ask] {
        Settings {
            close_behavior,
            ..Default::default()
        }
        .save(&path)
        .unwrap();
        assert_eq!(
            Settings::load(&path).unwrap().close_behavior,
            close_behavior
        );
    }
}

#[test]
fn earlier_theme_choice_is_kept() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(
        &path,
        br#"{"language":"English","dark":false,"album_grid":false}"#,
    )
    .unwrap();
    let settings = Settings::load(&path).unwrap();
    assert_eq!(settings.appearance, Appearance::Light);
    assert!(!settings.album_grid);
    assert_eq!(settings.accent, Accent::Teal);
    settings.save(&path).unwrap();
    // Once saved, the new field wins over the stale flag.
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("\"appearance\": \"Light\"") && !saved.contains("dark"));
    fs::write(&path, br#"{"dark":true}"#).unwrap();
    assert_eq!(Settings::load(&path).unwrap().appearance, Appearance::Dark);
    fs::write(&path, br#"{}"#).unwrap();
    assert_eq!(
        Settings::load(&path).unwrap().appearance,
        Appearance::System
    );
}

#[test]
fn interface_size_defaults_and_stays_in_range() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, br#"{}"#).unwrap();
    assert_eq!(Settings::load(&path).unwrap().ui_rem_size, 16);
    fs::write(&path, br#"{"ui_rem_size":100}"#).unwrap();
    assert_eq!(Settings::load(&path).unwrap().ui_rem_size, 24);
    fs::write(&path, br#"{"ui_rem_size":0}"#).unwrap();
    assert_eq!(Settings::load(&path).unwrap().ui_rem_size, 12);
}

#[test]
fn last_view_and_artist_layout_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, br#"{}"#).unwrap();
    let mut settings = Settings::load(&path).unwrap();
    assert!(settings.artist_grid);
    assert_eq!(settings.last_view, LastView::default());
    settings.artist_grid = false;
    settings.last_queue = LastQueue {
        playlist_entries: vec![Some(10), Some(11), None],
        current_index: Some(1),
        tracks: vec![3, 1, 2],
        current: Some(1),
        position_ms: 61_000,
    };
    settings.last_view = LastView {
        playlist: None,
        view: View::Artists,
        album: Some(42),
        artist: Some("Björk".into()),
    };
    settings.save(&path).unwrap();
    let loaded = Settings::load(&path).unwrap();
    assert!(!loaded.artist_grid);
    assert_eq!(loaded.last_view, settings.last_view);
    assert_eq!(loaded.last_queue, settings.last_queue);
}

#[test]
fn repeated_queue_occurrences_and_playlist_views_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let settings = Settings {
        last_queue: LastQueue {
            playlist_entries: vec![Some(10), Some(11), None],
            tracks: vec![3, 3, 4],
            current: Some(3),
            current_index: Some(1),
            position_ms: 4000,
        },
        last_view: LastView {
            view: View::Playlists,
            playlist: Some(9),
            ..Default::default()
        },
        ..Default::default()
    };
    settings.save(&path).unwrap();
    let saved = Settings::load(&path).unwrap();
    assert_eq!(saved.last_queue, settings.last_queue);
    assert_eq!(saved.last_view, settings.last_view);
    fs::write(&path, br#"{"last_queue":{"tracks":[3,3],"current":3}}"#).unwrap();
    assert_eq!(
        Settings::load(&path).unwrap().last_queue.current_index,
        None
    );
}

#[test]
fn home_artist_and_album_round_trip() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let settings = Settings {
        last_view: LastView {
            view: View::Home,
            artist: Some("Björk".into()),
            album: Some(42),
            playlist: None,
        },
        ..Default::default()
    };
    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path).unwrap().last_view, settings.last_view);
}

#[test]
fn system_accent_and_appearance_persist_independently() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    for appearance in [Appearance::System, Appearance::Light, Appearance::Dark] {
        let settings = Settings {
            appearance,
            accent: Accent::System,
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.accent, Accent::System);
        assert_eq!(loaded.appearance, appearance);
    }
}
