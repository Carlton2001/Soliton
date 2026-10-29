use super::*;
use crate::test_renderer::{Mock, track};

#[tokio::test]
async fn upnp_session_controls_media_and_serves_only_the_current_track() {
    let mock = Mock::start().await;
    let renderer = mock.renderer.clone();
    let dir = tempfile::tempdir().unwrap();
    let track = track(dir.path());
    let path = track.path.clone();
    let mut controller = Controller::default();
    controller
        .command(Command::Connect(mock.device(true)))
        .await
        .unwrap();
    let status = controller
        .command(Command::Load {
            track: track.clone(),
            position_ms: 2000,
            autoplay: false,
            volume: 0.,
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.state, State::Paused);
    let first_generation = status.generation;
    let uri = renderer.lock().await.uri.clone();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    assert_eq!(
        client
            .get(&uri)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
        "synthetic WAV bytes"
    );
    assert_eq!(
        controller
            .command(Command::Play)
            .await
            .unwrap()
            .unwrap()
            .state,
        State::Playing
    );
    controller.command(Command::Seek(5000)).await.unwrap();
    controller.poll().await.unwrap();
    assert_eq!(controller.status.as_ref().unwrap().position_ms, 5000);
    assert_eq!(
        controller
            .command(Command::Pause)
            .await
            .unwrap()
            .unwrap()
            .state,
        State::Paused
    );
    controller.command(Command::Volume(0.25)).await.unwrap();
    assert_eq!(renderer.lock().await.volume, 25);
    controller.command(Command::Play).await.unwrap();
    renderer.lock().await.position = "0:00:09".into();
    controller.poll().await.unwrap();
    renderer.lock().await.state = "STOPPED".into();
    controller.poll().await.unwrap();
    assert_eq!(controller.status.as_ref().unwrap().state, State::Finished);
    assert_eq!(
        client.get(&uri).send().await.unwrap().status(),
        reqwest::StatusCode::NOT_FOUND
    );
    let next = controller
        .command(Command::Load {
            track,
            position_ms: 0,
            autoplay: true,
            volume: 0.,
        })
        .await
        .unwrap()
        .unwrap();
    assert!(next.generation > first_generation);
    controller.command(Command::Stop).await.unwrap();
    assert_eq!(controller.status.as_ref().unwrap().state, State::Stopped);
    controller.disconnect().await;
    assert_eq!(std::fs::read(path).unwrap(), b"synthetic WAV bytes");
}

#[tokio::test]
async fn commands_need_a_selected_and_connected_output() {
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::default();
    for command in [
        Command::Play,
        Command::Pause,
        Command::Seek(0),
        Command::Stop,
    ] {
        let error = controller.command(command).await.unwrap_err();
        assert!(error.to_string().contains("disconnected"), "{error}");
    }
    let load = controller
        .command(Command::Load {
            track: track(dir.path()),
            position_ms: 0,
            autoplay: true,
            volume: 0.,
        })
        .await
        .unwrap_err();
    assert!(load.to_string().contains("No network output selected"));
    // Polling without a loaded track asks nothing of anyone.
    controller.poll().await.unwrap();
    assert!(controller.status.is_none());
}

#[tokio::test]
async fn selecting_the_connected_output_again_keeps_its_session() {
    let mock = Mock::start().await;
    let mut controller = Controller::default();
    controller
        .command(Command::Connect(mock.device(true)))
        .await
        .unwrap();
    controller
        .command(Command::Connect(mock.device(true)))
        .await
        .unwrap();
    // Connecting checks the transport once; the second selection reused it.
    assert_eq!(mock.seen("GetTransportInfo").await, 1);
    controller.command(Command::Disconnect).await.unwrap();
    let error = controller.command(Command::Play).await.unwrap_err();
    assert!(error.to_string().contains("disconnected"));
}

#[tokio::test]
async fn a_failure_revokes_the_media_and_the_next_load_reconnects() {
    let mock = Mock::start().await;
    let dir = tempfile::tempdir().unwrap();
    let track = track(dir.path());
    let mut controller = Controller::default();
    controller
        .command(Command::Connect(mock.device(true)))
        .await
        .unwrap();
    let loaded = controller
        .command(Command::Load {
            track: track.clone(),
            position_ms: 0,
            autoplay: true,
            volume: 0.5,
        })
        .await
        .unwrap()
        .unwrap();
    let uri = mock.renderer.lock().await.uri.clone();
    controller.fail(anyhow::anyhow!("receiver lost")).await;
    let failed = controller.status.clone().unwrap();
    assert_eq!(failed.state, State::Failed);
    assert_eq!(failed.generation, loaded.generation);
    assert!(failed.error.unwrap().contains("receiver lost"));
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    assert_eq!(
        client.get(&uri).send().await.unwrap().status(),
        reqwest::StatusCode::NOT_FOUND
    );
    // A failed output is never polled, and reconnects only to load again.
    controller.poll().await.unwrap();
    assert_eq!(controller.status.as_ref().unwrap().state, State::Failed);
    let connections = mock.seen("GetTransportInfo").await;
    let reloaded = controller
        .command(Command::Load {
            track,
            position_ms: 0,
            autoplay: true,
            volume: 0.5,
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reloaded.state, State::Playing);
    assert!(reloaded.generation > loaded.generation);
    assert_eq!(mock.seen("GetTransportInfo").await, connections + 1);
}

#[tokio::test]
async fn statuses_without_a_duration_keep_the_known_one() {
    let mock = Mock::start().await;
    let dir = tempfile::tempdir().unwrap();
    let mut controller = Controller::default();
    controller
        .command(Command::Connect(mock.device(true)))
        .await
        .unwrap();
    controller
        .command(Command::Load {
            track: track(dir.path()),
            position_ms: 0,
            autoplay: true,
            volume: 0.,
        })
        .await
        .unwrap();
    {
        let mut renderer = mock.renderer.lock().await;
        renderer.duration = "0:00:00".into();
        renderer.position = "0:00:03".into();
    }
    controller.poll().await.unwrap();
    let status = controller.status.clone().unwrap();
    assert_eq!(status.position_ms, 3000);
    assert_eq!(status.duration_ms, 10_000);
}
