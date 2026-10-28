use super::super::{Connection, check};
use crate::output::Output;
use pipewire as pw;
use soliton_core::failure::{Failure, Kind};
use std::{
    sync::{atomic::Ordering, mpsc},
    thread::JoinHandle,
    time::{Duration, Instant},
};

/// A sink owned by a test client: it disappears with the client's connection.
struct NullSink {
    stop: pw::channel::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl NullSink {
    fn create(name: &str) -> Self {
        let (stop, stopped) = pw::channel::channel();
        let (ready, created) = mpsc::channel();
        let name = name.to_owned();
        let thread = std::thread::spawn(move || {
            let connection = Connection::new(|_| {}).unwrap();
            let _node: pw::node::Node = connection
                .core
                .create_object(
                    "adapter",
                    &pw::properties::properties! {
                        "factory.name" => "support.null-audio-sink",
                        "node.name" => name.as_str(),
                        "node.description" => "Soliton test sink",
                        "media.class" => "Audio/Sink",
                        "audio.position" => "FL,FR",
                        "object.linger" => "false",
                    },
                )
                .unwrap();
            connection.roundtrip().unwrap();
            let _stop = stopped.attach(connection.mainloop.loop_(), {
                let mainloop = connection.mainloop.clone();
                move |()| mainloop.quit()
            });
            ready.send(()).unwrap();
            connection.mainloop.run();
        });
        created.recv_timeout(Duration::from_secs(5)).unwrap();
        Self {
            stop,
            thread: Some(thread),
        }
    }

    fn remove(mut self) {
        let _ = self.stop.send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn wait_for(what: &str, condition: impl Fn() -> bool) {
    let start = Instant::now();
    while !condition() {
        assert!(start.elapsed() < Duration::from_secs(5), "{what} timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[ignore = "Requires a PipeWire session; creates a temporary silent sink"]
fn chosen_sink_is_played_and_its_removal_fails_without_fallback() {
    let name = format!("soliton-test-sink-{}", std::process::id());
    let sink = NullSink::create(&name);
    wait_for("sink announcement", || check(&name).is_ok());
    let output = Output::open(48000, 2, 0., Some(&name)).unwrap();
    assert_eq!(output.info().device_label, "Soliton test sink");
    wait_for("link to the chosen sink", || {
        output.info().device_name == name && output.info().device_label == "Soliton test sink"
    });
    assert!(!output.failed.load(Ordering::Relaxed));
    sink.remove();
    wait_for("output failure", || output.failed.load(Ordering::Relaxed));
    // The stream must not be moved to another sink.
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(output.info().device_name, name);
    let missing = check(&name).unwrap_err();
    assert_eq!(
        missing.downcast_ref::<Failure>(),
        Some(&Failure::with(Kind::OutputUnavailable, name.as_str()))
    );
}

#[test]
#[ignore = "Requires a PipeWire session with another sink; creates a temporary silent sink"]
fn engine_pauses_on_sink_loss_resumes_on_the_default_and_returns_to_the_sink() {
    use crate::{Command, Engine, Playback, PlaybackState};
    let name = format!("soliton-test-return-{}", std::process::id());
    let sink = NullSink::create(&name);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    crate::test_support::wav(&path, 44100, 20);
    let engine = Engine::start(0., Some(name.clone())).unwrap();
    let wait = |what: &str, predicate: &dyn Fn(&Playback) -> bool| {
        let start = Instant::now();
        loop {
            assert!(start.elapsed() < Duration::from_secs(8), "{what} timed out");
            let value = engine.events.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_ne!(value.state, PlaybackState::Failed, "{:?}", value.error);
            if predicate(&value) {
                return value;
            }
        }
    };
    let on = |p: &Playback, sink: &str| p.output.as_ref().is_some_and(|o| o.device_name == sink);
    wait("listed sink", &|p| p.outputs.iter().any(|o| o.name == name));
    engine
        .send(Command::Play {
            tracks: vec![soliton_core::model::Track {
                id: 1,
                source_id: 1,
                path,
                metadata: Default::default(),
                cover: None,
                available: true,
            }],
            start: 0,
        })
        .unwrap();
    wait("playback on the chosen sink", &|p| {
        p.state == PlaybackState::Playing && p.position_ms > 200 && on(p, &name)
    });
    sink.remove();
    let paused = wait("pause on sink loss", &|p| p.state == PlaybackState::Paused);
    assert!(paused.fallback);
    assert_eq!(
        paused.interruption.as_ref().map(|f| f.kind),
        Some(Kind::OutputDisconnected)
    );
    assert_eq!(paused.selected_output.as_deref(), Some(name.as_str()));
    engine.send(Command::Resume).unwrap();
    let fallback = wait("playback on the default", &|p| {
        p.state == PlaybackState::Playing
            && p.output.as_ref().is_some_and(|o| !o.device_name.is_empty())
    });
    assert!(!on(&fallback, &name));
    assert!(fallback.interruption.is_none());
    let sink = NullSink::create(&name);
    let returned = wait("return to the chosen sink", &|p| {
        !p.fallback && on(p, &name)
    });
    assert_eq!(returned.state, PlaybackState::Playing);
    assert!(returned.position_ms >= fallback.position_ms);
    engine.send(Command::Stop).unwrap();
    wait("stop", &|p| p.state == PlaybackState::Stopped);
    sink.remove();
}

const DEFAULT_KEY: &str = "default.configured.audio.sink";

/// The configured default sink, restored on drop even when the test fails:
/// WirePlumber persists it as the user's choice.
struct ConfiguredDefault(Option<String>);

impl ConfiguredDefault {
    fn replace(name: &str) -> Self {
        let output = std::process::Command::new("pw-metadata")
            .args(["0", DEFAULT_KEY])
            .output()
            .expect("pw-metadata from pipewire-utils");
        let listing = String::from_utf8_lossy(&output.stdout);
        let original = listing.lines().find_map(|line| {
            let value = line.split_once("value:'")?.1;
            Some(value.rsplit_once("' type:")?.0.to_owned())
        });
        let saved = Self(original);
        set_default(Some(&format!(r#"{{ "name": "{name}" }}"#)));
        saved
    }
}

impl Drop for ConfiguredDefault {
    fn drop(&mut self) {
        set_default(self.0.as_deref());
    }
}

fn set_default(value: Option<&str>) {
    let mut command = std::process::Command::new("pw-metadata");
    match value {
        Some(value) => command.args(["0", DEFAULT_KEY, value, "Spa:String:JSON"]),
        None => command.args(["-d", "0", DEFAULT_KEY]),
    };
    assert!(command.output().unwrap().status.success());
}

#[test]
#[ignore = "Requires a PipeWire session; briefly makes a temporary silent sink the system default"]
fn default_following_playback_moves_with_the_system_default() {
    use crate::{Command, Engine, Playback, PlaybackState};
    let name = format!("soliton-test-default-{}", std::process::id());
    let sink = NullSink::create(&name);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    crate::test_support::wav(&path, 44100, 20);
    let engine = Engine::start(0., None).unwrap();
    let wait = |what: &str, predicate: &dyn Fn(&Playback) -> bool| {
        let start = Instant::now();
        loop {
            assert!(start.elapsed() < Duration::from_secs(8), "{what} timed out");
            let value = engine.events.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_ne!(value.state, PlaybackState::Failed, "{:?}", value.error);
            if predicate(&value) {
                return value;
            }
        }
    };
    let on = |p: &Playback, sink: &str| p.output.as_ref().is_some_and(|o| o.device_name == sink);
    wait("listed sink", &|p| p.outputs.iter().any(|o| o.name == name));
    engine
        .send(Command::Play {
            tracks: vec![soliton_core::model::Track {
                id: 1,
                source_id: 1,
                path,
                metadata: Default::default(),
                cover: None,
                available: true,
            }],
            start: 0,
        })
        .unwrap();
    let before = wait("playback on the default", &|p| {
        p.state == PlaybackState::Playing
            && p.position_ms > 200
            && p.output.as_ref().is_some_and(|o| !o.device_name.is_empty())
    });
    let original = before.output.unwrap().device_name;
    assert_ne!(original, name);
    let configured = ConfiguredDefault::replace(&name);
    let moved = wait("move to the new default", &|p| on(p, &name));
    assert_eq!(moved.state, PlaybackState::Playing);
    assert!(!moved.fallback && moved.interruption.is_none());
    assert_eq!(moved.selected_output, None);
    drop(configured);
    let back = wait("return to the restored default", &|p| on(p, &original));
    assert_eq!(back.state, PlaybackState::Playing);
    assert!(back.position_ms > moved.position_ms);
    engine.send(Command::Stop).unwrap();
    wait("stop", &|p| p.state == PlaybackState::Stopped);
    sink.remove();
}
