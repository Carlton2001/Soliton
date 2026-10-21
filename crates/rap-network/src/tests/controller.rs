use super::*;
use axum::{Router, body::Bytes, extract::State as HttpState, http::HeaderMap, routing::post};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
struct Renderer {
    state: String,
    uri: String,
    position: String,
    volume: u32,
    seen: Vec<String>,
}

async fn soap(
    HttpState(shared): HttpState<Arc<Mutex<Renderer>>>,
    headers: HeaderMap,
    body: Bytes,
) -> String {
    let action = headers["SOAPAction"]
        .to_str()
        .unwrap()
        .trim_matches('"')
        .rsplit('#')
        .next()
        .unwrap();
    let text = std::str::from_utf8(&body).unwrap();
    let doc = roxmltree::Document::parse(text).unwrap();
    let value = |tag: &str| {
        doc.descendants()
            .find(|n| n.has_tag_name(tag))
            .and_then(|n| n.text())
            .unwrap_or_default()
            .to_string()
    };
    let mut renderer = shared.lock().await;
    renderer.seen.push(action.into());
    let response = match action {
        "GetProtocolInfo" => "<Sink>http-get:*:audio/wav:*</Sink>".into(),
        "SetAVTransportURI" => {
            renderer.uri = value("CurrentURI");
            renderer.state = "STOPPED".into();
            String::new()
        }
        "Play" => {
            renderer.state = "PLAYING".into();
            String::new()
        }
        "Pause" => {
            renderer.state = "PAUSED_PLAYBACK".into();
            String::new()
        }
        "Stop" => {
            renderer.state = "STOPPED".into();
            String::new()
        }
        "Seek" => {
            renderer.position = value("Target");
            String::new()
        }
        "SetVolume" => {
            renderer.volume = value("DesiredVolume").parse().unwrap();
            String::new()
        }
        "GetVolume" => format!("<CurrentVolume>{}</CurrentVolume>", renderer.volume),
        "GetTransportInfo" => format!(
            "<CurrentTransportState>{}</CurrentTransportState><CurrentTransportStatus>OK</CurrentTransportStatus>",
            if renderer.state.is_empty() {
                "STOPPED"
            } else {
                &renderer.state
            }
        ),
        "GetPositionInfo" => format!(
            "<TrackURI>{}</TrackURI><RelTime>{}</RelTime><TrackDuration>0:00:10</TrackDuration>",
            media::xml(&renderer.uri),
            if renderer.position.is_empty() {
                "0:00:00"
            } else {
                &renderer.position
            }
        ),
        other => panic!("Unexpected SOAP command: {other}"),
    };
    format!(
        "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body><u:{action}Response xmlns:u=\"urn:schemas-upnp-org:service:AVTransport:1\">{response}</u:{action}Response></s:Body></s:Envelope>"
    )
}

#[tokio::test]
async fn upnp_session_controls_media_and_serves_only_the_current_track() {
    use crate::dlna::{Service, Services};
    let renderer = Arc::new(Mutex::new(Renderer::default()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/control", post(soap))
        .with_state(renderer.clone());
    let mock = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let service = |name| Service {
        kind: format!("urn:schemas-upnp-org:service:{name}:1"),
        url: format!("http://{address}/control"),
    };
    let device = Device {
        id: "upnp:fixture".into(),
        name: "Fixture renderer".into(),
        protocol: Protocol::Upnp,
        address,
        services: Some(Services {
            transport: service("AVTransport"),
            rendering: Some(service("RenderingControl")),
            connection: Some(service("ConnectionManager")),
        }),
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    std::fs::write(&path, b"synthetic WAV bytes").unwrap();
    let track = rap_core::model::Track {
        id: 1,
        source_id: 1,
        path: path.clone(),
        cover: None,
        available: true,
        metadata: rap_core::model::Metadata {
            title: "Été & <live>".into(),
            duration_ms: 10_000,
            ..Default::default()
        },
    };
    let mut controller = Controller::default();
    controller.command(Command::Connect(device)).await.unwrap();
    let status = controller
        .command(Command::Load {
            track: track.clone(),
            position_ms: 2000,
            autoplay: false,
            volume: 0.,
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.state, State::Paused);
    let first_generation = status.generation;
    let uri = renderer.lock().await.uri.clone();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    assert_eq!(
        client
            .get(&uri)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
        "synthetic WAV bytes"
    );
    assert_eq!(
        controller
            .command(Command::Play)
            .await
            .unwrap()
            .unwrap()
            .state,
        State::Playing
    );
    controller.command(Command::Seek(5000)).await.unwrap();
    controller.poll().await.unwrap();
    assert_eq!(controller.status.as_ref().unwrap().position_ms, 5000);
    assert_eq!(
        controller
            .command(Command::Pause)
            .await
            .unwrap()
            .unwrap()
            .state,
        State::Paused
    );
    controller.command(Command::Volume(0.25)).await.unwrap();
    assert_eq!(renderer.lock().await.volume, 25);
    controller.command(Command::Play).await.unwrap();
    renderer.lock().await.position = "0:00:09".into();
    controller.poll().await.unwrap();
    renderer.lock().await.state = "STOPPED".into();
    controller.poll().await.unwrap();
    assert_eq!(controller.status.as_ref().unwrap().state, State::Finished);
    assert_eq!(
        client.get(&uri).send().await.unwrap().status(),
        reqwest::StatusCode::NOT_FOUND
    );
    let next = controller
        .command(Command::Load {
            track,
            position_ms: 0,
            autoplay: true,
            volume: 0.,
        })
        .await
        .unwrap()
        .unwrap();
    assert!(next.generation > first_generation);
    controller.command(Command::Stop).await.unwrap();
    assert_eq!(controller.status.as_ref().unwrap().state, State::Stopped);
    controller.disconnect().await;
    assert_eq!(std::fs::read(path).unwrap(), b"synthetic WAV bytes");
    mock.abort();
}
