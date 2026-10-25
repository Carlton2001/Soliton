use super::*;

#[test]
fn startup_command_preserves_spaces_unicode_and_no_shell() {
    assert_eq!(
        command(Path::new(r"C:\Musique été\Soliton.exe")).unwrap(),
        r#""C:\Musique été\Soliton.exe""#
    );
    assert!(command(Path::new("bad\"path")).is_err());
    assert!(command(Path::new(&"x".repeat(260))).is_err());
}

#[test]
#[ignore = "Writes an isolated temporary HKCU test key; never changes the Run key"]
fn registration_is_idempotent_and_preserves_other_values() {
    let path = format!(r"Software\Soliton-Test-{}", std::process::id());
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = CURRENT_USER.remove_tree(&self.0);
        }
    }
    let _cleanup = Cleanup(path.clone());
    let key = CURRENT_USER.create(&path).unwrap();
    key.set_string("Other app", "preserve me").unwrap();
    let executable = command(Path::new(r"C:\Program Files\Soliton été\Soliton.exe")).unwrap();
    assert!(!registered(&key, &executable).unwrap());
    for _ in 0..2 {
        configure(&key, Some(executable.clone())).unwrap();
        assert!(registered(&key, &executable).unwrap());
    }
    assert!(!registered(&key, "another executable").unwrap());
    for _ in 0..2 {
        configure(&key, None).unwrap();
        assert!(!registered(&key, &executable).unwrap());
    }
    assert_eq!(key.get_string("Other app").unwrap(), "preserve me");
}
