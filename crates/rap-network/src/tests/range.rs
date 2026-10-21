use super::*;

#[test]
fn byte_ranges_are_bounded_and_malformed_requests_are_rejected() {
    assert_eq!(parse("bytes=2-5", 10), Some((2, 5)));
    assert_eq!(parse("bytes=2-99", 10), Some((2, 9)));
    assert_eq!(parse("bytes=7-", 10), Some((7, 9)));
    assert_eq!(parse("bytes=-3", 10), Some((7, 9)));
    assert_eq!(parse("bytes=-99", 10), Some((0, 9)));
    for value in [
        "bytes=10-",
        "bytes=5-2",
        "bytes=-0",
        "bytes=a-4",
        "bytes=+1-4",
        "bytes=1-+4",
        "bytes=0-1,4-5",
        "items=0-1",
        "bytes=18446744073709551616-",
    ] {
        assert_eq!(parse(value, 10), None, "{value}");
    }
    assert_eq!(parse("bytes=0-", 0), None);
}
