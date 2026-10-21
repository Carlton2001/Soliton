//! Google Cast via the standard media receiver. The adapter deliberately never
//! reconnects or re-launches playback without an explicit command from RAP.
use crate::{Device, State, Status, http::MediaUrls};
use anyhow::{Context, Result, bail, ensure};
use oxicast::{
    CastApp, CastClient, IdleReason, MediaInfo, MediaMetadata, MediaStatus, PlayerState,
};
use rap_core::model::Track;
use std::time::Duration;

pub(crate) struct Session {
    client: CastClient,
    uri: Option<String>,
    last: Status,
    app_session: String,
    media_session: Option<i32>,
}

impl Session {
    pub async fn connect(device: &Device) -> Result<Self> {
        let client = CastClient::builder(device.address.ip().to_string(), device.address.port())
            .auto_reconnect(false)
            .request_timeout(Duration::from_secs(6))
            .heartbeat_interval(Duration::from_secs(5))
            .heartbeat_timeout(Duration::from_secs(15))
            .event_buffer_size(32)
            .connect()
            .await
            .context("Connect to Google Cast")?;
        // Discovery is never enough to take over a receiver: launch occurs only
        // after the user selects this output.
        let app = match client.launch_app(&CastApp::DefaultMediaReceiver).await {
            Ok(app) => app,
            Err(error) => {
                let _ = client.disconnect().await;
                return Err(anyhow::Error::new(error).context("Launch Google Cast media receiver"));
            }
        };
        Ok(Self {
            client,
            uri: None,
            last: Status::default(),
            app_session: app.session_id,
            media_session: None,
        })
    }

    pub async fn load(
        &mut self,
        track: &Track,
        urls: &MediaUrls,
        position: u64,
        autoplay: bool,
        volume: f32,
    ) -> Result<Status> {
        let m = &track.metadata;
        let images = urls
            .cover
            .as_ref()
            .map(|url| oxicast::Image {
                url: url.clone(),
                width: None,
                height: None,
            })
            .into_iter()
            .collect();
        let info = MediaInfo::new(&urls.audio, &urls.mime)
            .maybe_duration((m.duration_ms > 0).then_some(m.duration_ms as f64 / 1000.))
            .metadata(MediaMetadata::MusicTrack {
                title: Some(m.title.clone()),
                artist: Some(m.artist.clone()),
                album_name: Some(m.album.clone()),
                composer: None,
                track_number: m.track,
                disc_number: m.disc,
                images,
            });
        self.volume(volume).await?;
        let media = self
            .client
            .load_media(&info, autoplay, position as f64 / 1000., None)
            .await
            .context("Load Google Cast media")?;
        tracing::debug!(?media, "Cast LOAD acknowledgement");
        self.uri = Some(urls.audio.clone());
        self.media_session = Some(media.media_session_id);
        let initial_idle = media.player_state == PlayerState::Idle;
        let mut status = self.update(media)?;
        if initial_idle {
            // Android TV can acknowledge LOAD before its media player starts.
            // Keep polling this generation rather than treating that as STOP.
            status.state = State::Buffering;
            self.last = status.clone();
        }
        Ok(status)
    }

    fn update(&mut self, media: MediaStatus) -> Result<Status> {
        ensure!(
            self.media_session
                .is_none_or(|id| id == media.media_session_id),
            "The Cast media session was replaced by another controller"
        );
        if let Some(info) = &media.media {
            ensure!(
                self.uri.as_ref().is_none_or(|uri| uri == &info.content_id),
                "The Cast receiver is playing media selected by another controller"
            );
        }
        self.last = status(&media, self.last.volume)?;
        Ok(self.last.clone())
    }

