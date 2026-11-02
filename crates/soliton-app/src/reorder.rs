//! Long-press reordering of queue and playlist rows, alone or as a Shift-selected block.
use crate::{
    app::Soliton,
    track_list::{Selection, same_list},
};
use gpui::{prelude::*, *};
use soliton_audio::Command;
use soliton_core::playlist::PlaylistChange;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    time::Duration,
};

const LONG_PRESS: Duration = Duration::from_millis(350);
const AUTOSCROLL_TICK: Duration = Duration::from_millis(16);
/// Moving further than this during the press cancels it.
const PRESS_SLOP: f32 = 5.;

#[derive(Default)]
pub struct Reorder {
    pub marked: Vec<Selection>,
    pub drag: Option<Drag>,
    anchor: Option<Selection>,
    press: Option<(Selection, Point<Pixels>)>,
    /// The release ending a drag must not count as a click on its row.
    swallow_click: bool,
    task: Option<Task<()>>,
}

pub struct Drag {
    pub rows: Vec<Selection>,
    /// Position of the pressed row within the block, kept under the pointer.
    grab: usize,
    /// Block position among the other rows; none while the pointer is outside the list.
    start: Option<usize>,
    pointer: Option<(Point<Pixels>, Bounds<Pixels>)>,
}

/// Rows from one end of the selection to the other, in list order.
fn range_between<T: Copy + PartialEq>(rows: &[T], anchor: T, target: T) -> Vec<T> {
    let (Some(a), Some(b)) = (
        rows.iter().position(|&r| r == anchor),
        rows.iter().position(|&r| r == target),
    ) else {
        return vec![target];
    };
    rows[a.min(b)..=a.max(b)].to_vec()
}

/// The list as it would be with the block at `start` among the other rows.
fn preview<T: Copy + Eq + Hash>(rows: &[T], moved: &[T], start: usize) -> Vec<T> {
    let block: HashSet<T> = moved.iter().copied().collect();
    let mut order: Vec<T> = rows
        .iter()
        .filter(|r| !block.contains(r))
        .copied()
        .collect();
    let at = start.min(order.len());
    order.splice(at..at, moved.iter().copied());
    order
}

/// The row to insert the block before (`Some(None)` for the end of the list),
/// or `None` when the drop would leave the order unchanged.
fn plan_move<T: Copy + Eq + Hash>(rows: &[T], moved: &[T], start: usize) -> Option<Option<T>> {
    let order = preview(rows, moved, start);
    let others = order.len() - moved.len();
    let before = order.get(start.min(others) + moved.len()).copied();
    (order != rows).then_some(before)
}

/// Block position keeping the grabbed row under a pointer `y` relative to the list's top edge.
fn start_at(y: f32, offset: f32, row_height: f32, grab: usize, others: usize) -> usize {
    let row = ((y - offset) / row_height).floor().max(0.) as usize;
    row.saturating_sub(grab).min(others)
}

/// Whether the row at `at` starts and ends a run of consecutive moved rows.
fn run_edges<T: PartialEq>(displayed: &[T], moved: &[T], at: usize) -> (bool, bool) {
    let outside = |row: Option<&T>| row.is_none_or(|row| !moved.contains(row));
    let previous = at.checked_sub(1).and_then(|i| displayed.get(i));
    (outside(previous), outside(displayed.get(at + 1)))
}

/// Adds or removes a row from a selection.
fn toggled<T: Copy + PartialEq>(marked: &[T], row: T) -> Vec<T> {
    if marked.contains(&row) {
        marked.iter().copied().filter(|&r| r != row).collect()
    } else {
        marked.iter().copied().chain([row]).collect()
    }
}

/// Entry ids grouped by album in order of first appearance, each in its library
/// order; entries unknown to the library stay last, as they are.
fn original_order<T: Copy>(entries: &[(T, Option<(usize, usize)>)]) -> Vec<T> {
    let mut albums = HashMap::new();
    let mut keyed: Vec<_> = entries
        .iter()
        .map(|&(id, place)| match place {
            Some((album, position)) => {
                let next = albums.len();
                ((*albums.entry(album).or_insert(next), position), id)
            }
            None => ((usize::MAX, 0), id),
        })
        .collect();
    keyed.sort_by_key(|&(key, _)| key);
    keyed.into_iter().map(|(_, id)| id).collect()
}

