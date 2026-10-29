//! Playlist actions and small native text-entry dialogs.
use crate::{
    app::Soliton,
    i18n::Text,
    input::TextInput,
    notices::Level,
    scrollbar::Scrollbar,
    service::Work,
    widgets::{RADIUS_LARGE, TEXT_TITLE, accent_button, button},
};
use gpui::{prelude::*, *};
use soliton_core::{
    model::Track,
    playlist::{Playlist, PlaylistChange},
};

pub enum Dialog {
    New(Vec<Track>),
    Rename(i64),
    Add(Vec<Track>),
    Delete(i64),
}

pub struct Playlists {
    pub lists: Vec<Playlist>,
    pub selected: Option<i64>,
    pub name: Entity<TextInput>,
    pub dialog: Option<Dialog>,
    pub pending: bool,
    pub scroll: UniformListScrollHandle,
    pub bar: Scrollbar,
    pub picker_scroll: UniformListScrollHandle,
    pub picker_bar: Scrollbar,
}

impl Playlists {
    pub fn new(name: Entity<TextInput>) -> Self {
        let scroll = UniformListScrollHandle::new();
        let picker_scroll = UniformListScrollHandle::new();
        let bar = Scrollbar::new(scroll.0.borrow().base_handle.clone());
        let picker_bar = Scrollbar::new(picker_scroll.0.borrow().base_handle.clone());
        Self {
            lists: Vec::new(),
            selected: None,
            name,
            dialog: None,
            pending: false,
            bar,
            picker_bar,
            scroll,
            picker_scroll,
        }
    }
}

impl Soliton {
    pub fn open_playlist(&mut self, id: i64, cx: &mut Context<Self>) {
        self.record_place(cx);
        self.playlists.selected = Some(id);
        self.playlists.scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }
    pub fn change_playlist(&mut self, change: PlaylistChange, cx: &mut Context<Self>) -> bool {
        if self.playlists.pending {
            return false;
        }
        match self.services.work.try_send(Work::Playlist(change)) {
            Ok(()) => {
                self.playlists.pending = true;
                cx.notify();
                true
            }
            Err(_) => {
                let busy = soliton_core::failure::Failure::new(soliton_core::failure::Kind::Busy);
                self.report_failure(Level::Error, Text::Playlists, &busy);
                cx.notify();
                false
            }
        }
    }

    pub fn open_playlist_dialog(
        &mut self,
        dialog: Dialog,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = match &dialog {
            Dialog::Rename(id) => self
                .playlists
                .lists
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            _ => self.t(Text::NewPlaylistName).to_string(),
        };
        self.playlists.name.update(cx, |input, cx| {
            input.set_text(name);
            input.placeholder = self.t(Text::PlaylistName).into();
            cx.notify();
        });
        self.menu = None;
        self.track_menu = None;
        self.playlists.dialog = Some(dialog);
        if matches!(
            self.playlists.dialog,
            Some(Dialog::New(_) | Dialog::Rename(_))
        ) {
            self.playlists.name.read(cx).focus_handle(cx).focus(window);
        } else {
            self.focus.focus(window);
        }
        cx.notify();
    }

    pub fn submit_playlist_dialog(&mut self, cx: &mut Context<Self>) {
        let name = self.playlists.name.read(cx).content.trim().to_string();
        let change = match &self.playlists.dialog {
            Some(Dialog::New(tracks)) if valid_name(&name) => PlaylistChange::Create {
                name,
                tracks: tracks.clone(),
            },
            Some(Dialog::Rename(id)) if valid_name(&name) => {
                PlaylistChange::Rename { id: *id, name }
            }
            Some(Dialog::Delete(id)) => PlaylistChange::Delete(*id),
            _ => return,
        };
        if self.change_playlist(change, cx) {
            self.playlists.dialog = None;
        }
    }

    pub fn playlist_tracks(&self, playlist: &Playlist) -> Vec<Track> {
        let known = &self.tracks;
        playlist
            .entries
            .iter()
            .filter_map(|e| {
                e.track_id
                    .and_then(|id| known.get(&id))
                    .filter(|t| t.available)
                    .map(|t| (*t).clone())
            })
            .collect()
    }

    pub fn play_playlist(&mut self, id: i64, start: Option<i64>, cx: &mut Context<Self>) {
        let Some(list) = self.playlists.lists.iter().find(|p| p.id == id) else {
            return;
        };
        // Preserve the chosen occurrence when the same track is listed twice.
        let known = &self.tracks;
        let entries: Vec<_> = list
            .entries
            .iter()
            .filter_map(|e| {
                e.track_id
                    .and_then(|id| known.get(&id))
                    .filter(|t| t.available)
                    .map(|t| (e.id, (*t).clone()))
            })
            .collect();
        let at = match start {
            Some(id) => match entries.iter().position(|(e, _)| *e == id) {
                Some(at) => at,
                None => return,
            },
            None => 0,
        };
        if !entries.is_empty() {
            self.audio_command(
                soliton_audio::Command::PlayPlaylist { entries, start: at },
                cx,
            );
        }
    }

