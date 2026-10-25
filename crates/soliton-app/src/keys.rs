//! Keyboard shortcuts of the main window. Text fields keep their own bindings.
use crate::app::{Section, Soliton};
use gpui::*;
use soliton_audio::Command;

/// Step of the keyboard seek shortcuts.
const SEEK_STEP_MS: u64 = 10_000;
/// Repeated keys within this delay step from the previous target: the engine
/// reports positions later than keys repeat.
const SEEK_REPEAT: std::time::Duration = std::time::Duration::from_millis(500);

impl Soliton {
    pub fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let control = event.keystroke.modifiers.control;
        if key == "tab" {
            for _ in 0..512 {
                if event.keystroke.modifiers.shift {
                    window.focus_prev();
                } else {
                    window.focus_next();
                }
                if !(self.confirm_close || self.playlists.dialog.is_some())
                    || self.modal_focus.contains_focused(window, cx)
                {
                    break;
                }
            }
            cx.stop_propagation();
            return;
        }
        if self.confirm_close {
            if key == "escape" {
                self.confirm_close = false;
                cx.notify();
            }
            return;
        }
        if self.playlists.dialog.is_some() {
            match key {
                "escape" => {
                    self.playlists.dialog = None;
                    self.focus.focus(window);
                    cx.notify();
                }
                "enter"
                    if self
                        .playlists
                        .name
                        .read(cx)
                        .focus_handle(cx)
                        .is_focused(window)
                        || self.focus.is_focused(window) =>
                {
                    self.submit_playlist_dialog(cx);
                    if self.playlists.dialog.is_none() {
                        self.focus.focus(window);
                    }
                }
                _ => {}
            }
            return;
        }
        match key {
            "f" if control => {
                self.now_playing = false;
                self.selected = None;
                self.menu = None;
                self.track_menu = None;
                if !matches!(self.section, Section::Albums | Section::Artists) {
                    self.navigate(Section::Albums, cx);
                }
                self.search.read(cx).focus_handle(cx).focus(window);
                cx.notify();
                return;
            }
            "left" if event.keystroke.modifiers.alt => {
                self.go_back(cx);
                self.focus.focus(window);
                return;
            }
            "right" if event.keystroke.modifiers.alt => {
                self.go_forward(cx);
                self.focus.focus(window);
                return;
            }
            "+" | "=" | "add" if control => {
                self.set_ui_rem_size(self.settings.ui_rem_size + 1, cx);
                return;
            }
            "-" | "subtract" if control => {
                self.set_ui_rem_size(self.settings.ui_rem_size.saturating_sub(1), cx);
                return;
            }
            "0" if control => {
                self.set_ui_rem_size(soliton_core::config::Settings::DEFAULT_REM_SIZE, cx);
                return;
            }
            "f11" => {
                window.toggle_fullscreen();
                return;
            }
            "escape" => {
                if self.volume_popup {
                    self.volume_popup = false;
                    cx.notify();
                } else if window.is_fullscreen() {
                    window.toggle_fullscreen();
                } else {
                    self.back(cx);
                }
                self.focus.focus(window);
                return;
            }
            _ => {}
        }
        let typing = [&self.search, &self.menu_search, &self.playlists.name]
            .iter()
            .any(|input| input.read(cx).focus_handle(cx).is_focused(window));
        if typing {
            return;
        }
        // Focused controls receive GPUI's Enter/Space click on key-up.
        if !self.focus.is_focused(window) {
            return;
        }
        match key {
            "space" => return self.play_pause(cx),
            "up" | "down" if control => {
                let delta = if key == "up" { 0.05 } else { -0.05 };
                self.set_volume(self.settings.volume + delta, true, cx);
                return;
            }
            "left" | "right" if control => {
                let position = self
                    .seek_target
                    .filter(|(_, sent)| sent.elapsed() < SEEK_REPEAT)
                    .map_or(self.playback.position_ms, |(target, _)| target);
                let mut target = if key == "left" {
                    position.saturating_sub(SEEK_STEP_MS)
                } else {
                    position.saturating_add(SEEK_STEP_MS)
                };
                if self.playback.duration_ms > 0 {
                    target = target.min(self.playback.duration_ms);
                }
                self.seek_target = Some((target, std::time::Instant::now()));
                return self.audio_command(Command::Seek(target), cx);
            }
            _ => {}
        }
        if self.section == Section::Artists
            && !self.now_playing
            && self.menu.is_none()
            && matches!(key, "left" | "right" | "up" | "down" | "enter")
        {
            let query = soliton_core::text::fold(&self.search.read(cx).content);
            let names: Vec<_> = self
                .facets
                .artist_cards
                .iter()
                .filter(|card| {
                    query.is_empty() || soliton_core::text::fold(&card.name).contains(&query)
                })
                .map(|card| card.name.clone())
                .collect();
            if names.is_empty() {
                return;
            }
            let step = if self.settings.artist_grid {
                self.columns.max(1)
            } else {
                1
            };
            let last = names.len() - 1;
            self.keyboard_index = self.keyboard_index.min(last);
            self.keyboard_index = match key {
                "left" => self.keyboard_index.saturating_sub(1),
                "right" => (self.keyboard_index + 1).min(last),
                "up" => self.keyboard_index.saturating_sub(step),
                "down" => (self.keyboard_index + step).min(last),
                "enter" => {
                    self.show_artist(names[self.keyboard_index].clone(), cx);
                    return;
                }
                _ => return,
            };
            self.facet_scroll
                .scroll_to_item(self.keyboard_index / step, ScrollStrategy::Center);
            cx.notify();
            return;
        }
        let browsing = self.section == Section::Albums
            && !self.now_playing
            && (self.selected.is_none()
                || crate::collection::split_collection(
                    crate::navigation::content_width(window),
                    self.section,
                    self.selected.is_some(),
                ))
            && self.menu.is_none();
        if !browsing || self.visible.is_empty() {
            return;
        }
        let step = if self.settings.album_grid {
            self.columns
        } else {
            1
        };
        let last = self.visible.len() - 1;
        self.keyboard_index = match key {
            "left" => self.keyboard_index.saturating_sub(1),
            "right" => (self.keyboard_index + 1).min(last),
            "up" => self.keyboard_index.saturating_sub(step),
            "down" => (self.keyboard_index + step).min(last),
            "enter" => {
                let id = self.snapshot.albums[self.visible[self.keyboard_index.min(last)]].id;
                self.open_album(id, cx);
                return;
            }
            _ => return,
        };
        self.album_scroll
            .scroll_to_item(self.keyboard_index / step, ScrollStrategy::Center);
        cx.notify();
    }
}
