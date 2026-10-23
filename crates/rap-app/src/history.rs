//! Back and forward navigation between the places the user has visited,
//! driven by the mouse side buttons.
use crate::{
    app::{Rap, Section},
    catalog::Filter,
};
use gpui::*;
use rap_core::config::{LastView, View};

/// Places kept in each direction.
const DEPTH: usize = 50;

/// What the user sees, independently of the search text being typed and of
/// display settings.
#[derive(Clone, PartialEq)]
pub struct Place {
    home_artist: Option<String>,
    playlist: Option<i64>,
    section: Section,
    album: Option<(i64, usize)>,
    filter: Filter,
}

#[derive(Default)]
pub struct History {
    current: Option<Place>,
    back: Vec<Place>,
    forward: Vec<Place>,
}

impl History {
    /// Records `place` when it differs from the current one. A new place
    /// discards the forward path, as in a browser.
    /// Returns whether the place changed.
    pub fn observe(&mut self, place: Place) -> bool {
        if self.current.as_ref() == Some(&place) {
            return false;
        }
        if let Some(previous) = self.current.replace(place) {
            self.back.push(previous);
            if self.back.len() > DEPTH {
                self.back.remove(0);
            }
        }
        self.forward.clear();
        true
    }

    pub fn back(&mut self) -> Option<Place> {
        let place = self.back.pop()?;
        if let Some(current) = self.current.replace(place.clone()) {
            self.forward.push(current);
        }
        Some(place)
    }

    pub fn forward(&mut self) -> Option<Place> {
        let place = self.forward.pop()?;
        if let Some(current) = self.current.replace(place.clone()) {
            self.back.push(current);
        }
        Some(place)
    }
}

impl Rap {
    fn place(&self) -> Place {
        Place {
            home_artist: (self.section == Section::Home)
                .then(|| self.home.artist.clone())
                .flatten(),
            playlist: if self.section == Section::Playlists {
                self.playlists.selected
            } else {
                None
            },
            section: self.section,
            album: self.selected.map(|id| (id, self.selected_edition)),
            filter: Filter {
                query: String::new(),
                sort: Default::default(),
                hide_unavailable: false,
                ..self.filter.clone()
            },
        }
    }

    /// Called before each render, so every way of moving is recorded. The
    /// page is also saved, to reopen the application where it was left.
    pub fn record_place(&mut self) {
        let place = self.place();
        if !self.history.observe(place) {
            return;
        }
        let last = LastView {
            playlist: self.playlists.selected,
            view: self.section.into(),
            album: self.selected,
            artist: if self.section == Section::Home {
                self.home.artist.clone()
            } else {
                self.filter.artist.clone()
            },
        };
        if last != self.settings.last_view {
            self.settings.last_view = last;
            self.save();
        }
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        // An open menu is closed first, like the Escape key does.
        if self.menu.take().is_some() || self.track_menu.take().is_some() {
            cx.notify();
        } else if let Some(place) = self.history.back() {
            self.restore(place, cx);
        }
    }

    pub fn go_forward(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        if let Some(place) = self.history.forward() {
            self.restore(place, cx);
        }
    }

    fn restore(&mut self, place: Place, cx: &mut Context<Self>) {
        let filter_changed = self.place().filter != place.filter;
        self.track_menu = None;
        self.section = place.section;
        if self.section == Section::Home {
            self.home.artist = place.home_artist;
        }
        self.playlists.selected = place.playlist;
        self.selected = place.album.map(|(id, _)| id);
        self.selected_edition = place.album.map_or(0, |(_, edition)| edition);
        if filter_changed {
            // The search text and display settings stay as they are.
            self.filter = Filter {
                query: std::mem::take(&mut self.filter.query),
                sort: self.filter.sort,
                hide_unavailable: self.filter.hide_unavailable,
                ..place.filter
            };
            self.refresh(cx);
        }
        cx.notify();
    }
}

impl From<View> for Section {
    fn from(view: View) -> Self {
        match view {
            View::Home => Section::Home,
            View::Artists => Section::Artists,
            View::Albums => Section::Albums,
            View::Playlists => Section::Playlists,
            View::Queue => Section::Queue,
            View::Settings => Section::Settings,
        }
    }
}

impl From<Section> for View {
    fn from(section: Section) -> Self {
        match section {
            Section::Home => View::Home,
            Section::Artists => View::Artists,
            Section::Albums => View::Albums,
            Section::Playlists => View::Playlists,
            Section::Queue => View::Queue,
            Section::Settings => View::Settings,
        }
    }
}

#[cfg(test)]
#[path = "tests/history.rs"]
mod tests;
