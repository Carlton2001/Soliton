use super::*;

fn device(name: &str, label: &str) -> OutputDevice {
    OutputDevice::new(name.into(), label.into(), false)
}

#[test]
fn duplicate_names_are_never_used_as_device_identifiers() {
    let devices = describe(vec![
        device("DAC", "DAC"),
        device("Speakers", "Speakers"),
        device("DAC", "DAC"),
    ]);
    assert_eq!(
        devices,
        vec![
            OutputDevice {
                selectable: false,
                ..device("DAC", "DAC")
            },
            device("Speakers", "Speakers"),
        ]
    );
    assert!(describe(Vec::new()).is_empty());
}

#[test]
fn devices_are_sorted_by_label_and_identical_labels_stay_distinct() {
    let devices = describe(vec![
        device("usb-2", "USB DAC"),
        device("alsa", "Built-in"),
        device("usb-1", "USB DAC"),
    ]);
    assert_eq!(
        devices,
        vec![
            device("alsa", "Built-in"),
            device("usb-1", "USB DAC"),
            device("usb-2", "USB DAC")
        ]
    );
}
