use super::*;

// Public prehashed test vector from minisign-verify 0.2.5 (MIT), not a RAP key.
const KEY: &str = "untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
const SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1556193335\tfile:test\ny/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";

fn encode(text: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(text)
}

#[test]
fn only_verified_bytes_are_staged_and_dropped_preparations_clean_up() {
    let prepared = stage(b"test", &encode(SIGNATURE), &encode(KEY)).unwrap();
    assert_eq!(std::fs::read(&prepared.setup).unwrap(), b"test");
    let directory = prepared.directory.path().to_owned();
    drop(prepared);
    assert!(!directory.exists());
    assert!(stage(b"Test", &encode(SIGNATURE), &encode(KEY)).is_err());
}

#[test]
fn invalid_keys_and_signatures_never_stage_an_installer() {
    assert!(stage(b"test", "invalid", &encode(KEY)).is_err());
    assert!(stage(b"test", &encode(SIGNATURE), "invalid").is_err());
    assert!(stage(b"test", "", "").is_err());
}
