use super::*;

// Public prehashed test vector from minisign-verify 0.2.5 (MIT), not a Soliton key.
const KEY: &str = "untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
const SIGNATURE: &str = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1556193335\tfile:test\ny/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";

// Throwaway key made for these tests (Ed25519 seed of 7s), not a Soliton key;
// it signs b"test" with the trusted comment "soliton-version:1.2.3".
const TEST_KEY: &str = "untrusted comment: test key
RWQBAgMEBQYHCOpKbGPinFIKvvVQexMuxfmVR3auvr57kkIe6mkURtIs";
const VERSIONED: &str = "untrusted comment: test signature
RUQBAgMEBQYHCBh0od4foi9vkvb4iisNrDD8+mHIhwhSI7T4vRrHtGY5mLCnK5qqaRTd29Unl3vY9jMAiPhE5/iowuCEI5mb8A0=
trusted comment: soliton-version:1.2.3
MuL06LI6Kj4TACfnEXIChfxJ9jA9wCONt1LZ65vW+qHWDD6UNV0lml2fzZaT/sMEr83I+xLkan4l34TT2x9zAg==";

fn encode(text: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(text)
}

#[test]
fn only_verified_bytes_are_staged_and_dropped_preparations_clean_up() {
    let prepared = stage(b"test", &encode(VERSIONED), &encode(TEST_KEY), "1.2.3").unwrap();
    assert_eq!(std::fs::read(&prepared.setup).unwrap(), b"test");
    let directory = prepared.directory.path().to_owned();
    drop(prepared);
    assert!(!directory.exists());
    assert!(stage(b"Test", &encode(VERSIONED), &encode(TEST_KEY), "1.2.3").is_err());
}

#[test]
fn a_valid_signature_for_another_version_is_refused() {
    assert!(stage(b"test", &encode(VERSIONED), &encode(TEST_KEY), "1.2.4").is_err());
    assert!(stage(b"test", &encode(VERSIONED), &encode(TEST_KEY), "1.2").is_err());
    // Correct bytes and key, but a trusted comment naming no version.
    assert!(stage(b"test", &encode(SIGNATURE), &encode(KEY), "1.2.3").is_err());
    assert!(
        verify(b"test", &encode(SIGNATURE), &encode(KEY), "1.2.3")
            .unwrap_err()
            .contains("1.2.3")
    );
}

#[test]
fn invalid_keys_and_signatures_never_stage_an_installer() {
    assert!(stage(b"test", "invalid", &encode(TEST_KEY), "1.2.3").is_err());
    assert!(stage(b"test", &encode(VERSIONED), "invalid", "1.2.3").is_err());
    assert!(stage(b"test", "", "", "1.2.3").is_err());
}
