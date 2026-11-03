//! Keyboard use of the open menu: focus enters it when it is opened from the
//! keyboard, arrows move between its items and focus returns to its opener.
use crate::{app::Soliton, track_list::Selection};
use gpui::*;

impl Soliton {
    pub(super) fn menu_open(&self) -> bool {
        self.menu.is_some() || self.track_menu.is_some()
    }

    /// Called at each render to follow menus opening and closing.
    pub(super) fn sync_menu_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open = self.menu_open();
        if open == self.menu_was_open {
            return;
        }
        self.menu_was_open = open;
        if open {
            // Pointer presses keep the root focused; a focused control means the keyboard.
            self.menu_opener = window
                .focused(cx)
                .filter(|_| !self.focus.is_focused(window));
            if self.menu_opener.is_some() {
                cx.on_next_frame(window, |this, window, cx| this.step_menu(true, window, cx));
            }
        } else if let Some(opener) = self.menu_opener.take() {
            // Focus left inside the closed menu returns to its opener.
            if self.menu_focus.contains_focused(window, cx) {
                cx.on_next_frame(window, move |_, window, _| window.focus(&opener));
            }
        }
    }

    /// Moves focus to the next or previous item of the open menu.
    fn step_menu(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        for _ in 0..512 {
            if forward {
                window.focus_next();
            } else {
                window.focus_prev();
            }
            if self.menu_focus.contains_focused(window, cx) {
                break;
            }
        }
    }

    /// Handles a key while a menu is open; returns whether it was used.
    pub(super) fn menu_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = event.keystroke.key.as_str();
        if !self.menu_open() {
            // Shift+F10 or the Menu key open the focused album track's menu.
            let menu_key = key == "menu" || (key == "f10" && event.keystroke.modifiers.shift);
            if let Some(selection @ Selection::Album { .. }) =
                self.track_focus.focused(window).filter(|_| menu_key)
            {
                self.track_menu = Some((selection, false));
                cx.notify();
                return true;
            }
            return false;
        }
        match key {
            "up" | "down" => {
                self.step_menu(key == "down", window, cx);
                true
            }
            _ => false,
        }
    }
}