    pub async fn play(&mut self) -> Result<Status> {
        let m = self
            .client
            .play()
            .await
            .context("Resume Google Cast playback")?;
        self.update(m)
    }
    pub async fn pause(&mut self) -> Result<Status> {
        let m = self
            .client
            .pause()
            .await
            .context("Pause Google Cast playback")?;
        self.update(m)
    }
    pub async fn stop(&mut self) -> Result<Status> {
        if self.uri.is_some() {
            let _ = self
                .client
                .stop_media()
                .await
                .context("Stop Google Cast playback")?;
        }
        self.uri = None;
        self.media_session = None;
        self.last.state = State::Stopped;
        self.last.position_ms = 0;
        Ok(self.last.clone())
    }
    pub async fn seek(&mut self, position: u64) -> Result<Status> {
        let m = self
            .client
            .seek(position as f64 / 1000.)
            .await
            .context("Seek Google Cast media")?;
        self.update(m)
    }
    pub async fn volume(&mut self, volume: f32) -> Result<Status> {
        ensure!(volume.is_finite(), "Invalid volume");
        let volume = self
            .client
            .set_volume(volume.clamp(0., 1.))
            .await
            .context("Set Google Cast volume")?;
        self.last.volume = Some(if volume.muted { 0. } else { volume.level });
        Ok(self.last.clone())
    }
    pub async fn poll(&mut self) -> Result<Status> {
        ensure!(
            self.client.is_connected(),
            "The Google Cast receiver disconnected"
        );
        if self.uri.is_none() {
            return Ok(self.last.clone());
        }
        // FINISHED is broadcast asynchronously; a later GET_STATUS may already
        // return an empty list. Retain that real end reason for queue navigation.
        let watcher = self.client.watch_media_status();
        let cached = watcher.borrow().clone();
        if let Some(media) = cached.filter(|m| {
            Some(m.media_session_id) == self.media_session
                && m.player_state == PlayerState::Idle
                && m.idle_reason.is_some()
        }) {
            return self.update(media);
        }
        let media = self
            .client
            .media_status()
            .await
            .context("Read Google Cast playback status")?
            .or_else(|| {
                watcher.borrow().clone().filter(|m| {
                    Some(m.media_session_id) == self.media_session
                        && m.player_state == PlayerState::Idle
                })
            })
            .context("Google Cast media session ended without a playback status")?;
        let mut status = self.update(media)?;
        let receiver = self.client.watch_receiver_status();
        if let Some(receiver) = receiver.borrow().as_ref() {
            status.volume = Some(if receiver.volume.muted {
                0.
            } else {
                receiver.volume.level
            });
        }
        self.last = status.clone();
        Ok(status)
    }
    pub async fn disconnect(&mut self) {
        // Only close the receiver session selected by RAP, not an unrelated app
        // that another controller might have launched in the meantime.
        let _ = self.client.stop_app(&self.app_session).await;
        let _ = self.client.disconnect().await;
    }
}

fn status(media: &MediaStatus, volume: Option<f32>) -> Result<Status> {
    let state = match media.player_state {
        PlayerState::Playing => State::Playing,
        PlayerState::Paused => State::Paused,
        PlayerState::Buffering => State::Buffering,
        PlayerState::Idle => match media.idle_reason {
            Some(IdleReason::Finished) => State::Finished,
            Some(IdleReason::Error) => bail!("Google Cast rejected the media or its format"),
            _ => State::Stopped,
        },
        _ => bail!("Unknown Google Cast playback state"),
    };
    ensure!(
        media.current_time.is_finite() && media.current_time >= 0.,
        "Invalid Cast playback position"
    );
    Ok(Status {
        state,
        position_ms: (media.current_time * 1000.) as u64,
        duration_ms: media
            .duration
            .or_else(|| media.media.as_ref().and_then(|m| m.duration))
            .filter(|d| d.is_finite() && *d >= 0.)
            .map_or(0, |d| (d * 1000.) as u64),
        volume,
        ..Default::default()
    })
}

#[cfg(test)]
#[path = "tests/cast.rs"]
mod tests;