impl Soliton {
    /// Edition and position in the library of the given tracks.
    fn album_places(&self, tracks: HashSet<i64>) -> HashMap<i64, (usize, usize)> {
        let mut places = HashMap::with_capacity(tracks.len());
        let editions = self.snapshot.albums.iter().flat_map(|a| a.editions.iter());
        for (edition_index, edition) in editions.enumerate() {
            for (position, track) in edition.tracks.iter().enumerate() {
                if tracks.contains(&track.id) {
                    places.insert(track.id, (edition_index, position));
                }
            }
        }
        places
    }

    /// The queue's original order, when manual edits or shuffling have changed it.
    pub(crate) fn original_queue_order(&self) -> Option<Vec<u64>> {
        let entries = &self.playback.queue.entries;
        if entries.len() < 2 {
            return None;
        }
        let places = self.album_places(entries.iter().map(|e| e.track.id).collect());
        let keyed: Vec<_> = entries
            .iter()
            .map(|e| (e.id, places.get(&e.track.id).copied()))
            .collect();
        let current: Vec<u64> = entries.iter().map(|e| e.id).collect();
        Some(original_order(&keyed)).filter(|order| *order != current)
    }

    fn list_rows(&self, selection: Selection) -> Vec<Selection> {
        match selection {
            Selection::Album { .. } => Vec::new(),
            Selection::Queue(_) => self
                .playback
                .queue
                .entries
                .iter()
                .map(|e| Selection::Queue(e.id))
                .collect(),
            Selection::Playlist { list, .. } => self
                .playlists
                .lists
                .iter()
                .find(|p| p.id == list)
                .map_or_else(Vec::new, |p| {
                    p.entries
                        .iter()
                        .map(|e| Selection::Playlist { list, entry: e.id })
                        .collect()
                }),
        }
    }

