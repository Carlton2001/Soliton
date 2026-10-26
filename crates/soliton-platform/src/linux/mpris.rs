//! MPRIS 2 on the session bus: GNOME media controls and multimedia keys.
//! Bus callbacks only enqueue commands; the application stays authoritative.
use crate::{DesktopEvent, MediaCommand, MediaState, TransportState};
use anyhow::Result;
use crossbeam_channel::Sender;
use std::{collections::HashMap, os::unix::ffi::OsStrExt, path::Path, time::Instant};
use zbus::{
    blocking::{Connection, connection::Builder},
    fdo, interface,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedValue, Value},
};

const PATH: &str = "/org/mpris/MediaPlayer2";
const NAME: &str = "org.mpris.MediaPlayer2.soliton";
const NO_TRACK: &str = "/org/mpris/MediaPlayer2/TrackList/NoTrack";
/// Position drift beyond normal playback is announced as a seek.
const SEEK_TOLERANCE_MS: i64 = 1500;

pub(super) struct Mpris {
    connection: Connection,
    published: Instant,
}

impl Mpris {
    pub fn start(events: Sender<DesktopEvent>) -> Result<Self> {
        let connect = |name: String| -> zbus::Result<Connection> {
            Builder::session()?
                .allow_name_replacements(false)
                .replace_existing_names(false)
                .serve_at(
                    PATH,
                    Root {
                        events: events.clone(),
                    },
                )?
                .serve_at(
                    PATH,
                    Player {
                        events: events.clone(),
                        state: MediaState::default(),
                        track: 0,
                    },
                )?
                .name(name)?
                .build()
        };
        // A second running instance takes the suffixed name the specification defines.
        let connection = match connect(NAME.into()) {
            Err(zbus::Error::NameTaken) => {
                connect(format!("{NAME}.instance{}", std::process::id()))?
            }
            result => result?,
        };
        Ok(Self {
            connection,
            published: Instant::now(),
        })
    }

    pub fn update(&mut self, state: MediaState) -> Result<()> {
        let elapsed = self.published.elapsed().as_millis() as i64;
        self.published = Instant::now();
        let server = self.connection.object_server();
        let player = server.interface::<_, Player>(PATH)?;
        let emitter = player.signal_emitter().clone();
        let changes = {
            let mut player = player.get_mut();
            let changes = Changes::between(&player.state, &state, elapsed);
            if changes.track {
                player.track = player.track.wrapping_add(1);
            }
            player.state = state;
            changes
        };
        if !changes.any() {
            return Ok(());
        }
        let player = player.get();
        zbus::block_on(async {
            if changes.status {
                player.playback_status_changed(&emitter).await?;
            }
            if changes.metadata {
                player.metadata_changed(&emitter).await?;
            }
            if changes.capabilities {
                player.can_play_changed(&emitter).await?;
                player.can_pause_changed(&emitter).await?;
                player.can_go_next_changed(&emitter).await?;
                player.can_go_previous_changed(&emitter).await?;
                player.can_seek_changed(&emitter).await?;
            }
            if changes.seeked {
                Player::seeked(&emitter, micros(player.state.position_ms)).await?;
            }
            zbus::Result::Ok(())
        })?;
        Ok(())
    }
}

/// Which published values differ between two application states.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Changes {
    pub status: bool,
    pub track: bool,
    pub metadata: bool,
    pub capabilities: bool,
    pub seeked: bool,
}

impl Changes {
    pub fn between(previous: &MediaState, next: &MediaState, elapsed_ms: i64) -> Self {
        let track = previous.track != next.track;
        let expected = previous.position_ms as i64
            + if previous.state == TransportState::Playing {
                elapsed_ms
            } else {
                0
            };
        Self {
            status: previous.state != next.state,
            track,
            metadata: track || previous.duration_ms != next.duration_ms,
            capabilities: previous.track.is_some() != next.track.is_some()
                || (previous.duration_ms > 0) != (next.duration_ms > 0),
            seeked: !track
                && next.track.is_some()
                && (next.position_ms as i64 - expected).abs() > SEEK_TOLERANCE_MS,
        }
    }

    fn any(&self) -> bool {
        self != &Self::default()
    }
}

fn micros(milliseconds: u64) -> i64 {
    i64::try_from(milliseconds.saturating_mul(1000)).unwrap_or(i64::MAX)
}

