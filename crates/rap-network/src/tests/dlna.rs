use super::*;

#[test]
fn soap_faults_and_format_negotiation_are_explicit() {
    let response = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetTransportInfoResponse xmlns:u="urn:schemas-upnp-org:service:AVTransport:1"><CurrentTransportState>PLAYING</CurrentTransportState></u:GetTransportInfoResponse></s:Body></s:Envelope>"#;
    assert_eq!(
        parse_response(response).unwrap()["CurrentTransportState"],
        "PLAYING"
    );
    let fault = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault><faultcode>s:Client</faultcode><detail><UPnPError><errorCode>714</errorCode><errorDescription>Illegal MIME-type</errorDescription></UPnPError></detail></s:Fault></s:Body></s:Envelope>"#;
    assert!(
        parse_response(fault)
            .unwrap_err()
            .to_string()
            .contains("714")
    );
    let sinks = vec![
        "http-get:*:audio/x-flac:*".into(),
        "http-get:*:audio/mpeg:*".into(),
    ];
    assert_eq!(
        negotiate("audio/flac", Some(&sinks)).unwrap(),
        "audio/x-flac"
    );
    assert!(negotiate("audio/aiff", Some(&sinks)).is_err());
    assert_eq!(negotiate("audio/wav", None).unwrap(), "audio/wav");
}
