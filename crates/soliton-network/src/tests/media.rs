use super::*;

#[test]
fn xml_and_upnp_durations_handle_metadata_without_markup_injection() {
    assert_eq!(
        xml("Été & <album> \"live\" 'mix'"),
        "Été &amp; &lt;album&gt; &quot;live&quot; &apos;mix&apos;"
    );
    assert_eq!(duration(3_723_999), "1:02:03");
    assert_eq!(milliseconds("1:02:03.5"), Some(3_723_500));
    for invalid in [
        "NOT_IMPLEMENTED",
        "0:60:00",
        "0:00:NaN",
        "0:00:inf",
        "0:00:-1",
        "0:00:60",
        "0:00:01:02",
        "999999999999999999:00:00",
    ] {
        assert_eq!(milliseconds(invalid), None, "{invalid}");
    }
}
