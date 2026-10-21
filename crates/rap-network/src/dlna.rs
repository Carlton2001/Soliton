//! UPnP AVTransport and RenderingControl. HTTP destinations are validated at
//! discovery and redirects/proxies are disabled for every subsequent request.
use crate::{Device, State, Status, http::MediaUrls, media};
use anyhow::{Context, Result, bail, ensure};
use rap_core::model::Track;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Service {
    pub kind: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Services {
    pub transport: Service,
    pub rendering: Option<Service>,
    pub connection: Option<Service>,
}

pub(crate) fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(2))
        .timeout(std::time::Duration::from_secs(4))
        .build()?)
}

pub(crate) async fn text(mut response: reqwest::Response) -> Result<String> {
    ensure!(
        response.content_length().is_none_or(|n| n <= 1024 * 1024),
        "UPnP response is too large"
    );
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            body.len() + chunk.len() <= 1024 * 1024,
            "UPnP response is too large"
        );
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8(body)?)
}

async fn action(
    client: &reqwest::Client,
    service: &Service,
    name: &str,
    args: &str,
) -> Result<HashMap<String, String>> {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{name} xmlns:u=\"{}\">{args}</u:{name}></s:Body></s:Envelope>",
        media::xml(&service.kind)
    );
    let response = client
        .post(&service.url)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPAction", format!("\"{}#{name}\"", service.kind))
        .body(body)
        .send()
        .await?;
    let status = response.status();
    let body = text(response).await?;
    parse_response(&body).with_context(|| format!("UPnP {name} ({status})"))
}

fn parse_response(body: &str) -> Result<HashMap<String, String>> {
    let doc = roxmltree::Document::parse(body)?;
    if let Some(fault) = doc.descendants().find(|n| n.has_tag_name("Fault")) {
        let details: Vec<_> = fault
            .descendants()
            .filter(|n| n.is_element() && n.children().all(|c| !c.is_element()))
            .filter_map(|n| n.text())
            .collect();
        bail!("Renderer rejected the command: {}", details.join(" "));
    }
    let response = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name().ends_with("Response"))
        .context("No UPnP response")?;
    Ok(response
        .children()
        .filter(|n| n.is_element())
        .map(|n| {
            (
                n.tag_name().name().into(),
                n.text().unwrap_or_default().into(),
            )
        })
        .collect())
}

pub(crate) struct Session {
    client: reqwest::Client,
    services: Services,
    sinks: Option<Vec<String>>,
    pub last: Status,
    uri: Option<String>,
    played: bool,
    ready_paused: bool,
}

impl Session {
    pub async fn connect(device: &Device) -> Result<Self> {
        let services = device.services.clone().context("Missing UPnP services")?;
        let client = client()?;
        action(
            &client,
            &services.transport,
            "GetTransportInfo",
            "<InstanceID>0</InstanceID>",
        )
        .await?;
        let sinks = match &services.connection {
            Some(service) => action(&client, service, "GetProtocolInfo", "")
                .await
                .ok()
                .and_then(|map| {
                    map.get("Sink")
                        .map(|s| s.split(',').map(str::to_owned).collect())
                }),
            None => None,
        };
        Ok(Self {
            client,
            services,
            sinks,
            last: Status::default(),
            uri: None,
            played: false,
            ready_paused: false,
        })
    }

    pub fn mime(&self, track: &Track) -> Result<String> {
        let mime = media::mime(track)?;
        negotiate(mime, self.sinks.as_deref())
    }

    pub async fn load(
        &mut self,
        track: &Track,
        urls: &MediaUrls,
        position: u64,
        autoplay: bool,
        volume: f32,
    ) -> Result<Status> {
        // Several renderers reject a new URI until the previous transport stops.
        let _ = self.transport("Stop", "").await;
        let didl = metadata(track, urls);
        self.transport(
            "SetAVTransportURI",
            &format!(
                "<CurrentURI>{}</CurrentURI><CurrentURIMetaData>{}</CurrentURIMetaData>",
                media::xml(&urls.audio),
                media::xml(&didl)
            ),
        )
        .await?;
        self.uri = Some(urls.audio.clone());
        self.played = false;
        self.ready_paused = !autoplay;
        self.last = Status {
            state: State::Paused,
            duration_ms: track.metadata.duration_ms,
            ..Default::default()
        };
        if self.services.rendering.is_some() {
            self.volume(volume).await?;
        }
        if position > 0 {
            self.seek(position).await?;
        }
        if autoplay {
            self.play().await?;
        }
        Ok(self.last.clone())
    }

    async fn transport(&self, name: &str, args: &str) -> Result<HashMap<String, String>> {
        action(
            &self.client,
            &self.services.transport,
            name,
            &format!("<InstanceID>0</InstanceID>{args}"),
        )
        .await
    }

