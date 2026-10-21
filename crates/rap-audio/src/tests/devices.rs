use super::*;

#[test]
fn duplicate_names_are_never_used_as_device_identifiers() {
    let devices = describe(vec!["DAC".into(), "Speakers".into(), "DAC".into()]);
    assert_eq!(
        devices,
        vec![
            OutputDevice {
                name: "DAC".into(),
                selectable: false
            },
            OutputDevice {
                name: "Speakers".into(),
                selectable: true
            },
        ]
    );
    assert!(describe(Vec::new()).is_empty());
}
