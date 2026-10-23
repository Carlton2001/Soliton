//! Three independent, virtualised panes: artists, their albums, and album tracks.
mod layout;
use crate::{
    app::Rap,
    catalog::Facets,
    i18n::Text,
    scrollbar::Scrollbar,
    views::scroll_area,
    widgets::{RADIUS, TEXT_SMALL, TEXT_TITLE, cover, quality},
};
use gpui::{prelude::*, *};
pub(super) use layout::HomeMetrics;
use rap_core::{model::LibrarySnapshot, text::fold};
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

pub struct ArtistAlbums {
    pub name: String,
    pub albums: Vec<usize>,
    album_width: f32,
}

/// Prepared by the query worker, never by a render or a pointer callback.
#[derive(Default)]
pub struct HomeCatalog {
    pub artists: Vec<ArtistAlbums>,
    pub metrics: Arc<HomeMetrics>,
}

impl HomeCatalog {
    pub fn collect(snapshot: &LibrarySnapshot, facets: &Facets, visible: &[usize]) -> Self {
        let mut by_artist: HashMap<String, Vec<usize>> = HashMap::new();
        for &index in visible {
            let album = &snapshot.albums[index];
            let mut names: BTreeSet<_> = album
                .tracks()
                .map(|track| fold(&track.metadata.artist))
                .collect();
            names.insert(fold(&album.artist));
            for name in names.into_iter().filter(|name| !name.is_empty()) {
                by_artist.entry(name).or_default().push(index);
            }
        }
        Self {
            metrics: Arc::default(),
            artists: facets
                .artists
                .iter()
                .filter_map(|name| {
                    Some(ArtistAlbums {
                        album_width: 128.,
                        name: name.clone(),
                        albums: by_artist.remove(&fold(name))?,
                    })
                })
                .collect(),
        }
    }

    /// Cache each visible artist's width on the query worker. Selecting an
    /// artist only reads one number on the UI thread, without reshaping text.
    pub fn set_metrics(&mut self, snapshot: &LibrarySnapshot, metrics: Arc<HomeMetrics>) {
        for artist in &mut self.artists {
            artist.album_width = artist
                .albums
                .iter()
                .map(|&index| metrics.album_width(snapshot.albums[index].id))
                .fold(128., f32::max);
        }
        self.metrics = metrics;
    }
}

pub struct Home {
    pub catalog: Arc<HomeCatalog>,
    pub artist: Option<String>,
    artist_scroll: UniformListScrollHandle,
    artist_bar: Scrollbar,
    album_scroll: UniformListScrollHandle,
    album_bar: Scrollbar,
}

impl Home {
    pub fn new(artist: Option<String>) -> Self {
        let artist_scroll = UniformListScrollHandle::new();
        let album_scroll = UniformListScrollHandle::new();
        let artist_bar = Scrollbar::new(artist_scroll.0.borrow().base_handle.clone());
        let album_bar = Scrollbar::new(album_scroll.0.borrow().base_handle.clone());
        Self {
            catalog: Arc::default(),
            artist,
            artist_bar,
            album_bar,
            artist_scroll,
            album_scroll,
        }
    }

    fn selected_artist(&self) -> Option<&ArtistAlbums> {
        self.catalog
            .artists
            .iter()
            .find(|a| Some(&a.name) == self.artist.as_ref())
    }

    fn album_column_width(&self) -> f32 {
        self.selected_artist()
            .map_or(128., |artist| artist.album_width)
    }

    /// Keep IDs through reordering; replace selections removed by filtering or a refresh.
    fn reconcile(&mut self, snapshot: &LibrarySnapshot, selected: &mut Option<i64>) -> bool {
        if self.selected_artist().is_none() {
            self.artist = self.catalog.artists.first().map(|a| a.name.clone());
            self.album_scroll.scroll_to_item(0, ScrollStrategy::Top);
        }
        let albums = self
            .selected_artist()
            .map(|a| a.albums.as_slice())
            .unwrap_or_default();
        let previous = *selected;
        if !albums
            .iter()
            .any(|&i| Some(snapshot.albums[i].id) == *selected)
        {
            *selected = albums.first().map(|&i| snapshot.albums[i].id);
        }
        previous != *selected
    }
}

