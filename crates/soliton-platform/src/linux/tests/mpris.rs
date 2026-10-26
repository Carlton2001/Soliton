use super::*;
use crate::MediaTrack;
use std::{path::PathBuf, time::Duration};

fn playing(position_ms: u64) -> MediaState {
    MediaState {
        track: Some(MediaTrack {
            title: "Été".into(),
            artist: "Artiste".into(),
            album: "Album".into(),
            cover: Some(PathBuf::from(
                "/home/moi/.local/share/Soliton/covers/a b.png",
            )),
        }),
        state: TransportState::Playing,
        position_ms,
        duration_ms: 200_000,
        ..Default::default()
    }
}

#[test]
fn position_ticks_are_not_published_but_jumps_are_seeks() {
    let previous = playing(10_000);
    assert_eq!(
        Changes::between(&previous, &playing(10_060), 60),
        Changes::default()
    );
    let seek = Changes::between(&previous, &playing(90_000), 60);
    assert!(seek.seeked && !seek.track && !seek.metadata);
    let mut paused = playing(10_000);
    paused.state = TransportState::Paused;
    // Paused time does not advance; a two-second wait is not a seek.
    let resumed = Changes::between(&paused, &playing(10_000), 2_000);
    assert!(resumed.status && !resumed.seeked);
}

#[test]
fn track_changes_refresh_metadata_without_a_seek() {
    let mut next = playing(0);
    next.track.as_mut().unwrap().title = "Suivante".into();
    let changes = Changes::between(&playing(150_000), &next, 60);
    assert!(changes.track && changes.metadata && !changes.seeked && !changes.capabilities);
    let stopped = Changes::between(&playing(0), &MediaState::default(), 60);
    assert!(stopped.capabilities && stopped.status && !stopped.seeked);
}

#[test]
fn relative_seeks_clamp_at_zero_and_go_next_past_the_end() {
    assert_eq!(
        seek_command(&playing(10_000), -5_000_000),
        Some(MediaCommand::Seek(5_000))
    );
    assert_eq!(
        seek_command(&playing(1_000), -5_000_000),
        Some(MediaCommand::Seek(0))
    );
    assert_eq!(
        seek_command(&playing(199_000), 5_000_000),
        Some(MediaCommand::Next)
    );
    assert_eq!(seek_command(&MediaState::default(), 1_000_000), None);
}

#[test]
fn artwork_uris_are_percent_encoded() {
    assert_eq!(
        file_uri(Path::new("/home/moi/covers/é a%b.png")),
        "file:///home/moi/covers/%C3%A9%20a%25b.png"
    );
}

#[test]
#[ignore = "Requires a D-Bus session bus"]
fn session_bus_clients_read_metadata_and_send_commands() {
    let (events, received) = crossbeam_channel::bounded(8);
    let mut mpris = Mpris::start(events).unwrap();
    mpris.update(playing(42_000)).unwrap();
    let client = Connection::session().unwrap();
    let service = mpris.connection.unique_name().unwrap().to_owned();
    let player = zbus::blocking::Proxy::new(
        &client,
        service.as_str(),
        PATH,
        "org.mpris.MediaPlayer2.Player",
    )
    .unwrap();
    let status: String = player.get_property("PlaybackStatus").unwrap();
    assert_eq!(status, "Playing");
    let position: i64 = player.get_property("Position").unwrap();
    assert_eq!(position, 42_000_000);
    let metadata: HashMap<String, OwnedValue> = player.get_property("Metadata").unwrap();
    let title: String = metadata["xesam:title"]
        .try_clone()
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(title, "Été");
    let track: ObjectPath<'_> = metadata["mpris:trackid"].downcast_ref().unwrap();
    let () = player.call("PlayPause", &()).unwrap();
    let () = player
        .call("SetPosition", &(track.to_owned(), 5_000_000i64))
        .unwrap();
    let commands: Vec<_> = (0..2)
        .map(|_| received.recv_timeout(Duration::from_secs(2)).unwrap())
        .collect();
    assert!(matches!(
        commands[0],
        DesktopEvent::Media(MediaCommand::Toggle)
    ));
    assert!(matches!(
        commands[1],
        DesktopEvent::Media(MediaCommand::Seek(5_000))
    ));
}
