use super::*;

const XML: &str = r#"<root xmlns="urn:schemas-upnp-org:device-1-0"><device>
<deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType>
<friendlyName>Salon &amp; musique</friendlyName><UDN>uuid:stable-renderer</UDN>
<serviceList><service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType><controlURL>/control/transport</controlURL></service>
<service><serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType><controlURL>control/volume</controlURL></service></serviceList>
</device></root>"#;

#[test]
fn renderer_discovery_resolves_relative_urls_without_leaving_the_peer() {
    let ip = "192.168.1.2".parse().unwrap();
    let url = Url::parse("http://192.168.1.2:1400/xml/device.xml").unwrap();
    let device = description(XML, &url, ip).unwrap();
    assert_eq!(device.name, "Salon & musique");
    assert_eq!(device.id, "upnp:uuid:stable-renderer");
    let services = device.services.unwrap();
    assert_eq!(
        services.transport.url,
        "http://192.168.1.2:1400/control/transport"
    );
    assert_eq!(
        services.rendering.unwrap().url,
        "http://192.168.1.2:1400/xml/control/volume"
    );
    for replacement in [
        "http://203.0.113.3/control",
        "http://127.0.0.1/control",
        "file:///etc/passwd",
        "http://someone@192.168.1.2/control",
    ] {
        assert!(description(&XML.replace("/control/transport", replacement), &url, ip).is_err());
    }
    assert!(description(&XML.replace("MediaRenderer", "MediaServer"), &url, ip).is_err());
}

#[test]
fn ssdp_headers_are_case_insensitive_and_non_successes_are_ignored() {
    assert_eq!(
        location(b"HTTP/1.1 200 OK\r\nLoCaTiOn: http://192.168.1.2/desc.xml\r\n\r\n").as_deref(),
        Some("http://192.168.1.2/desc.xml")
    );
    assert!(location(b"NOTIFY * HTTP/1.1\r\nLOCATION: http://192.168.1.2/desc.xml\r\n").is_none());
}

#[test]
fn url_base_is_honoured_only_on_the_announcing_device() {
    let ip = "192.168.1.2".parse().unwrap();
    let url = Url::parse("http://192.168.1.2/desc.xml").unwrap();
    let based = XML.replace(
        "<device>",
        "<URLBase>http://192.168.1.2:49152/upnp/</URLBase><device>",
    );
    let device = description(&based, &url, ip).unwrap();
    assert_eq!(
        device.services.unwrap().rendering.unwrap().url,
        "http://192.168.1.2:49152/upnp/control/volume"
    );
    // The device answers on the description's port, 80 when it names none.
    assert_eq!(device.address, "192.168.1.2:80".parse().unwrap());
    let elsewhere = XML.replace("<device>", "<URLBase>http://192.168.1.9/</URLBase><device>");
    assert!(description(&elsewhere, &url, ip).is_err());
}

#[test]
fn renderers_need_a_transport_and_a_bounded_identity() {
    let ip = "192.168.1.2".parse().unwrap();
    let url = Url::parse("http://192.168.1.2:1400/xml/device.xml").unwrap();
    let no_transport = XML.replace("service:AVTransport:1", "service:Other:1");
    assert!(description(&no_transport, &url, ip).is_err());
    let no_udn = XML.replace("<UDN>uuid:stable-renderer</UDN>", "");
    assert!(description(&no_udn, &url, ip).is_err());
    let empty_udn = XML.replace("uuid:stable-renderer", "");
    assert!(description(&empty_udn, &url, ip).is_err());
    let long_udn = XML.replace("uuid:stable-renderer", &"u".repeat(513));
    assert!(description(&long_udn, &url, ip).is_err());
    // Without a friendly name, the identity names the renderer.
    let unnamed = XML.replace("<friendlyName>Salon &amp; musique</friendlyName>", "");
    assert_eq!(
        description(&unnamed, &url, ip).unwrap().name,
        "uuid:stable-renderer"
    );
    let long_name = XML.replace("Salon &amp; musique", &"n".repeat(300));
    assert_eq!(
        description(&long_name, &url, ip)
            .unwrap()
            .name
            .chars()
            .count(),
        200
    );
    // Services missing a type or a control URL are skipped, not fatal.
    let partial = XML.replace(
        "<serviceList>",
        "<serviceList><service><controlURL>/x</controlURL></service><service><serviceType>urn:x</serviceType></service>",
    );
    assert!(description(&partial, &url, ip).is_ok());
}

#[test]
fn ssdp_replies_need_a_success_status_a_location_and_text() {
    assert!(
        location(b"HTTP/1.1 404 Not Found\r\nLOCATION: http://192.168.1.2/d.xml\r\n").is_none()
    );
    assert!(location(b"HTTP/1.1 200 OK\r\nST: upnp:rootdevice\r\n\r\n").is_none());
    assert!(location(b"\xff\xfe\x00").is_none());
    assert!(location(b"").is_none());
    for invalid in [
        "https://192.168.1.2/d.xml",
        "http://user:secret@192.168.1.2/d.xml",
        "http://renderer.local/d.xml",
        "not a url",
    ] {
        assert!(
            checked_url(invalid, "192.168.1.2".parse().unwrap()).is_err(),
            "{invalid}"
        );
    }
    assert!(
        checked_url(
            "http://192.168.1.2:8080/d.xml",
            "192.168.1.2".parse().unwrap()
        )
        .is_ok()
    );
}
