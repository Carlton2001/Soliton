//! SMTC desktop interop. All WinRT calls and cached artwork reads run off the UI.
use crate::{DesktopEvent, MediaCommand, MediaState, TransportState};
use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use std::{fs::File, io::Read, path::Path};
use windows::{
    Foundation::{TimeSpan, TypedEventHandler},
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
        SystemMediaTransportControlsButton as Button,
        SystemMediaTransportControlsTimelineProperties,
    },
    Storage::Streams::{DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference},
    Win32::{Foundation::HWND, System::WinRT::ISystemMediaTransportControlsInterop},
    core::{HSTRING, factory},
};

pub(super) struct Media {
    controls: SystemMediaTransportControls,
    button: i64,
    seek: i64,
    previous: Option<MediaState>,
    events: Sender<DesktopEvent>,
}

impl Media {
    pub fn new(window: HWND, events: Sender<DesktopEvent>) -> Result<Self> {
        let interop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()?;
        let controls: SystemMediaTransportControls = unsafe { interop.GetForWindow(window)? };
        controls.SetIsPlayEnabled(true)?;
        controls.SetIsPauseEnabled(true)?;
        controls.SetIsStopEnabled(true)?;
        controls.SetIsPreviousEnabled(true)?;
        controls.SetIsNextEnabled(true)?;
        controls.SetPlaybackStatus(MediaPlaybackStatus::Stopped)?;
        let buttons = events.clone();
        let button = controls.ButtonPressed(&TypedEventHandler::<
            SystemMediaTransportControls,
            windows::Media::SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            if let Some(args) = args.as_ref()
                && let Some(command) = button_command(args.Button()?)
            {
                let _ = buttons.try_send(DesktopEvent::Media(command));
            }
            Ok(())
        }))?;
        let seeks = events.clone();
        let seek =
            match controls.PlaybackPositionChangeRequested(&TypedEventHandler::<
                SystemMediaTransportControls,
                windows::Media::PlaybackPositionChangeRequestedEventArgs,
            >::new(move |_, args| {
                if let Some(args) = args.as_ref() {
                    let ticks = args.RequestedPlaybackPosition()?.Duration;
                    let _ = seeks.try_send(DesktopEvent::Media(MediaCommand::Seek(
                        (ticks.max(0) / 10_000) as u64,
                    )));
                }
                Ok(())
            })) {
                Ok(token) => token,
                Err(error) => {
                    let _ = controls.RemoveButtonPressed(button);
                    return Err(error.into());
                }
            };
        Ok(Self {
            controls,
            button,
            seek,
            previous: None,
            events,
        })
    }

    pub fn update(&mut self, state: &MediaState) -> Result<()> {
        if self.previous.as_ref() == Some(state) {
            return Ok(());
        }
        self.controls.SetIsEnabled(state.track.is_some())?;
        if self
            .previous
            .as_ref()
            .is_none_or(|last| last.track != state.track)
        {
            let display = self.controls.DisplayUpdater()?;
            // Clear the preceding cover even if the next track has no artwork.
            display.ClearAll()?;
            if let Some(track) = &state.track {
                display.SetType(MediaPlaybackType::Music)?;
                let music = display.MusicProperties()?;
                music.SetTitle(&HSTRING::from(&track.title))?;
                music.SetArtist(&HSTRING::from(&track.artist))?;
                music.SetAlbumTitle(&HSTRING::from(&track.album))?;
                if let Some(cover) = &track.cover {
                    match artwork(cover) {
                        Ok(artwork) => display.SetThumbnail(&artwork)?,
                        Err(error) => {
                            let _ = self.events.try_send(DesktopEvent::Warning(format!(
                                "SMTC artwork: {error:#}"
                            )));
                        }
                    }
                }
            }
            display.Update()?;
        }
        self.controls.SetPlaybackStatus(match state.state {
            TransportState::Playing => MediaPlaybackStatus::Playing,
            TransportState::Paused => MediaPlaybackStatus::Paused,
            TransportState::Stopped => MediaPlaybackStatus::Stopped,
        })?;
        let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
        timeline.SetStartTime(timespan(0))?;
        timeline.SetMinSeekTime(timespan(0))?;
        timeline.SetEndTime(timespan(state.duration_ms))?;
        timeline.SetMaxSeekTime(timespan(state.duration_ms))?;
        timeline.SetPosition(timespan(state.position_ms.min(state.duration_ms)))?;
        self.controls.UpdateTimelineProperties(&timeline)?;
        self.previous = Some(state.clone());
        Ok(())
    }
}

/// Rust handles canonical/long Windows paths that StorageFile rejects. SMTC
/// receives an owned memory stream, independent of the cache file's lifetime.
/// Only called on the desktop worker, when the track/artwork changes.
fn artwork(path: &Path) -> Result<RandomAccessStreamReference> {
    // Cached covers are at most 512 px. Bound even a damaged/replaced cache file.
    const MAX_BYTES: u64 = 8 * 1024 * 1024;
    let mut bytes = Vec::new();
    File::open(path)
        .context("Cannot open cached artwork")?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .context("Cannot read cached artwork")?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "Cached artwork exceeds 8 MiB"
    );
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(&bytes)?;
    let stored = writer.StoreAsync()?.get()?;
    anyhow::ensure!(stored as usize == bytes.len(), "Incomplete artwork stream");
    writer.DetachStream()?;
    stream.Seek(0)?;
    Ok(RandomAccessStreamReference::CreateFromStream(&stream)?)
}

fn timespan(milliseconds: u64) -> TimeSpan {
    TimeSpan {
        Duration: milliseconds.saturating_mul(10_000).min(i64::MAX as u64) as i64,
    }
}

fn button_command(button: Button) -> Option<MediaCommand> {
    match button {
        Button::Play => Some(MediaCommand::Play),
        Button::Pause => Some(MediaCommand::Pause),
        Button::Stop => Some(MediaCommand::Stop),
        Button::Previous => Some(MediaCommand::Previous),
        Button::Next => Some(MediaCommand::Next),
        _ => None,
    }
}

impl Drop for Media {
    fn drop(&mut self) {
        let _ = self.controls.RemoveButtonPressed(self.button);
        let _ = self
            .controls
            .RemovePlaybackPositionChangeRequested(self.seek);
        let _ = self.controls.SetPlaybackStatus(MediaPlaybackStatus::Closed);
        if let Ok(display) = self.controls.DisplayUpdater() {
            let _ = display.ClearAll();
            let _ = display.Update();
        }
        let _ = self.controls.SetIsEnabled(false);
    }
}

#[cfg(test)]
#[path = "tests/media.rs"]
mod tests;
