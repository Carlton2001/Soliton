use super::{DEPTH, Filter, History, Place, Section};

fn place(section: Section, album: Option<i64>) -> Place {
    Place {
        home_artist: None,
        playlist: None,
        section,
        album: album.map(|id| (id, 0)),
        filter: Filter::default(),
    }
}

#[test]
fn walks_back_and_forward_and_drops_forward_on_new_place() {
    let mut history = History::default();
    let artists = place(Section::Artists, None);
    let albums = place(Section::Albums, None);
    let sheet = place(Section::Albums, Some(7));
    history.observe(artists.clone());
    history.observe(albums.clone());
    history.observe(albums.clone());
    history.observe(sheet.clone());
    assert!(history.back() == Some(albums.clone()));
    assert!(history.back() == Some(artists.clone()));
    assert!(history.back().is_none());
    // Restoring a place is observed again without changing the history.
    history.observe(artists.clone());
    assert!(history.forward() == Some(albums.clone()));
    let settings = place(Section::Settings, None);
    history.observe(settings.clone());
    assert!(history.forward().is_none());
    assert!(history.back() == Some(albums));
}

#[test]
fn keeps_a_bounded_depth() {
    let mut history = History::default();
    for id in 0..(DEPTH as i64 + 10) {
        history.observe(place(Section::Albums, Some(id)));
    }
    let mut steps = 0;
    while history.back().is_some() {
        steps += 1;
    }
    assert_eq!(steps, DEPTH);
}

#[test]
fn queue_is_a_named_history_destination() {
    let mut history = History::default();
    let albums = place(Section::Albums, None);
    let queue = place(Section::Queue, None);
    history.observe(albums.clone());
    history.observe(queue.clone());
    history.observe(place(Section::Playlists, None));
    assert!(history.back() == Some(queue));
    assert!(history.back() == Some(albums));
    assert!(Section::from(rap_core::config::View::Queue) == Section::Queue);
    assert_eq!(
        rap_core::config::View::from(Section::Queue),
        rap_core::config::View::Queue
    );
}

#[test]
fn home_keeps_its_artist_and_album_in_navigation_history() {
    let mut history = History::default();
    let mut home = place(Section::Home, Some(42));
    home.home_artist = Some("Björk".into());
    history.observe(home.clone());
    history.observe(place(Section::Artists, None));
    assert!(history.back() == Some(home));
    assert!(Section::from(rap_core::config::View::Home) == Section::Home);
    assert_eq!(
        rap_core::config::View::from(Section::Home),
        rap_core::config::View::Home
    );
}
