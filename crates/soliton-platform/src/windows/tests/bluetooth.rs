use super::*;

#[test]
fn routes_by_container_and_preserves_disconnected_namesakes() {
    let first = GUID::from_u128(1);
    let second = GUID::from_u128(2);
    let devices = vec![
        Device {
            id: "first".into(),
            name: "Echo".into(),
            container: Some(first),
            connected: true,
        },
        Device {
            id: "second".into(),
            name: "Echo".into(),
            container: Some(second),
            connected: false,
        },
        Device {
            id: "unknown".into(),
            name: "Headphones".into(),
            container: None,
            connected: false,
        },
    ];
    let endpoints = vec![
        Endpoint {
            name: "Speakers (Echo)".into(),
            container: Some(first),
        },
        Endpoint {
            name: "Speakers (Echo)".into(),
            container: Some(first),
        },
        Endpoint {
            name: "Unknown".into(),
            container: None,
        },
    ];
    let result = associate(devices, endpoints);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].outputs, ["Speakers (Echo)"]);
    assert!(result[1].outputs.is_empty());
    assert!(!result[1].connected);
    assert!(result[2].outputs.is_empty());
}

#[test]
fn zero_container_never_routes_to_another_unknown_device() {
    let result = associate(
        vec![Device {
            id: "one".into(),
            name: "Echo".into(),
            container: Some(GUID::zeroed()),
            connected: true,
        }],
        vec![Endpoint {
            name: "Other speakers".into(),
            container: Some(GUID::zeroed()),
        }],
    );
    assert!(result[0].outputs.is_empty());
}
