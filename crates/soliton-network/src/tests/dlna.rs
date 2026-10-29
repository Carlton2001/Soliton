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

#[test]
fn responses_without_an_action_result_or_well_formed_xml_are_errors() {
    assert!(parse_response("<not closed").is_err());
    let empty =
        r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body/></s:Envelope>"#;
    assert!(parse_response(empty).is_err());
    let values = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetPositionInfoResponse xmlns:u="urn:schemas-upnp-org:service:AVTransport:1"><TrackURI/><RelTime>0:00:05</RelTime></u:GetPositionInfoResponse></s:Body></s:Envelope>"#;
    let values = parse_response(values).unwrap();
    assert_eq!(values["TrackURI"], "");
    assert_eq!(values["RelTime"], "0:00:05");
}

#[test]
fn negotiation_takes_the_first_http_sink_accepting_the_format() {
    let sinks = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // Other transports are never chosen, even when they list the format.
    assert_eq!(
        negotiate(
            "audio/mpeg",
            Some(&sinks(&["rtsp-rtp-udp:*:audio/mpeg:*", "HTTP-GET:*:*:*"]))
        )
        .unwrap(),
        "audio/mpeg"
    );
    assert!(negotiate("audio/mpeg", Some(&sinks(&["rtsp-rtp-udp:*:audio/mpeg:*"]))).is_err());
    assert_eq!(
        negotiate("audio/wav", Some(&sinks(&["http-get:*:audio/wave:*"]))).unwrap(),
        "audio/wave"
    );
    assert_eq!(
        negotiate(
            "audio/aiff",
            Some(&sinks(&[
                "http-get:*:audio/x-aiff:*",
                "http-get:*:audio/aiff:*"
            ]))
        )
        .unwrap(),
        "audio/x-aiff"
    );
    // A renderer listing nothing is trusted with the original format.
    assert_eq!(negotiate("audio/ogg", Some(&[])).unwrap(), "audio/ogg");
    let error = negotiate("audio/ogg", Some(&sinks(&["http-get:*:audio/mpeg:*"]))).unwrap_err();
    assert!(error.to_string().contains("audio/ogg"));
}

#[test]
fn didl_metadata_escapes_every_tag_and_names_the_negotiated_format() {
    let track = Track {
        id: 1,
        source_id: 1,
        path: "a.flac".into(),
        metadata: soliton_core::model::Metadata {
            title: "Rock & <roll>".into(),
            artist: "\"Quoted\"".into(),
            album: "L'album".into(),
            duration_ms: 61_000,
            ..Default::default()
        },
        cover: None,
        available: true,
    };
    let urls = MediaUrls {
        audio: "http://192.168.1.2:1234/token/audio?a=1&b=2".into(),
        cover: None,
        mime: "audio/x-flac".into(),
    };
    let didl = metadata(&track, &urls);
    assert!(didl.contains("<dc:title>Rock &amp; &lt;roll&gt;</dc:title>"));
    assert!(didl.contains("<upnp:artist>&quot;Quoted&quot;</upnp:artist>"));
    assert!(didl.contains("<upnp:album>L&apos;album</upnp:album>"));
    assert!(didl.contains("protocolInfo=\"http-get:*:audio/x-flac:*\" duration=\"0:01:01\""));
    assert!(didl.contains(">http://192.168.1.2:1234/token/audio?a=1&amp;b=2</res>"));
    assert!(!didl.contains("albumArtURI"));
    roxmltree::Document::parse(&didl).unwrap();
    let with_cover = MediaUrls {
        cover: Some("http://192.168.1.2:1234/token/cover".into()),
        ..urls
    };
    assert!(
        metadata(&track, &with_cover)
            .contains("<upnp:albumArtURI>http://192.168.1.2:1234/token/cover</upnp:albumArtURI>")
    );
}

mod session {
    use super::super::*;
    use crate::{
        http::MediaUrls,
        test_renderer::{Mock, track},
    };

    fn urls(uri: &str) -> MediaUrls {
        MediaUrls {
            audio: uri.into(),
            cover: None,
            mime: "audio/wav".into(),
        }
    }

    async fn loaded(mock: &Mock, full: bool, autoplay: bool) -> Session {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::connect(&mock.device(full)).await.unwrap();
        session
            .load(
                &track(dir.path()),
                &urls("http://127.0.0.1:9/token/audio"),
                0,
                autoplay,
                0.5,
            )
            .await
            .unwrap();
        session
    }

    async fn state(mock: &Mock, session: &mut Session, transport: &str) -> anyhow::Result<State> {
        mock.renderer.lock().await.state = transport.into();
        session.poll().await.map(|status| status.state)
    }

