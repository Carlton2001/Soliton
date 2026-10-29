use super::*;

#[test]
fn portal_paths_split_into_document_and_subpath() {
    let mount = Path::new("/run/flatpak/doc");
    let (id, rest) = split(Path::new("/run/flatpak/doc/CiwhDJ/Musique/Jazz"), mount).unwrap();
    assert_eq!(id, "CiwhDJ");
    assert_eq!(rest, Path::new("Jazz"));
    let (_, rest) = split(Path::new("/run/flatpak/doc/DZWoSy/_FLAC"), mount).unwrap();
    assert_eq!(rest, Path::new(""));
    assert!(split(Path::new("/home/user/Musique"), mount).is_none());
    assert!(split(mount, mount).is_none());
    assert!(split(&mount.join("CiwhDJ"), mount).is_none());
}

#[test]
fn paths_outside_the_portal_are_kept() {
    let path = PathBuf::from("/home/user/Musique");
    assert_eq!(host_path(path.clone()), path);
}