    pub fn playlist_dialog(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let dialog = self.playlists.dialog.as_ref()?;
        let theme = self.theme();
        let title = match dialog {
            Dialog::New(_) => Text::NewPlaylist,
            Dialog::Rename(_) => Text::RenamePlaylist,
            Dialog::Add(_) => Text::ChoosePlaylist,
            Dialog::Delete(_) => Text::DeletePlaylistQuestion,
        };
        let naming = matches!(dialog, Dialog::New(_) | Dialog::Rename(_));
        let valid = !naming || valid_name(self.playlists.name.read(cx).content.trim());
        let mut panel = div()
            .track_focus(&self.modal_focus)
            .w(rems(29.))
            .max_w_full()
            .p(rems(0.75))
            .rounded(RADIUS_LARGE)
            .bg(theme.panel)
            .border_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .gap(rems(0.75));
        panel = panel.child(div().text_size(TEXT_TITLE).child(self.t(title)));
        if naming {
            panel = panel
                .child(div().child(self.t(Text::PlaylistName)))
                .child(crate::widgets::input_field(
                    self.playlists.name.clone(),
                    false,
                    theme,
                ))
                .child(
                    div()
                        .text_size(crate::widgets::TEXT_SMALL)
                        .text_color(theme.muted)
                        .child(self.t(Text::PlaylistNameHint)),
                );
        }
        if let Dialog::Delete(id) = dialog {
            panel = panel
                .child(
                    self.playlists
                        .lists
                        .iter()
                        .find(|p| p.id == *id)
                        .map(|p| p.name.clone())
                        .unwrap_or_default(),
                )
                .child(
                    div()
                        .text_color(theme.muted)
                        .child(self.t(Text::DeletePlaylistHint)),
                );
        }
        if matches!(dialog, Dialog::Add(_)) {
            let list = uniform_list(
                "playlist-picker",
                self.playlists.lists.len(),
                cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                    range
                        .map(|i| {
                            let p = &this.playlists.lists[i];
                            let id = p.id;
                            button(
                                ("playlist-target", id as usize),
                                "list",
                                p.name.clone(),
                                this.theme(),
                            )
                            .w_full()
                            .justify_start()
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    let Some(Dialog::Add(tracks)) = &this.playlists.dialog else {
                                        return;
                                    };
                                    if this.change_playlist(
                                        PlaylistChange::Append {
                                            id,
                                            tracks: tracks.clone(),
                                        },
                                        cx,
                                    ) {
                                        this.playlists.dialog = None;
                                    }
                                },
                            ))
                        })
                        .collect()
                }),
            )
            .track_scroll(self.playlists.picker_scroll.clone())
            .h(rems((self.playlists.lists.len().clamp(1, 6) * 2) as f32))
            .min_h_0();
            panel = panel
                .child(
                    div()
                        .h(rems(12.))
                        .flex()
                        .flex_col()
                        .child(crate::views::scroll_area(
                            list,
                            &self.playlists.picker_bar,
                            theme,
                        )),
                )
                .child(
                    button(
                        "playlist-create-from-add",
                        "plus",
                        self.t(Text::NewPlaylist),
                        theme,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        let Some(Dialog::Add(tracks)) = this.playlists.dialog.take() else {
                            return;
                        };
                        this.open_playlist_dialog(Dialog::New(tracks), window, cx);
                    })),
                );
        }
        panel = panel.child(
            div()
                .flex()
                .justify_end()
                .gap(rems(0.5))
                .child(
                    button("playlist-cancel", "x", self.t(Text::Cancel), theme).on_click(
                        cx.listener(|this, _, window, cx| {
                            this.playlists.dialog = None;
                            this.focus.focus(window);
                            cx.notify();
                        }),
                    ),
                )
                .when(!matches!(dialog, Dialog::Add(_)), |row| {
                    row.child(
                        accent_button(
                            "playlist-submit",
                            "check",
                            self.t(if matches!(dialog, Dialog::Delete(_)) {
                                Text::DeletePlaylist
                            } else {
                                Text::SavePlaylist
                            }),
                            theme,
                        )
                        .opacity(if valid && !self.playlists.pending {
                            1.
                        } else {
                            0.45
                        })
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.submit_playlist_dialog(cx);
                            if this.playlists.dialog.is_none() {
                                this.focus.focus(window);
                            }
                        })),
                    )
                }),
        );
        Some(
            crate::window_chrome::round(
                div(),
                crate::window_chrome::inner_corners(window),
                Corners::all(true),
            )
            .absolute()
            .inset_0()
            .occlude()
            .p(rems(0.75))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.scrim)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(panel)
            .into_any_element(),
        )
    }

    pub fn playlist_tools(
        &self,
        tracks: Vec<Track>,
        id: impl Into<ElementId>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Entry of the album actions menu, aligned with the others.
        let trigger =
            button(id, "list-plus", self.t(Text::AddToPlaylist), self.theme()).justify_start();
        trigger.on_click(cx.listener(move |this, _, window, cx| {
            cx.stop_propagation();
            if !tracks.is_empty() {
                this.open_playlist_dialog(Dialog::Add(tracks.clone()), window, cx);
            }
        }))
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.chars().count() <= 200
}
