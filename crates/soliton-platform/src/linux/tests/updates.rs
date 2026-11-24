use super::*;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    thread,
    time::{Duration, Instant},
};

#[test]
fn only_the_archive_installation_is_eligible_for_self_update() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let executable = home.join(".local/bin/soliton");
    let data = temp.path().join("data");
    let entry = data
        .join("applications")
        .join(format!("{}.desktop", crate::APP_ID));
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::create_dir_all(entry.parent().unwrap()).unwrap();
    fs::write(&executable, b"Soliton").unwrap();
    let running = executable.canonicalize().unwrap();
    assert!(archive_executable(&running, &home, &data).is_none());

    fs::write(&entry, super::super::launcher_icon::MARKER).unwrap();
    let home_link = temp.path().join("home-link");
    symlink(&home, &home_link).unwrap();
    assert_eq!(
        archive_executable(&running, &home_link, &data),
        Some(home_link.join(".local/bin/soliton")),
    );
    assert!(archive_executable(Path::new("/usr/bin/soliton"), &home, &data).is_none());
    assert!(archive_executable(Path::new("/app/bin/soliton"), &home, &data).is_none());

    let elsewhere = temp.path().join("elsewhere");
    fs::rename(&executable, &elsewhere).unwrap();
    symlink(&elsewhere, &executable).unwrap();
    assert!(archive_executable(&elsewhere, &home, &data).is_none());
}

#[test]
fn installs_the_script_from_the_expected_archive_directory_and_reports_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = "Soliton-1.2.3-x86_64-linux";
    let source = temp.path().join("source");
    let destination = temp.path().join("staged");
    fs::create_dir_all(source.join(root)).unwrap();
    fs::create_dir(&destination).unwrap();
    let script = source.join(root).join("install.sh");
    let archive = destination.join(format!("{root}.tar.gz"));
    let pack = || {
        assert!(
            Command::new("tar")
                .arg("-czf")
                .arg(&archive)
                .arg("-C")
                .arg(&source)
                .arg(root)
                .status()
                .unwrap()
                .success()
        );
    };

    fs::write(&script, "[ \"$1\" = --update ] && [ \"$2\" = 123 ]").unwrap();
    pack();
    install_archive(&archive, &destination, 123).unwrap();

    fs::write(&script, "echo 'installation failed' >&2; exit 3").unwrap();
    pack();
    let error = install_archive(&archive, &destination, 123).unwrap_err();
    assert!(error.to_string().contains("installation failed"));
}

#[test]
fn relaunch_waits_for_the_previous_process_to_exit() {
    let temp = tempfile::tempdir().unwrap();
    let executable = temp.path().join("new soliton");
    let launched = temp.path().join("launched");
    fs::write(
        &executable,
        "#!/bin/sh\ntouch -- \"$(dirname -- \"$0\")/launched\"\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();

    let mut previous = Command::new("sleep").arg("30").spawn().unwrap();
    let result = relaunch_after(previous.id(), &executable);
    thread::sleep(Duration::from_millis(300));
    let started_early = launched.exists();
    previous.kill().unwrap();
    previous.wait().unwrap();
    result.unwrap();
    assert!(!started_early);

    let deadline = Instant::now() + Duration::from_secs(5);
    while !launched.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(launched.exists());
}
