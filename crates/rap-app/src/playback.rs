//! Playback commands from the interface. The engine runs on its own threads.
use crate::{app::Rap, i18n::Text};
use gpui::*;
use rap_audio::{Command, PlaybackState};
use rap_core::model::Album;
use rap_library::Cancellation;
use std::path::PathBuf;

/// Upper bound of files read for one drop or open.
const LOOSE_LIMIT: usize = 2000;

impl Rap {
    pub fn audio_command(&mut self, command: Command, cx: &mut Context<Self>) {
        if let Some(audio) = &self.audio
            && let Err(error) = audio.send(command)
        {
            self.report_error(error.to_string());
        }
        cx.notify();
    }

    /// Plays one edition of an album from `start`, skipping unavailable tracks.
    pub fn play_album(&mut self, id: i64, edition: usize, start: usize, cx: &mut Context<Self>) {
        let Some(edition) = self
            .snapshot
            .albums
            .iter()
            .find(|a| a.id == id)
            .and_then(|album| album.editions.get(edition))
        else {
            return;
        };
        if edition.tracks.get(start).is_none_or(|t| !t.available) {
            return;
        }
        let start_id = edition.tracks[start].id;
        let tracks: Vec<_> = edition
            .tracks
            .iter()
            .filter(|t| t.available)
            .cloned()
            .collect();
        let start = tracks.iter().position(|t| t.id == start_id).unwrap_or(0);
        self.audio_command(Command::Play { tracks, start }, cx);
    }

    pub fn play_pause(&mut self, cx: &mut Context<Self>) {
        if self.playback.state == PlaybackState::Playing {
            self.audio_command(Command::Pause, cx);
        } else if self.playback.track.is_some() {
            self.audio_command(Command::Resume, cx);
        } else if let Some(id) = self
            .selected
            .or_else(|| self.visible.first().map(|&i| self.snapshot.albums[i].id))
        {
            let edition = if self.selected == Some(id) {
                self.selected_edition
            } else {
                0
            };
            let start = self
                .snapshot
                .albums
                .iter()
                .find(|a| a.id == id)
                .and_then(|a| a.editions.get(edition))
                .and_then(|e| e.tracks.iter().position(|t| t.available));
            if let Some(start) = start {
                self.play_album(id, edition, start, cx);
            }
        }
    }

    /// `save` persists the value; dragging only sends it to the engine.
    pub fn set_volume(&mut self, value: f32, save: bool, cx: &mut Context<Self>) {
        self.settings.volume = value.clamp(0., 1.);
        self.muted_volume = None;
        if save {
            self.save();
        }
        self.audio_command(Command::Volume(self.settings.volume), cx);
    }

    pub fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        let restored = self.muted_volume.take();
        match restored {
            Some(volume) => self.set_volume(volume, true, cx),
            None if self.settings.volume > 0. => {
                let volume = self.settings.volume;
                self.set_volume(0., true, cx);
                self.muted_volume = Some(volume);
            }
            None => self.set_volume(0.5, true, cx),
        }
    }

    /// Position slider: previews while dragging, seeks once released.
    pub fn seek_fraction(&mut self, fraction: f32, released: bool, cx: &mut Context<Self>) {
        if released {
            self.seek_preview = None;
            let target = (self.playback.duration_ms as f64 * f64::from(fraction)) as u64;
            self.audio_command(Command::Seek(target), cx);
        } else {
            self.seek_preview = Some(fraction);
            cx.notify();
        }
    }

    /// Album of the current track, when it comes from the library.
    pub fn current_album(&self) -> Option<&Album> {
        let id = self.playback.track.as_ref()?.id;
        self.snapshot
            .albums
            .iter()
            .find(|album| album.tracks().any(|t| t.id == id))
    }

    pub fn open_file(&mut self, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(self.t(Text::OpenFile).into()),
        });
        cx.spawn(async move |this, cx| match prompt.await {
            Ok(Ok(Some(paths))) => {
                let _ = this.update(cx, |this, cx| this.play_paths(paths, cx));
            }
            Ok(Ok(None)) => {}
            error => {
                let _ = this.update(cx, |this, cx| {
                    this.report_error(format!("{error:?}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// One-off playback of files and folders, read in the background and never
    /// added to the library. A newer request cancels the previous one.
    pub fn play_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if let Some(previous) = self.preparing.take() {
            previous.cancel();
        }
        let cancel = Cancellation::default();
        self.preparing = Some(cancel.clone());
        let covers = self.loose_covers.clone();
        let task = cx.background_executor().spawn({
            let cancel = cancel.clone();
            async move { rap_library::read_loose_files(&paths, &covers, &cancel, LOOSE_LIMIT) }
        });
        cx.spawn(async move |this, cx| {
            let loose = task.await;
            let _ = this.update(cx, |this, cx| {
                if cancel.is_cancelled() {
                    return;
                }
                this.preparing = None;
                for error in loose.errors {
                    this.report_warning(error);
                }
                if loose.truncated {
                    this.report_warning(this.t(Text::Truncated));
                }
                if loose.tracks.is_empty() {
                    this.report_warning(this.t(Text::NothingToPlay));
                    cx.notify();
                } else {
                    let tracks = loose.tracks;
                    this.audio_command(Command::Play { tracks, start: 0 }, cx);
                }
            });
        })
        .detach();
        cx.notify();
    }
}