    /// Source indices in displayed order while a drag previews this list's new order.
    pub(crate) fn reorder_preview(&self, list: Selection) -> Option<Vec<usize>> {
        let drag = self.reorder.drag.as_ref()?;
        let start = drag.start?;
        if !same_list(*drag.rows.first()?, list) {
            return None;
        }
        let rows = self.list_rows(list);
        let block: HashSet<Selection> = drag.rows.iter().copied().collect();
        let indices: Vec<usize> = (0..rows.len()).collect();
        let moved: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| block.contains(&rows[i]))
            .collect();
        Some(preview(&indices, &moved, start))
    }

    /// For a dragged row, whether it opens and closes its displayed run of dragged rows.
    pub(crate) fn drag_frame(&self, row: Selection) -> Option<(bool, bool)> {
        let block = &self.reorder.drag.as_ref()?.rows;
        if !block.contains(&row) {
            return None;
        }
        let rows = self.list_rows(row);
        let displayed: Vec<Selection> = match self.reorder_preview(row) {
            Some(order) => order.into_iter().map(|i| rows[i]).collect(),
            None => rows,
        };
        let at = displayed.iter().position(|&r| r == row)?;
        Some(run_edges(&displayed, block, at))
    }

    pub(crate) fn reorder_press(
        &mut self,
        selection: Selection,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        let reorder = &mut self.reorder;
        reorder.swallow_click = false;
        reorder.drag = None;
        reorder.press = Some((selection, event.position));
        if event.modifiers.shift {
            let rows = self.list_rows(selection);
            let reorder = &mut self.reorder;
            // Without a visible selection, this row starts a new one.
            let anchor = reorder
                .anchor
                .filter(|a| !reorder.marked.is_empty() && rows.contains(a))
                .unwrap_or(selection);
            reorder.anchor = Some(anchor);
            reorder.marked = range_between(&rows, anchor, selection);
        }
        self.reorder.task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LONG_PRESS).await;
            if this.update(cx, |this, cx| this.start_drag(cx)).is_err() {
                return;
            }
            loop {
                cx.background_executor().timer(AUTOSCROLL_TICK).await;
                if !this
                    .update(cx, |this, cx| this.autoscroll(cx))
                    .unwrap_or(false)
                {
                    break;
                }
            }
        }));
        cx.notify();
    }

    /// Whether a click reaches the row. Ctrl adds or removes the row from the
    /// selection. A plain click selects only its row, or
    /// deselects it; a double click, which plays, leaves nothing selected.
    pub(crate) fn reorder_click(
        &mut self,
        selection: Selection,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if std::mem::take(&mut self.reorder.swallow_click) || event.modifiers().shift {
            return false;
        }
        if event.modifiers().secondary() {
            self.reorder.marked = toggled(&self.reorder.marked, selection);
            self.reorder.anchor = Some(selection);
            cx.notify();
            return false;
        }
        let alone = self.reorder.marked == [selection];
        if alone || event.click_count() > 1 {
            self.reorder.marked.clear();
            self.reorder.anchor = None;
        } else {
            self.reorder.marked = vec![selection];
            self.reorder.anchor = Some(selection);
        }
        cx.notify();
        true
    }

    fn start_drag(&mut self, cx: &mut Context<Self>) {
        let Some((selection, _)) = self.reorder.press.take() else {
            return;
        };
        let rows = if self.reorder.marked.contains(&selection) {
            let marked = &self.reorder.marked;
            self.list_rows(selection)
                .into_iter()
                .filter(|row| marked.contains(row))
                .collect()
        } else {
            self.reorder.marked.clear();
            vec![selection]
        };
        self.track_menu = None;
        let grab = rows.iter().position(|&r| r == selection).unwrap_or(0);
        self.reorder.drag = Some(Drag {
            rows,
            grab,
            start: None,
            pointer: None,
        });
        cx.notify();
    }

    fn drag_handle(&self) -> Option<&UniformListScrollHandle> {
        match self.reorder.drag.as_ref()?.rows.first()? {
            Selection::Album { .. } => None,
            Selection::Queue(_) => Some(&self.queue_scroll),
            Selection::Playlist { .. } => Some(&self.playlists.scroll),
        }
    }

    /// Row height and scrolling state as laid out by the list.
    fn drag_metrics(&self, count: usize) -> Option<(f32, f32, f32)> {
        let state = self.drag_handle()?.0.borrow();
        let size = state.last_item_size?;
        let row = f32::from(size.contents.height) / count.max(1) as f32;
        let most = (f32::from(size.contents.height) - f32::from(size.item.height)).max(0.);
        (row > 0.).then(|| (row, f32::from(state.base_handle.offset().y), most))
    }

    fn update_start(&mut self, cx: &mut Context<Self>) {
        let Some(first) = self.reorder.drag.as_ref().and_then(|d| d.rows.first()) else {
            return;
        };
        let count = self.list_rows(*first).len();
        let metrics = self.drag_metrics(count);
        let Some(drag) = self.reorder.drag.as_mut() else {
            return;
        };
        let others = count.saturating_sub(drag.rows.len());
        let grab = drag.grab;
        let start = drag
            .pointer
            .zip(metrics)
            .and_then(|((at, bounds), (row, offset, _))| {
                bounds
                    .contains(&at)
                    .then(|| start_at(f32::from(at.y - bounds.top()), offset, row, grab, others))
            });
        if drag.start != start {
            drag.start = start;
            cx.notify();
        }
    }

    /// Scrolls while the pointer rests near an edge; false ends the drag's task.
    fn autoscroll(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(drag) = self.reorder.drag.as_ref() else {
            return false;
        };
        let (Some(first), Some((at, bounds))) = (drag.rows.first().copied(), drag.pointer) else {
            return true;
        };
        let count = self.list_rows(first).len();
        let Some((row, offset, most)) = self.drag_metrics(count) else {
            return true;
        };
        let y = f32::from(at.y);
        let step = if !bounds.contains(&at) {
            0.
        } else if y < f32::from(bounds.top()) + row {
            row / 4.
        } else if y > f32::from(bounds.bottom()) - row {
            -row / 4.
        } else {
            0.
        };
        let next = (offset + step).clamp(-most, 0.);
        if next != offset {
            if let Some(handle) = self.drag_handle() {
                let state = handle.0.borrow();
                let x = state.base_handle.offset().x;
                state.base_handle.set_offset(point(x, px(next)));
            }
            self.update_start(cx);
            cx.notify();
        }
        true
    }

    fn reorder_moved(
        &mut self,
        event: &MouseMoveEvent,
        bounds: Bounds<Pixels>,
        cx: &mut Context<Self>,
    ) {
        // A release outside the window is only seen as a move without the button.
        if event.pressed_button != Some(MouseButton::Left) {
            self.cancel_reorder(cx);
            return;
        }
        if let Some((_, start)) = self.reorder.press {
            let moved = (event.position - start).magnitude() as f32;
            if moved > crate::scale::scaled(PRESS_SLOP) {
                self.cancel_reorder(cx);
            }
        } else if let Some(drag) = self.reorder.drag.as_mut() {
            drag.pointer = Some((event.position, bounds));
            self.update_start(cx);
        }
    }

    fn cancel_reorder(&mut self, cx: &mut Context<Self>) {
        self.reorder.press = None;
        self.reorder.drag = None;
        self.reorder.task = None;
        cx.notify();
    }

    fn reorder_released(&mut self, cx: &mut Context<Self>) {
        if let Some(drag) = self.reorder.drag.take() {
            self.reorder.swallow_click = true;
            if let (Some(start), Some(&first)) = (drag.start, drag.rows.first()) {
                let rows = self.list_rows(first);
                if let Some(before) = plan_move(&rows, &drag.rows, start) {
                    self.apply_move(&drag.rows, before, cx);
                    self.show_order(&rows, &preview(&rows, &drag.rows, start));
                }
            }
        }
        self.cancel_reorder(cx);
    }

    /// Keeps the dropped order on screen until the engine or the library reports it.
    fn show_order(&mut self, rows: &[Selection], order: &[Selection]) {
        let sources: Vec<usize> = order
            .iter()
            .filter_map(|row| rows.iter().position(|r| r == row))
            .collect();
        match rows.first() {
            Some(Selection::Queue(_)) => {
                let entries = &mut std::sync::Arc::make_mut(&mut self.playback.queue).entries;
                *entries = sources.iter().map(|&i| entries[i].clone()).collect();
            }
            Some(&Selection::Playlist { list, .. }) => {
                if let Some(p) = self.playlists.lists.iter_mut().find(|p| p.id == list) {
                    p.entries = sources.iter().map(|&i| p.entries[i].clone()).collect();
                }
            }
            _ => {}
        }
    }

    fn apply_move(
        &mut self,
        moved: &[Selection],
        before: Option<Selection>,
        cx: &mut Context<Self>,
    ) {
        match moved.first() {
            Some(Selection::Queue(_)) => {
                let ids = moved
                    .iter()
                    .filter_map(|row| match row {
                        Selection::Queue(id) => Some(*id),
                        _ => None,
                    })
                    .collect();
                let before = match before {
                    Some(Selection::Queue(id)) => Some(id),
                    _ => None,
                };
                self.audio_command(Command::MoveQueuedBlock { ids, before }, cx);
            }
            Some(&Selection::Playlist { list, .. }) => {
                let entries = moved
                    .iter()
                    .filter_map(|row| match row {
                        Selection::Playlist { entry, .. } => Some(*entry),
                        _ => None,
                    })
                    .collect();
                let before = match before {
                    Some(Selection::Playlist { entry, .. }) => Some(entry),
                    _ => None,
                };
                self.change_playlist(
                    PlaylistChange::MoveBlock {
                        id: list,
                        entries,
                        before,
                    },
                    cx,
                );
            }
            _ => {}
        }
    }

    /// Wraps an editable list; the pointer is followed everywhere in the window
    /// while a press or a drag lasts.
    pub(crate) fn reorder_area(&self, list: AnyElement, cx: &mut Context<Self>) -> AnyElement {
        let active = self.reorder.press.is_some() || self.reorder.drag.is_some();
        let this = cx.entity();
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(list)
            .when(active, |area| {
                area.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            let moved = this.clone();
                            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                                if phase == DispatchPhase::Capture {
                                    moved.update(cx, |this, cx| {
                                        this.reorder_moved(event, bounds, cx)
                                    });
                                }
                            });
                            let released = this.clone();
                            window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                                if phase == DispatchPhase::Capture
                                    && event.button == MouseButton::Left
                                {
                                    released.update(cx, |this, cx| this.reorder_released(cx));
                                }
                            });
                        },
                    )
                    .absolute()
                    .size_full(),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
#[path = "tests/reorder.rs"]
mod tests;