/// `Seek` is relative; past the end it means the next track, per the specification.
pub(super) fn seek_command(state: &MediaState, offset_us: i64) -> Option<MediaCommand> {
    state.track.as_ref()?;
    let target = state.position_ms as i64 + offset_us / 1000;
    if state.duration_ms > 0 && target > state.duration_ms as i64 {
        return Some(MediaCommand::Next);
    }
    Some(MediaCommand::Seek(target.max(0) as u64))
}

/// Percent-encoded `file://` URI; the path is Soliton's artwork cache, never a source.
pub(super) fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

struct Root {
    events: Sender<DesktopEvent>,
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) {
        let _ = self.events.try_send(DesktopEvent::Show);
    }

    fn quit(&self) {
        let _ = self.events.try_send(DesktopEvent::Quit);
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_raise(&self) -> bool {
        true
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn identity(&self) -> String {
        "Soliton".into()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn desktop_entry(&self) -> String {
        crate::APP_ID.into()
    }

    /// Files outside the library are never opened.
    #[zbus(property(emits_changed_signal = "const"))]
    fn supported_uri_schemes(&self) -> Vec<String> {
        Vec::new()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn supported_mime_types(&self) -> Vec<String> {
        Vec::new()
    }
}

pub(super) struct Player {
    events: Sender<DesktopEvent>,
    state: MediaState,
    /// Incremented on each track change, so clients can tell occurrences apart.
    track: u64,
}

impl Player {
    fn send(&self, command: MediaCommand) {
        let _ = self.events.try_send(DesktopEvent::Media(command));
    }

    fn track_id(&self) -> String {
        if self.state.track.is_some() {
            format!("/io/github/Carlton2001/Soliton/Track/{}", self.track)
        } else {
            NO_TRACK.into()
        }
    }
}

fn owned(value: Value<'_>) -> Option<OwnedValue> {
    value.try_to_owned().ok()
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    fn next(&self) {
        self.send(MediaCommand::Next);
    }

    fn previous(&self) {
        self.send(MediaCommand::Previous);
    }

    fn pause(&self) {
        self.send(MediaCommand::Pause);
    }

    fn play_pause(&self) {
        self.send(MediaCommand::Toggle);
    }

    fn stop(&self) {
        self.send(MediaCommand::Stop);
    }

    fn play(&self) {
        self.send(MediaCommand::Play);
    }

    fn seek(&self, offset: i64) {
        if let Some(command) = seek_command(&self.state, offset) {
            self.send(command);
        }
    }

    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
        // Stale requests for a previous track and out-of-range positions are ignored.
        if self.state.track.is_some()
            && track_id.as_str() == self.track_id()
            && (0..=micros(self.state.duration_ms)).contains(&position)
        {
            self.send(MediaCommand::Seek(position as u64 / 1000));
        }
    }

    fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "Soliton only plays its library".into(),
        ))
    }

    #[zbus(signal)]
    async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> String {
        match self.state.state {
            TransportState::Playing => "Playing",
            TransportState::Paused => "Paused",
            TransportState::Stopped => "Stopped",
        }
        .into()
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn rate(&self) -> f64 {
        1.
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn minimum_rate(&self) -> f64 {
        1.
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn maximum_rate(&self) -> f64 {
        1.
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let mut metadata = HashMap::new();
        if let Ok(path) = ObjectPath::try_from(self.track_id())
            && let Some(value) = owned(Value::from(path))
        {
            metadata.insert("mpris:trackid".into(), value);
        }
        let Some(track) = &self.state.track else {
            return metadata;
        };
        let mut insert = |key: &str, value: Value<'_>| {
            if let Some(value) = owned(value) {
                metadata.insert(key.into(), value);
            }
        };
        if self.state.duration_ms > 0 {
            insert("mpris:length", Value::from(micros(self.state.duration_ms)));
        }
        insert("xesam:title", Value::from(track.title.as_str()));
        if !track.artist.is_empty() {
            insert("xesam:artist", Value::from(vec![track.artist.as_str()]));
        }
        if !track.album.is_empty() {
            insert("xesam:album", Value::from(track.album.as_str()));
        }
        if let Some(cover) = &track.cover {
            insert("mpris:artUrl", Value::from(file_uri(cover)));
        }
        metadata
    }

    /// Read on demand by clients; changes are signalled through `Seeked` only.
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> i64 {
        micros(self.state.position_ms)
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        self.state.track.is_some()
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        self.state.track.is_some()
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        self.state.track.is_some()
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        self.state.track.is_some()
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.state.track.is_some() && self.state.duration_ms > 0
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_control(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[path = "tests/mpris.rs"]
mod tests;
