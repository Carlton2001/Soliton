//! Keyboard shortcuts of the main window. Text fields keep their own bindings.
use crate::app::{Rap, Section};
use gpui::*;
use rap_audio::Command;

/// Step of the keyboard seek shortcuts.
const SEEK_STEP_MS: u64 = 10_000;

impl Rap {
    pub fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let control = event.keystroke.modifiers.control;
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
                "enter" => {
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
                if !matches!(self.section, Section::Albums | Section::Artists) {
                    self.navigate(Section::Albums, cx);
                }
                self.search.read(cx).focus_handle(cx).focus(window);
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
                self.set_ui_rem_size(rap_core::config::Settings::DEFAULT_REM_SIZE, cx);
                return;
            }
            "f11" => {
                window.toggle_fullscreen();
                return;
            }
            "escape" => {
                if window.is_fullscreen() {
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
        match key {
            "space" => return self.play_pause(cx),
            "left" | "right" if control => {
                let position = self.playback.position_ms;
                let target = if key == "left" {
                    position.saturating_sub(SEEK_STEP_MS)
                } else {
                    position.saturating_add(SEEK_STEP_MS)
                };
                return self.audio_command(Command::Seek(target), cx);
            }
            _ => {}
        }
        let browsing =
            self.section == Section::Albums && self.selected.is_none() && self.menu.is_none();
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
