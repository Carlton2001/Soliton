//! Keyboard focus of virtualized list items. Handles are kept by item key, so
//! arrow keys can focus an item before the list scrolls it into view.
use gpui::{App, FocusHandle, Window};
use std::{cell::RefCell, collections::HashMap, hash::Hash};

/// Handles beyond this count are dropped, except the last focused one.
const KEPT: usize = 1024;

pub struct ListFocus<K> {
    handles: RefCell<HashMap<K, FocusHandle>>,
    /// Item last focused or clicked, where arrow keys resume.
    pub last: Option<K>,
}

impl<K> Default for ListFocus<K> {
    fn default() -> Self {
        Self {
            handles: RefCell::default(),
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

/// Index reached by an arrow key in a list of `len` items laid out in rows of
/// `columns`, or `None` for other keys.
pub fn moved(index: usize, len: usize, columns: usize, key: &str) -> Option<usize> {
    let last = len.checked_sub(1)?;
    let index = index.min(last);
    let columns = columns.max(1);
    Some(match key {
        "left" => index.saturating_sub(1),
        "right" => (index + 1).min(last),
        "up" => index.saturating_sub(columns),
        "down" => (index + columns).min(last),
        _ => return None,
    })
}

#[cfg(test)]
#[path = "tests/list_focus.rs"]
mod tests;
