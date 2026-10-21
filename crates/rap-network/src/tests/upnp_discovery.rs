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
