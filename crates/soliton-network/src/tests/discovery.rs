use super::*;

#[test]
fn cast_discovery_accepts_every_brand_and_uses_stable_identity() {
    for name in ["SHIELD", "Kitchen speaker", "Other Cast TV"] {
        let info = mdns_sd::ServiceInfo::new(
            CAST,
            "device",
            "device.local.",
            "192.168.1.42",
            8009,
            [("id", "persistent-id"), ("fn", name)].as_slice(),
        )
        .unwrap()
        .as_resolved_service();
        let device = cast_device(&info).unwrap();
        assert_eq!(device.id, "cast:persistent-id");
        assert_eq!(device.name, name);
        assert_eq!(device.protocol, Protocol::Cast);
    }
    let info =
        mdns_sd::ServiceInfo::new(CAST, "device", "device.local.", "203.0.113.5", 8009, None)
            .unwrap()
            .as_resolved_service();
    assert!(cast_device(&info).is_none());
}
