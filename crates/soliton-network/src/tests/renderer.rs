//! A UPnP MediaRenderer answering SOAP on loopback, shared by the session and
//! controller tests. Its state is set and inspected through the shared lock.
use crate::{
    Device, Protocol,
    dlna::{Service, Services},
    media,
};
use axum::{Router, body::Bytes, extract::State, http::HeaderMap, routing::post};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

pub(crate) struct Renderer {
    pub state: String,
    /// `CurrentTransportStatus`; anything but OK reports a transport error.
    pub status: String,
    pub uri: String,
    pub position: String,
    pub duration: String,
    pub volume: u32,
    pub sinks: String,
    /// Actions answered with a SOAP fault.
    pub refused: Vec<&'static str>,
    pub seen: Vec<String>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            state: "STOPPED".into(),
            status: "OK".into(),
            uri: String::new(),
            position: "0:00:00".into(),
            duration: "0:00:10".into(),
            volume: 0,
            sinks: "http-get:*:audio/wav:*".into(),
            refused: Vec::new(),
            seen: Vec::new(),
        }
    }
}

pub(crate) type Shared = Arc<Mutex<Renderer>>;

pub(crate) struct Mock {
    pub renderer: Shared,
    pub address: SocketAddr,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl Mock {
    pub async fn start() -> Self {
        let renderer = Shared::default();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/control", post(soap))
            .with_state(renderer.clone());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            renderer,
            address,
            server,
        }
    }

    /// With RenderingControl and ConnectionManager only when `full`.
    pub fn device(&self, full: bool) -> Device {
        let service = |name| Service {
            kind: format!("urn:schemas-upnp-org:service:{name}:1"),
            url: format!("http://{}/control", self.address),
        };
        Device {
            id: "upnp:fixture".into(),
            name: "Fixture renderer".into(),
            protocol: Protocol::Upnp,
            address: self.address,
            services: Some(Services {
                transport: service("AVTransport"),
                rendering: full.then(|| service("RenderingControl")),
                connection: full.then(|| service("ConnectionManager")),
            }),
        }
    }

    pub async fn seen(&self, action: &str) -> usize {
        let renderer = self.renderer.lock().await;
        renderer.seen.iter().filter(|a| *a == action).count()
    }
}

async fn soap(State(shared): State<Shared>, headers: HeaderMap, body: Bytes) -> String {
    let action = headers["SOAPAction"]
        .to_str()
        .unwrap()
        .trim_matches('"')
        .rsplit('#')
        .next()
        .unwrap()
        .to_owned();
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
    renderer.seen.push(action.clone());
    if renderer.refused.contains(&action.as_str()) {
        return "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body><s:Fault><faultcode>s:Client</faultcode><detail><UPnPError><errorCode>401</errorCode><errorDescription>Invalid Action</errorDescription></UPnPError></detail></s:Fault></s:Body></s:Envelope>".into();
    }
    let response = match action.as_str() {
        "GetProtocolInfo" => format!("<Sink>{}</Sink>", renderer.sinks),
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
            "<CurrentTransportState>{}</CurrentTransportState><CurrentTransportStatus>{}</CurrentTransportStatus>",
            renderer.state, renderer.status
        ),
        "GetPositionInfo" => format!(
            "<TrackURI>{}</TrackURI><RelTime>{}</RelTime><TrackDuration>{}</TrackDuration>",
            media::xml(&renderer.uri),
            renderer.position,
            renderer.duration
        ),
        other => panic!("Unexpected SOAP command: {other}"),
    };
    format!(
        "<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body><u:{action}Response xmlns:u=\"urn:schemas-upnp-org:service:AVTransport:1\">{response}</u:{action}Response></s:Body></s:Envelope>"
    )
}

/// A readable WAV-named file with a ten-second duration in its metadata.
pub(crate) fn track(dir: &std::path::Path) -> soliton_core::model::Track {
    let path = dir.join("fixture.wav");
    std::fs::write(&path, b"synthetic WAV bytes").unwrap();
    soliton_core::model::Track {
        id: 1,
        source_id: 1,
        path,
        cover: None,
        available: true,
        metadata: soliton_core::model::Metadata {
            title: "Été & <live>".into(),
            duration_ms: 10_000,
            ..Default::default()
        },
    }
}