impl Rap {
    pub fn reconcile_home(&mut self) {
        if self.home.reconcile(&self.snapshot, &mut self.selected) {
            self.selected_edition = 0;
            self.track_menu = None;
            self.track_scroll.scroll_to_item(0, ScrollStrategy::Top);
        }
    }

    fn select_home_artist(&mut self, name: String, cx: &mut Context<Self>) {
        self.home.artist = Some(name);
        self.selected = None;
        self.selected_edition = 0;
        self.track_menu = None;
        self.home
            .album_scroll
            .scroll_to_item(0, ScrollStrategy::Top);
        self.reconcile_home();
        cx.notify();
    }

    fn select_home_album(&mut self, id: i64, cx: &mut Context<Self>) {
        if self.selected != Some(id) {
            self.selected = Some(id);
            self.selected_edition = 0;
            self.track_menu = None;
            self.track_scroll.scroll_to_item(0, ScrollStrategy::Top);
            cx.notify();
        }
    }

    pub fn home_view(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("home-panes")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .overflow_hidden()
            .child(self.home_artists(cx))
            .child(self.home_albums(cx))
            .child(self.home_tracks(cx))
            .into_any_element()
    }

    fn home_heading(
        &self,
        title: impl Into<SharedString>,
        subtitle: impl Into<SharedString>,
    ) -> Div {
        div()
            .p(rems(0.75))
            .flex_none()
            .flex()
            .flex_col()
            .gap(rems(0.25))
            .child(div().text_size(TEXT_TITLE).child(title.into()))
            .child(
                div()
                    .text_size(TEXT_SMALL)
                    .text_color(self.theme().muted)
                    .child(subtitle.into()),
            )
    }