    #[tokio::test]
    async fn transport_states_map_to_playback_states() {
        let mock = Mock::start().await;
        let mut session = loaded(&mock, true, true).await;
        assert_eq!(
            state(&mock, &mut session, "TRANSITIONING").await.unwrap(),
            State::Buffering
        );
        assert_eq!(
            state(&mock, &mut session, "PAUSED_PLAYBACK").await.unwrap(),
            State::Paused
        );
        assert_eq!(
            state(&mock, &mut session, "PLAYING").await.unwrap(),
            State::Playing
        );
        // Stopped well before the end: not a finished track.
        assert_eq!(
            state(&mock, &mut session, "STOPPED").await.unwrap(),
            State::Stopped
        );
        assert_eq!(
            state(&mock, &mut session, "NO_MEDIA_PRESENT")
                .await
                .unwrap(),
            State::Stopped
        );
        assert!(state(&mock, &mut session, "RECORDING").await.is_err());
    }

    #[tokio::test]
    async fn a_track_loaded_paused_stays_paused_until_played() {
        let mock = Mock::start().await;
        let mut session = loaded(&mock, true, false).await;
        assert_eq!(session.poll().await.unwrap().state, State::Paused);
        session.play().await.unwrap();
        assert_eq!(
            state(&mock, &mut session, "STOPPED").await.unwrap(),
            State::Stopped
        );
    }

    #[tokio::test]
    async fn another_controller_or_a_transport_error_fails_the_poll() {
        let mock = Mock::start().await;
        let mut session = loaded(&mock, true, true).await;
        mock.renderer.lock().await.uri = "http://192.168.1.9/other.flac".into();
        let error = session.poll().await.unwrap_err();
        assert!(error.to_string().contains("another controller"));
        mock.renderer.lock().await.uri = String::new();
        // An empty track URI says nothing about who plays.
        assert!(session.poll().await.is_ok());
        mock.renderer.lock().await.status = "ERROR_OCCURRED".into();
        assert!(session.poll().await.is_err());
    }

    #[tokio::test]
    async fn volume_is_scaled_clamped_and_read_back() {
        let mock = Mock::start().await;
        let mut session = loaded(&mock, true, true).await;
        assert_eq!(mock.renderer.lock().await.volume, 50);
        session.volume(1.7).await.unwrap();
        assert_eq!(mock.renderer.lock().await.volume, 100);
        assert!(session.volume(f32::NAN).await.is_err());
        mock.renderer.lock().await.volume = 30;
        assert_eq!(session.poll().await.unwrap().volume, Some(0.3));
        // A renderer refusing GetVolume keeps the last known volume.
        mock.renderer.lock().await.refused.push("GetVolume");
        assert_eq!(session.poll().await.unwrap().volume, Some(0.3));
    }

    #[tokio::test]
    async fn renderers_without_volume_control_still_play() {
        let mock = Mock::start().await;
        let mut session = loaded(&mock, false, true).await;
        assert_eq!(mock.seen("SetVolume").await, 0);
        assert!(session.volume(0.5).await.is_err());
        assert_eq!(session.poll().await.unwrap().volume, None);
        // Without ConnectionManager, the original format is offered.
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(session.mime(&track(dir.path())).unwrap(), "audio/wav");
    }

    #[tokio::test]
    async fn a_refused_command_reports_the_renderer_fault() {
        let mock = Mock::start().await;
        let mut session = loaded(&mock, true, true).await;
        mock.renderer.lock().await.refused.push("Seek");
        let error = format!("{:#}", session.seek(3_500).await.unwrap_err());
        assert!(error.contains("401"), "{error}");
        mock.renderer.lock().await.refused.clear();
        // Seeks address whole seconds, as REL_TIME does.
        assert_eq!(session.seek(3_500).await.unwrap().position_ms, 3_000);
        assert_eq!(mock.renderer.lock().await.position, "0:00:03");
    }

    #[tokio::test]
    async fn stopping_an_unloaded_session_sends_nothing() {
        let mock = Mock::start().await;
        let mut session = Session::connect(&mock.device(true)).await.unwrap();
        assert_eq!(session.stop().await.unwrap().state, State::Stopped);
        assert_eq!(mock.seen("Stop").await, 0);
        // Nothing is loaded, so nothing is polled either.
        session.poll().await.unwrap();
        assert_eq!(mock.seen("GetPositionInfo").await, 0);
        let refused = Mock::start().await;
        refused
            .renderer
            .lock()
            .await
            .refused
            .push("GetTransportInfo");
        assert!(Session::connect(&refused.device(true)).await.is_err());
    }
}
