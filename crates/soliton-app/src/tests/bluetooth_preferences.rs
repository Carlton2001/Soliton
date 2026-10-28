use super::{BluetoothDevice, ready_outputs};

#[test]
fn only_native_associations_with_unambiguous_active_outputs_are_selectable() {
    let mut device = BluetoothDevice {
        id: "paired".into(),
        name: "Echo".into(),
        connected: false,
        outputs: vec!["Echo Stereo".into(), "Duplicate".into(), "Missing".into()],
    };
    let outputs = vec![
        soliton_audio::OutputDevice {
            name: "Echo Stereo".into(),
            label: "Echo Stereo".into(),
            bluetooth: true,
            selectable: true,
        },
        soliton_audio::OutputDevice {
            name: "Duplicate".into(),
            label: "Duplicate".into(),
            bluetooth: true,
            selectable: false,
        },
    ];
    assert_eq!(ready_outputs(&device, &outputs), ["Echo Stereo"]);
    device.outputs.clear();
    device.name = "Echo Stereo".into();
    assert!(
        ready_outputs(&device, &outputs).is_empty(),
        "A friendly-name match is not an association"
    );
}