    pub async fn play(&mut self) -> Result<Status> {
        self.transport("Play", "<Speed>1</Speed>").await?;
        self.ready_paused = false;
        self.played = true;
        self.last.state = State::Playing;
        Ok(self.last.clone())
    }
    pub async fn pause(&mut self) -> Result<Status> {
        self.transport("Pause", "").await?;
        self.last.state = State::Paused;
        Ok(self.last.clone())
    }
    pub async fn stop(&mut self) -> Result<Status> {
        if self.uri.is_some() {
            self.transport("Stop", "").await?;
        }
        self.uri = None;
        self.last.state = State::Stopped;
        self.last.position_ms = 0;
        Ok(self.last.clone())
    }
    pub async fn seek(&mut self, position: u64) -> Result<Status> {
        self.transport(
            "Seek",
            &format!(
                "<Unit>REL_TIME</Unit><Target>{}</Target>",
                media::duration(position)
            ),
        )
        .await?;
        self.last.position_ms = position / 1000 * 1000;
        Ok(self.last.clone())
    }
    pub async fn volume(&mut self, volume: f32) -> Result<Status> {
        ensure!(volume.is_finite(), "Invalid volume");
        let service = self
            .services
            .rendering
            .as_ref()
            .context("Volume control is unavailable on this renderer")?;
        action(&self.client, service, "SetVolume", &format!("<InstanceID>0</InstanceID><Channel>Master</Channel><DesiredVolume>{}</DesiredVolume>", (volume.clamp(0., 1.) * 100.).round() as u32)).await?;
        self.last.volume = Some(volume.clamp(0., 1.));
        Ok(self.last.clone())
    }

    pub async fn poll(&mut self) -> Result<Status> {
        if self.uri.is_none() {
            return Ok(self.last.clone());
        }
        let info = self.transport("GetTransportInfo", "").await?;
        ensure!(
            info.get("CurrentTransportStatus").is_none_or(|s| s == "OK"),
            "Renderer reported a transport error"
        );
        let position = self.transport("GetPositionInfo", "").await?;
        if let Some(uri) = position.get("TrackURI").filter(|u| !u.is_empty()) {
            ensure!(
                Some(uri) == self.uri.as_ref(),
                "The renderer is playing media selected by another controller"
            );
        }
        let elapsed = position.get("RelTime").and_then(|s| media::milliseconds(s));
        let duration = position
            .get("TrackDuration")
            .and_then(|s| media::milliseconds(s))
            .filter(|n| *n > 0)
            .unwrap_or(self.last.duration_ms);
        let state = match info.get("CurrentTransportState").map(String::as_str) {
            Some("PLAYING") => {
                self.played = true;
                State::Playing
            }
            Some("PAUSED_PLAYBACK") => State::Paused,
            Some("TRANSITIONING") => State::Buffering,
            Some("STOPPED") if self.ready_paused => State::Paused,
            Some("STOPPED")
                if self.played
                    && duration > 0
                    && self.last.position_ms.saturating_add(1500) >= duration =>
            {
                State::Finished
            }
            Some("STOPPED" | "NO_MEDIA_PRESENT") => State::Stopped,
            _ => bail!("Unknown UPnP playback state"),
        };
        self.last.state = state;
        self.last.position_ms = elapsed.unwrap_or(self.last.position_ms);
        self.last.duration_ms = duration;
        if let Some(service) = &self.services.rendering {
            // Some devices advertise RenderingControl but omit volume support.
            if let Ok(volume) = action(
                &self.client,
                service,
                "GetVolume",
                "<InstanceID>0</InstanceID><Channel>Master</Channel>",
            )
            .await
            {
                self.last.volume = volume
                    .get("CurrentVolume")
                    .and_then(|s| s.parse::<f32>().ok())
                    .filter(|v| v.is_finite())
                    .map(|v| (v / 100.).clamp(0., 1.));
            }
        }
        Ok(self.last.clone())
    }
}

fn negotiate(mime: &str, sinks: Option<&[String]>) -> Result<String> {
    let Some(sinks) = sinks.filter(|s| !s.is_empty()) else {
        return Ok(mime.into());
    };
    for sink in sinks {
        let mut fields = sink.splitn(4, ':');
        let protocol = fields.next().unwrap_or_default();
        let _network = fields.next();
        let offered = fields.next().unwrap_or_default();
        let aliases = matches!(
            (mime, offered),
            ("audio/flac", "audio/x-flac")
                | ("audio/wav", "audio/x-wav" | "audio/wave")
                | ("audio/aiff", "audio/x-aiff")
        );
        if protocol.eq_ignore_ascii_case("http-get")
            && (offered == "*" || offered == mime || aliases)
        {
            return Ok(if offered == "*" { mime } else { offered }.into());
        }
    }
    bail!("The renderer does not accept {mime}; network transcoding is not enabled")
}

fn metadata(track: &Track, urls: &MediaUrls) -> String {
    let m = &track.metadata;
    let art = urls
        .cover
        .as_ref()
        .map(|u| format!("<upnp:albumArtURI>{}</upnp:albumArtURI>", media::xml(u)))
        .unwrap_or_default();
    format!(
        "<DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\"><item id=\"0\" parentID=\"-1\" restricted=\"1\"><dc:title>{}</dc:title><upnp:artist>{}</upnp:artist><upnp:album>{}</upnp:album><upnp:class>object.item.audioItem.musicTrack</upnp:class>{art}<res protocolInfo=\"http-get:*:{}:*\" duration=\"{}\">{}</res></item></DIDL-Lite>",
        media::xml(&m.title),
        media::xml(&m.artist),
        media::xml(&m.album),
        media::xml(&urls.mime),
        media::duration(m.duration_ms),
        media::xml(&urls.audio)
    )
}

#[cfg(test)]
#[path = "tests/dlna.rs"]
mod tests;
