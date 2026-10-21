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
        rap_audio::OutputDevice {
            name: "Echo Stereo".into(),
            selectable: true,
        },
        rap_audio::OutputDevice {
            name: "Duplicate".into(),
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
