//! Capability URLs, restricted to the selected receiver. No filesystem paths in
//! requests, directory browsing, control endpoints or music-file modifications.
mod range;
use anyhow::{Result, ensure};
use axum::{
    Router,
    body::Body,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::stream;
use soliton_core::model::Track;
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    net::{TcpListener, UdpSocket},
    sync::{RwLock, Semaphore, watch},
};

struct Grant {
    token: String,
    audio: PathBuf,
    cover: Option<PathBuf>,
    mime: String,
    valid: Arc<AtomicBool>,
}

#[derive(Clone)]
struct Access {
    receiver: IpAddr,
    grant: Arc<RwLock<Option<Grant>>>,
    streams: Arc<Semaphore>,
}

pub(crate) struct Server {
    address: SocketAddr,
    access: Access,
    stop: watch::Sender<bool>,
}

#[derive(Clone, Debug)]
pub(crate) struct MediaUrls {
    pub audio: String,
    pub cover: Option<String>,
    pub mime: String,
}

impl Server {
    pub async fn start(receiver: IpAddr) -> Result<Self> {
        let route = UdpSocket::bind("0.0.0.0:0").await?;
        route.connect(SocketAddr::new(receiver, 8009)).await?;
        let listener = TcpListener::bind(SocketAddr::new(route.local_addr()?.ip(), 0)).await?;
        let address = listener.local_addr()?;
        let access = Access {
            receiver,
            grant: Default::default(),
            streams: Arc::new(Semaphore::new(8)),
        };
        let router = Router::new()
            .route("/{token}/{kind}", get(serve).options(serve))
            .with_state(access.clone());
        let (stop, mut stopped) = watch::channel(false);
        tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                let _ = stopped.changed().await;
            })
            .await;
        });
        Ok(Self {
            address,
            access,
            stop,
        })
    }

    pub async fn authorize(&self, track: &Track, mime: String) -> Result<MediaUrls> {
        ensure!(track.available, "Track is unavailable");
        // Validate read access before replacing a working grant. Music is read-only.
        let file = tokio::fs::File::open(&track.path).await?;
        ensure!(
            file.metadata().await?.is_file(),
            "Audio source is not a regular file"
        );
        drop(file);
        let cover = match &track.cover {
            Some(path) if tokio::fs::metadata(path).await.is_ok_and(|m| m.is_file()) => {
                Some(path.clone())
            }
            _ => None,
        };
        let token = uuid::Uuid::new_v4().simple().to_string();
        let base = format!("http://{}/{token}", self.address);
        let urls = MediaUrls {
            audio: format!("{base}/audio"),
            cover: cover.as_ref().map(|_| format!("{base}/cover")),
            mime: mime.clone(),
        };
        let mut current = self.access.grant.write().await;
        if let Some(old) = current.take() {
            old.valid.store(false, Ordering::Relaxed);
        }
        *current = Some(Grant {
            token,
            audio: track.path.clone(),
            cover,
            mime,
            valid: Arc::new(AtomicBool::new(true)),
        });
        Ok(urls)
    }

    pub async fn revoke(&self) {
        if let Some(grant) = self.access.grant.write().await.take() {
            grant.valid.store(false, Ordering::Relaxed);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Ok(grant) = self.access.grant.try_read()
            && let Some(grant) = grant.as_ref()
        {
            grant.valid.store(false, Ordering::Relaxed);
        }
        let _ = self.stop.send(true);
    }
}

async fn serve(
    State(access): State<Access>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path((token, kind)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    tracing::debug!(%peer, %method, %kind, "Receiver media request");
    let cors = [
        (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
        (header::ACCESS_CONTROL_ALLOW_METHODS, "GET, HEAD, OPTIONS"),
        (
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            "Range, Content-Type, getcontentFeatures.dlna.org, transferMode.dlna.org",
        ),
        (
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            "Content-Length, Content-Range, Accept-Ranges",
        ),
    ];
    if peer.ip() != access.receiver {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some((path, mime, valid)) = access
        .grant
        .read()
        .await
        .as_ref()
        .filter(|g| g.token == token)
        .and_then(|g| match kind.as_str() {
            "audio" => Some((g.audio.clone(), g.mime.clone(), g.valid.clone())),
            "cover" => g
                .cover
                .clone()
                .map(|p| (p, "image/png".into(), g.valid.clone())),
            _ => None,
        })
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if method == Method::OPTIONS {
        return (StatusCode::NO_CONTENT, cors).into_response();
    }
    let Ok(permit) = access.streams.clone().try_acquire_owned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(metadata) = file.metadata().await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let length = metadata.len();
    let requested = headers.get(header::RANGE);
    let span = match requested {
        Some(value) => value.to_str().ok().and_then(|v| range::parse(v, length)),
        None => Some((0, length.saturating_sub(1))),
    };
    let Some((start, end)) = span else {
        return (
            StatusCode::RANGE_NOT_SATISFIABLE,
            cors,
            [(header::CONTENT_RANGE, format!("bytes */{length}"))],
        )
            .into_response();
    };
    let bytes = if length == 0 { 0 } else { end - start + 1 };
    if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let status = if requested.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        // The permit lives with the body, not merely until the handler returns.
        let stream = stream::try_unfold(
            (file.take(bytes), permit, valid),
            |(mut file, permit, valid)| async move {
                if !valid.load(Ordering::Relaxed) {
                    return Ok::<_, std::io::Error>(None);
                }
                let mut buffer = vec![0; 64 * 1024];
                let count = file.read(&mut buffer).await?;
                if count == 0 || !valid.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                buffer.truncate(count);
                Ok(Some((buffer, (file, permit, valid))))
            },
        );
        Body::from_stream(stream)
    };
    let mut response = (status, cors, body).into_response();
    let out = response.headers_mut();
    out.insert(header::CONTENT_TYPE, mime.parse().unwrap());
    out.insert(header::CONTENT_LENGTH, bytes.into());
    out.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
    out.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    out.insert("transferMode.dlna.org", "Streaming".parse().unwrap());
    out.insert(
        "contentFeatures.dlna.org",
        "DLNA.ORG_OP=01;DLNA.ORG_CI=0".parse().unwrap(),
    );
    if requested.is_some() {
        out.insert(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{length}").parse().unwrap(),
        );
    }
    response
}

#[cfg(test)]
#[path = "../tests/http.rs"]
mod tests;
