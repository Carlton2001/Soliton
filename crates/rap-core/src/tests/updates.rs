use super::*;

fn manifest(version: &str) -> Vec<u8> {
    let numeric = version.strip_prefix('v').unwrap_or(version);
    serde_json::to_vec(&serde_json::json!({
        "version": version,
        "platforms": { "windows-x86_64": {
            "url": format!("https://github.com/owner/RAP/releases/download/v{numeric}/RAP_{numeric}_x64-setup.exe"),
            "signature": "signed-asset"
        }}
    })).unwrap()
}

#[test]
fn versions_are_compared_numerically_and_never_downgrade() {
    let manifest = manifest("0.10.0");
    assert_eq!(
        parse(&manifest, "0.9.9", "owner/RAP")
            .unwrap()
            .unwrap()
            .version,
        "0.10.0"
    );
    assert!(parse(&manifest, "0.10.0", "owner/RAP").unwrap().is_none());
    assert!(parse(&manifest, "1.0.0", "owner/RAP").unwrap().is_none());
}

#[test]
fn only_a_release_page_in_the_configured_repository_can_be_opened() {
    let release = parse(&manifest("v1.2.3"), "0.1.0", "owner/RAP")
        .unwrap()
        .unwrap();
    assert_eq!(
        release.page,
        "https://github.com/owner/RAP/releases/tag/v1.2.3"
    );
    for candidate in [
        "../RAP",
        "owner/RAP/extra",
        "owner/RAP?redirect=evil",
        "/RAP",
        "owner/",
    ] {
        assert!(!valid_repository(candidate));
    }
}

#[test]
fn unsigned_foreign_or_mismatched_installers_are_rejected() {
    let text = String::from_utf8(manifest("1.2.3")).unwrap();
    for altered in [
        text.replace(
            "https://github.com/owner/RAP/",
            "https://evil.example/owner/RAP/",
        ),
        text.replace("RAP_1.2.3_x64-setup.exe", "RAP_1.2.2_x64-setup.exe"),
        text.replace("signed-asset", ""),
        text.replace("windows-x86_64", "linux-x86_64"),
        r#"{"version":"1.2.3"}"#.to_owned(),
    ] {
        assert!(parse(altered.as_bytes(), "0.1.0", "owner/RAP").is_err());
    }
}

#[test]
fn malformed_or_oversized_manifests_and_prereleases_are_rejected() {
    for manifest in [
        br#"{}"#.as_slice(),
        br#"{"version":"1.0.0-beta.1"}"#,
        br#"{"version":"1.0"}"#,
        br#"{"version":"1.0.+2"}"#,
        b"not JSON",
    ] {
        assert!(parse(manifest, "0.1.0", "owner/RAP").is_err());
    }
    assert!(parse(&vec![b' '; LIMIT + 1], "0.1.0", "owner/RAP").is_err());
}
