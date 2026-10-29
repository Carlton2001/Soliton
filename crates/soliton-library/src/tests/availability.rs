use super::*;

#[test]
fn only_a_root_emptied_on_its_own_volume_counts_as_deleted() {
    assert!(emptied_in_place(Some(42), Some(42)));
    // The mount point folder left on the parent volume after unmounting.
    assert!(!emptied_in_place(Some(42), Some(7)));
    // Never seen with music, or no volume identifier on this system.
    assert!(!emptied_in_place(None, Some(42)));
    assert!(!emptied_in_place(None, None));
}

#[cfg(unix)]
#[test]
fn a_folder_and_its_parent_share_a_volume() {
    let dir = tempfile::tempdir().unwrap();
    let child = dir.path().join("music");
    std::fs::create_dir(&child).unwrap();
    assert!(volume(&child).is_some());
    assert_eq!(volume(&child), volume(dir.path()));
    assert_eq!(volume(&dir.path().join("missing")), None);
}
