use super::*;
use std::time::SystemTime;

#[test]
fn prunes_only_unreferenced_cache_entries() {
    let dir = tempfile::tempdir().unwrap();
    let hash = |c: char| c.to_string().repeat(64);
    let kept = dir.path().join(format!("{}.png", hash('a')));
    let touch = |name: String| {
        let path = dir.path().join(name);
        std::fs::write(&path, b"x").unwrap();
        path
    };
    touch(format!("{}.png", hash('a')));
    let kept_small = touch(format!("{}.small.png", hash('a')));
    let gone = touch(format!("{}.png", hash('b')));
    let gone_small = touch(format!("{}.small.png", hash('b')));
    let fresh_tmp = touch(format!("{}.png.1.2.tmp", hash('c')));
    let old_tmp = touch(format!("{}.small.png.1.3.tmp", hash('c')));
    std::fs::File::options()
        .write(true)
        .open(&old_tmp)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(7200))
        .unwrap();
    let foreign = touch("notes.png".into());
    let removed = prune(dir.path(), &HashSet::from([kept.clone()])).unwrap();
    assert_eq!(removed, 3);
    for path in [&kept, &kept_small, &fresh_tmp, &foreign] {
        assert!(path.exists(), "{}", path.display());
    }
    for path in [&gone, &gone_small, &old_tmp] {
        assert!(!path.exists(), "{}", path.display());
    }
    assert!(complete(&kept));
    assert!(!complete(&gone));
}
