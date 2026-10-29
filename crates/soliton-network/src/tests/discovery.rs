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

#[test]
fn only_private_and_link_local_addresses_are_local() {
    for ip in ["10.0.0.1", "172.16.5.4", "192.168.1.2", "169.254.10.20"] {
        assert!(local(ip.parse().unwrap()), "{ip}");
    }
    for ip in [
        "127.0.0.1",
        "8.8.8.8",
        "203.0.113.5",
        "0.0.0.0",
        "255.255.255.255",
    ] {
        assert!(!local(ip.parse().unwrap()), "{ip}");
    }
}

#[test]
fn cast_receivers_without_identity_or_port_fall_back_or_are_ignored() {
    let resolved = |port, properties: &[(&str, &str)]| {
        mdns_sd::ServiceInfo::new(
            CAST,
            "device",
            "device.local.",
            "192.168.1.42",
            port,
            properties,
        )
        .unwrap()
        .as_resolved_service()
    };
    assert!(cast_device(&resolved(0, &[])).is_none());
    let device = cast_device(&resolved(8009, &[("id", ""), ("fn", "")])).unwrap();
    assert_eq!(device.id, format!("cast:device.{CAST}"));
    assert_eq!(device.name, format!("device.{CAST}"));
    assert_eq!(device.address, "192.168.1.42:8009".parse().unwrap());
    // TXT values stop at 255 bytes, still longer than shown names.
    let long = "n".repeat(250);
    let device = cast_device(&resolved(8009, &[("fn", long.as_str())])).unwrap();
    assert_eq!(device.name.chars().count(), 200);
}
