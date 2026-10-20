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