    fn home_artists(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let catalog = self.home.catalog.clone();
        let list = uniform_list(
            "home-artists",
            catalog.artists.len(),
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                range
                    .map(|index| {
                        let artist = &catalog.artists[index];
                        let name = artist.name.clone();
                        let selected = this.home.artist.as_ref() == Some(&name);
                        div()
                            .id(("home-artist", index))
                            .w_full()
                            .h(rems(2.5))
                            .px(rems(0.75))
                            .flex()
                            .items_center()
                            .rounded(RADIUS)
                            .when(selected, |v| v.bg(theme.selected).text_color(theme.accent))
                            .hover(move |s| {
                                s.bg(if selected {
                                    theme.selected
                                } else {
                                    theme.hover
                                })
                            })
                            .child(div().min_w_0().flex_1().truncate().child(name.clone()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.select_home_artist(name.clone(), cx)
                            }))
                    })
                    .collect()
            }),
        )
        .track_scroll(self.home.artist_scroll.clone())
        .px(rems(0.75))
        .w_full()
        .flex_1()
        .min_h_0();
        div()
            .w(rems(self.home.catalog.metrics.artists_width() / 16.))
            .flex_none()
            .min_h_0()
            .border_r_1()
            .border_color(theme.border)
            .flex()
            .flex_col()
            .bg(theme.panel)
            .child(self.home_heading(
                self.t(Text::Artists),
                self.home.catalog.artists.len().to_string(),
            ))
            .when(self.home.catalog.artists.is_empty(), |v| {
                v.child(
                    div()
                        .p(rems(0.75))
                        .text_color(theme.muted)
                        .child(self.t(Text::NoResults)),
                )
            })
            .child(scroll_area(list, &self.home.artist_bar, theme))
            .into_any_element()
    }

    fn home_albums(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let count = self.home.selected_artist().map_or(0, |a| a.albums.len());
        let list = uniform_list(
            "home-albums",
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                let Some(artist) = this.home.selected_artist() else {
                    return Vec::new();
                };
                range
                    .filter_map(|i| artist.albums.get(i))
                    .map(|&index| {
                        let album = &this.snapshot.albums[index];
                        let id = album.id;
                        let selected = this.selected == Some(id);
                        let first = &album.primary().tracks[0].metadata;
                        let year = first.year.map(|year| year.to_string());
                        let details = [year.clone(), Some(quality(first))]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(" · ");
                        let details = StyledText::new(details).with_highlights(
                            year.into_iter().map(|year| {
                                (
                                    0..year.len(),
                                    HighlightStyle {
                                        font_weight: Some(FontWeight::SEMIBOLD),
                                        ..Default::default()
                                    },
                                )
                            }),
                        );
                        div()
                            .id(("home-album", id as usize))
                            .w_full()
                            .h(rems(6.))
                            .p(rems(0.5))
                            .flex()
                            .items_center()
                            .gap(rems(0.75))
                            .rounded(RADIUS)
                            .when(selected, |v| v.bg(theme.selected))
                            .hover(move |s| {
                                s.bg(if selected {
                                    theme.selected
                                } else {
                                    theme.hover
                                })
                            })
                            .child(cover(album.cover(), 72., theme))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(rems(0.25))
                                    .child(
                                        div()
                                            .truncate()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(if selected {
                                                theme.accent
                                            } else {
                                                theme.text
                                            })
                                            .child(album.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(TEXT_SMALL)
                                            .text_color(theme.muted)
                                            .child(this.album_artist(album)),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(TEXT_SMALL)
                                            .text_color(theme.muted)
                                            .child(details),
                                    ),
                            )
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.select_home_album(id, cx)),
                            )
                    })
                    .collect()
            }),
        )
        .track_scroll(self.home.album_scroll.clone())
        .px(rems(0.75))
        .w_full()
        .flex_1()
        .min_h_0();
        div()
            .w(rems(self.home.album_column_width() / 16.))
            .flex_none()
            .min_h_0()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(theme.border)
            .child(self.home_heading(self.t(Text::Albums), crate::i18n::album_count(count)))
            .when(count == 0, |v| {
                v.child(
                    div()
                        .p(rems(0.75))
                        .text_color(theme.muted)
                        .child(self.t(Text::ChooseArtist)),
                )
            })
            .child(scroll_area(list, &self.home.album_bar, theme))
            .into_any_element()
    }

    fn home_tracks(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let pane = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(theme.panel)
            .gap(rems(0.75));
        let Some(album) = self
            .snapshot
            .albums
            .iter()
            .find(|a| Some(a.id) == self.selected)
        else {
            return pane
                .child(self.home_heading(self.t(Text::Tracks), self.t(Text::ChooseAlbum)))
                .into_any_element();
        };
        let index = if self.selected_edition < album.editions.len() {
            self.selected_edition
        } else {
            0
        };
        let edition = &album.editions[index];
        let summary = div()
            .p(rems(0.75))
            .flex_none()
            .flex()
            .gap(rems(1.))
            .child(cover(edition.cover().or(album.cover()), 72., theme))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(rems(0.25))
                    .child(
                        div()
                            .truncate()
                            .text_size(TEXT_TITLE)
                            .child(album.title.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_color(theme.muted)
                            .child(self.album_artist(album)),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(TEXT_SMALL)
                            .text_color(theme.muted)
                            .child(format!(
                                "{} {} · {}",
                                edition.tracks.len(),
                                self.t(Text::Tracks),
                                quality(&edition.tracks[0].metadata)
                            )),
                    ),
            );
        pane.child(summary)
            .child(
                self.album_actions(album, index, false, cx)
                    .px(rems(0.75))
                    .pt_0()
                    .flex_none(),
            )
            .when(album.editions.len() > 1, |v| {
                v.child(
                    self.editions_view(album, index, cx)
                        .px(rems(0.75))
                        .flex_none(),
                )
            })
            .child(self.tracks_view(album.id, index, cx))
            .into_any_element()
    }
}

#[cfg(test)]
#[path = "tests/home.rs"]
mod tests;
