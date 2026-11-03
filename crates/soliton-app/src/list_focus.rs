//! Keyboard focus of virtualized list items. Handles are kept by item key, so
//! arrow keys can focus an item before the list scrolls it into view.
use gpui::{App, FocusHandle, Window};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
};

/// Handles beyond this count are dropped, except the last focused one.
const KEPT: usize = 1024;

pub struct ListFocus<K> {
    handles: RefCell<HashMap<K, FocusHandle>>,
    /// Items rendered since the last frame began.
    rendered: RefCell<HashSet<K>>,
    /// Item last focused or clicked, where arrow keys resume.
    pub last: Option<K>,
}

impl<K> Default for ListFocus<K> {
    fn default() -> Self {
        Self {
            handles: RefCell::default(),
            rendered: RefCell::default(),
            last: None,
        }
    }
}

impl<K: Clone + Eq + Hash> ListFocus<K> {
    /// Handle tracked by the item's element while it is rendered. Tab stops
    /// only on the last focused item, or on the `entry` item when the last one
    /// is elsewhere, so Tab moves past the list; arrows move within it.
    pub fn handle(&self, key: &K, entry: bool, cx: &mut App) -> FocusHandle {
        let stop = entry || self.last.as_ref() == Some(key);
        self.rendered.borrow_mut().insert(key.clone());
        let mut handles = self.handles.borrow_mut();
        if let Some(handle) = handles.get(key) {
            return handle.clone().tab_stop(stop);
        }
        if handles.len() >= KEPT {
            handles.retain(|kept, _| self.last.as_ref() == Some(kept));
        }
        let handle = cx.focus_handle();
        handles.insert(key.clone(), handle.clone());
        handle.tab_stop(stop)
    }

    /// Whether the last focused item is not among `keys`.
    pub fn left_elsewhere<'a>(&self, mut keys: impl Iterator<Item = &'a K>) -> bool
    where
        K: 'a,
    {
        self.last
            .as_ref()
            .is_none_or(|last| !keys.any(|key| key == last))
    }

    pub fn begin_frame(&self) {
        self.rendered.borrow_mut().clear();
    }

    /// Whether the focused item was scrolled out of the rendered rows, where
    /// it would no longer receive keys.
    pub fn focus_hidden(&self, window: &Window) -> bool {
        self.focused(window)
            .is_some_and(|key| !self.rendered.borrow().contains(&key))
    }

    pub fn focused(&self, window: &Window) -> Option<K> {
        self.handles
            .borrow()
            .iter()
            .find(|(_, handle)| handle.is_focused(window))
            .map(|(key, _)| key.clone())
    }

    pub fn focus(&mut self, key: K, window: &mut Window, cx: &mut App) {
        window.focus(&self.handle(&key, false, cx));
        self.last = Some(key);
    }
}

/// Index reached by an arrow, Home, End or page key in a list of `len` items
/// laid out in rows of `columns`, `page` rows being visible; `None` for other keys.
pub fn moved(index: usize, len: usize, columns: usize, page: usize, key: &str) -> Option<usize> {
    let last = len.checked_sub(1)?;
    let index = index.min(last);
    let columns = columns.max(1);
    let page = columns * page.max(1);
    Some(match key {
        "left" => index.saturating_sub(1),
        "right" => (index + 1).min(last),
        "up" => index.saturating_sub(columns),
        "down" => (index + columns).min(last),
        "pageup" => index.saturating_sub(page),
        "pagedown" => (index + page).min(last),
        "home" => 0,
        "end" => last,
        _ => return None,
    })
}

/// Next item after `from` whose name starts with `letter`, wrapping around,
/// accents and case ignored.
pub fn next_with_letter<'a>(
    names: impl ExactSizeIterator<Item = &'a str> + Clone,
    from: Option<usize>,
    letter: char,
) -> Option<usize> {
    let len = names.len();
    let start = from.map_or(0, |from| from + 1);
    (0..len).map(|step| (start + step) % len).find(|&index| {
        names
            .clone()
            .nth(index)
            .is_some_and(|name| soliton_core::text::fold(name).chars().next() == Some(letter))
    })
}

/// Rows of a uniform list fitting its viewport, from the last frame's geometry.
pub fn page_rows(scroll: &gpui::UniformListScrollHandle, rows: usize) -> usize {
    let state = scroll.0.borrow();
    let Some(size) = state.last_item_size.filter(|_| rows > 0) else {
        return 1;
    };
    let row = f32::from(size.contents.height) / rows as f32;
    let height = f32::from(state.base_handle.bounds().size.height);
    if row <= 0. {
        return 1;
    }
    ((height / row).floor() as usize).saturating_sub(1).max(1)
}

#[cfg(test)]
#[path = "tests/list_focus.rs"]
mod tests;
