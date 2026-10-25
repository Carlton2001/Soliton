use super::*;

#[test]
fn portable_storage_requires_a_data_directory_next_to_the_executable() {
    let root = tempfile::tempdir().unwrap();
    let exe = root.path().join("Soliton.exe");
    assert!(portable_directory(&exe).is_none());
    std::fs::write(root.path().join("data"), b"not a directory").unwrap();
    assert!(portable_directory(&exe).is_none());
    std::fs::remove_file(root.path().join("data")).unwrap();
    std::fs::create_dir(root.path().join("data")).unwrap();
    assert_eq!(portable_directory(&exe), Some(root.path().join("data")));
    let paths = AppPaths::at(portable_directory(&exe).unwrap());
    paths.create().unwrap();
    assert!(paths.covers.is_dir());
    assert!(paths.database.starts_with(root.path().join("data")));
    assert!(paths.settings.starts_with(root.path().join("data")));
}
