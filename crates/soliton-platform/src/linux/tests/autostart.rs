use super::*;

#[test]
fn exec_quotes_spaces_unicode_and_reserved_characters_without_a_shell() {
    assert_eq!(
        exec(Path::new("/home/moi/Musique été/soliton")).unwrap(),
        r#""/home/moi/Musique été/soliton""#
    );
    // `$` and `"` are escaped for the Exec quoting, then `\` for the string value.
    assert_eq!(
        exec(Path::new(r#"/opt/a$b"c\d 100%/soliton"#)).unwrap(),
        r#""/opt/a\\$b\\"c\\\\d 100%%/soliton""#
    );
    assert!(exec(Path::new("/opt/bad\npath")).is_err());
}

#[test]
fn entry_is_written_compared_and_removed_idempotently() {
    let config = tempfile::tempdir().unwrap();
    let file = file(config.path());
    let expected = entry(Path::new("/home/moi/.local/bin/soliton")).unwrap();
    assert!(!registered(&file, &expected).unwrap());
    configure(&file, Some(expected.clone())).unwrap();
    configure(&file, Some(expected.clone())).unwrap();
    assert!(registered(&file, &expected).unwrap());
    assert!(expected.contains(&format!("\nIcon={}\n", crate::APP_ID)));
    // A moved executable is reported as not registered until enabled again.
    let moved = entry(Path::new("/opt/soliton/soliton")).unwrap();
    assert!(!registered(&file, &moved).unwrap());
    let other = file.with_file_name("other.desktop");
    std::fs::write(&other, "[Desktop Entry]\n").unwrap();
    configure(&file, None).unwrap();
    configure(&file, None).unwrap();
    assert!(!file.exists());
    assert!(other.exists());
}
