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
            // Dialogs, and menus holding the focus, keep Tab within them.
            let trap = if self.confirm_close || self.playlists.dialog.is_some() {
                Some(self.modal_focus.clone())
            } else {
                Some(self.menu_focus.clone())
                    .filter(|menu| self.menu_open() && menu.contains_focused(window, cx))
            };
            for _ in 0..512 {
                if event.keystroke.modifiers.shift {
                    window.focus_prev();
                } else {
                    window.focus_next();
                }
                if trap
                    .as_ref()
                    .is_none_or(|trap| trap.contains_focused(window, cx))
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
        // The full screen listening mode hides the library and its shortcuts.
        let listening = window.is_fullscreen();
        match key {
            "f" if control && !listening => {
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
            "left" if event.keystroke.modifiers.alt && !listening => {
                self.go_back(cx);
                self.focus.focus(window);
                return;
            }
            "right" if event.keystroke.modifiers.alt && !listening => {
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
            _ if control && digit(key) == Some(0) => {
                self.set_ui_rem_size(soliton_core::config::Settings::DEFAULT_REM_SIZE, cx);
                return;
            }
            // Sections in the order of the navigation bar.
            _ if control && !listening && matches!(digit(key), Some(1..=5)) => {
                let section = match digit(key) {
                    Some(1) => Section::Artists,
                    Some(2) => Section::Albums,
                    Some(3) => Section::Playlists,
                    Some(4) => Section::Queue,
                    _ => Section::Settings,
                };
                if section == Section::Playlists {
                    self.playlists.selected = None;
                }
                self.navigate(section, cx);
                self.focus.focus(window);
                return;
            }
            "f11" => {
                self.volume_popup = false;
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
                // Focus inside a closing menu returns to its opener instead.
                if !self.menu_focus.contains_focused(window, cx) {
                    self.focus.focus(window);
                }
                return;
            }
            _ => {}
        }
        if self.menu_key(event, window, cx) {
            return;
        }
        let typing = [&self.search, &self.menu_search, &self.playlists.name]
            .iter()
            .any(|input| input.read(cx).focus_handle(cx).is_focused(window));
        if typing {
            return;
        }
        // Focused controls and list items receive GPUI's Enter/Space click on key-up.
        let root = self.focus.is_focused(window);
        if !root && self.focused_list(window).is_none() {
            return;
        }
        match key {
            "space" if root => return self.play_pause(cx),
            "m" if root && !control => return self.toggle_mute(cx),
            "s" if root && !control => return self.set_shuffle(!self.playback.shuffle, cx),
            "r" if root && !control => return self.cycle_repeat(cx),
            "left" | "right" if control && !event.keystroke.modifiers.shift => {
                let command = if key == "left" {
                    Command::Previous
                } else {
                    Command::Next
                };
                return self.audio_command(command, cx);
            }
            "up" | "down" if control => return self.step_volume(key == "up", cx),
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
        if !listening {
            self.list_key(&event.keystroke, window, cx);
        }
    }
}

/// Digit of a key on the main row or the keypad. Windows names main row keys
/// after the layout's unshifted character, as on AZERTY.
fn digit(key: &str) -> Option<u8> {
    const AZERTY: [&str; 10] = ["à", "&", "é", "\"", "'", "(", "-", "è", "_", "ç"];
    key.parse()
        .ok()
        .filter(|&digit| digit < 10)
        .or_else(|| AZERTY.iter().position(|&k| k == key).map(|d| d as u8))
}

#[cfg(test)]
#[path = "tests/keys.rs"]
mod tests;
