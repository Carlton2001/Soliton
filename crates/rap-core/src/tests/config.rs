use super::*;

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
fn invalid_settings_are_reported() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    fs::write(&path, b"broken").unwrap();
    assert!(Settings::load(&path).is_err());
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
