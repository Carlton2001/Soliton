use super::*;

fn install(data: &Path, entry: &str) {
    let applications = data.join("applications");
    std::fs::create_dir_all(&applications).unwrap();
    std::fs::write(
        applications.join(format!("{}.desktop", crate::APP_ID)),
        entry,
    )
    .unwrap();
}

#[test]
fn icons_installed_by_other_means_are_never_modified() {
    let data = tempfile::tempdir().unwrap();
    update(data.path(), AppIcon::Blue).unwrap();
    assert!(!icon_file(data.path()).exists());
    install(data.path(), "[Desktop Entry]\nName=Soliton\n");
    std::fs::create_dir_all(icon_file(data.path()).parent().unwrap()).unwrap();
    std::fs::write(icon_file(data.path()), b"packaged").unwrap();
    update(data.path(), AppIcon::Blue).unwrap();
    assert_eq!(std::fs::read(icon_file(data.path())).unwrap(), b"packaged");
}

#[test]
fn script_installed_icon_follows_the_palette() {
    let data = tempfile::tempdir().unwrap();
    install(
        data.path(),
        &format!("[Desktop Entry]\nName=Soliton\n{MARKER}\n"),
    );
    update(data.path(), AppIcon::Copper).unwrap();
    assert_eq!(
        std::fs::read(icon_file(data.path())).unwrap(),
        svg(AppIcon::Copper)
    );
    update(data.path(), AppIcon::IvoryInk).unwrap();
    assert_eq!(
        std::fs::read(icon_file(data.path())).unwrap(),
        svg(AppIcon::IvoryInk)
    );
    let files: Vec<_> = std::fs::read_dir(icon_file(data.path()).parent().unwrap())
        .unwrap()
        .collect();
    assert_eq!(files.len(), 1, "no temporary file is left behind");
}
