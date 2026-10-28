use super::{Global, Graph};
use soliton_core::failure::{Failure, Kind};

fn add(graph: &mut Graph, id: u32, kind: Global, props: &[(&str, &str)]) -> bool {
    graph.add(id, kind, |key| {
        props
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.to_string())
    })
}

fn sink(graph: &mut Graph, id: u32, name: &str, description: Option<&str>, device: &str) -> bool {
    let mut props = vec![
        ("media.class", "Audio/Sink"),
        ("node.name", name),
        ("device.id", device),
    ];
    if let Some(description) = description {
        props.push(("node.description", description));
    }
    add(graph, id, Global::Node, &props)
}

#[test]
fn sinks_are_listed_with_their_description_and_bluetooth_device() {
    let mut graph = Graph::default();
    assert!(!add(
        &mut graph,
        10,
        Global::Device,
        &[("device.api", "alsa")]
    ));
    assert!(!add(
        &mut graph,
        11,
        Global::Device,
        &[("device.api", "bluez5")]
    ));
    assert!(sink(
        &mut graph,
        20,
        "alsa_output.analog",
        Some("Internal Audio"),
        "10"
    ));
    assert!(sink(
        &mut graph,
        21,
        "bluez_output.headset",
        Some("Headphones"),
        "11"
    ));
    assert!(sink(&mut graph, 22, "virtual", None, "none"));
    // Playback streams and sources are not outputs.
    assert!(!add(
        &mut graph,
        30,
        Global::Node,
        &[
            ("media.class", "Stream/Output/Audio"),
            ("node.name", "Firefox")
        ]
    ));
    let outputs = graph.outputs();
    let summary: Vec<_> = outputs
        .iter()
        .map(|o| (o.name.as_str(), o.label.as_str(), o.bluetooth, o.selectable))
        .collect();
    assert_eq!(
        summary,
        [
            ("bluez_output.headset", "Headphones", true, true),
            ("alsa_output.analog", "Internal Audio", false, true),
            ("virtual", "virtual", false, true),
        ]
    );
}

#[test]
fn removed_sinks_disappear_and_names_resolve_only_when_unique() {
    let mut graph = Graph::default();
    sink(
        &mut graph,
        20,
        "alsa_output.analog",
        Some("Internal Audio"),
        "10",
    );
    sink(
        &mut graph,
        21,
        "bluez_output.headset",
        Some("Headphones"),
        "11",
    );
    assert_eq!(graph.find("bluez_output.headset").unwrap(), 21);
    assert!(!graph.remove(99));
    assert!(graph.remove(21));
    assert_eq!(graph.sink(21), None);
    assert!(graph.has_sinks());
    assert_eq!(graph.outputs().len(), 1);
    let missing = graph.find("bluez_output.headset").unwrap_err();
    assert_eq!(
        missing.downcast_ref::<Failure>(),
        Some(&Failure::with(
            Kind::OutputUnavailable,
            "bluez_output.headset"
        ))
    );
    sink(&mut graph, 22, "alsa_output.analog", Some("Copy"), "10");
    let ambiguous = graph.find("alsa_output.analog").unwrap_err();
    assert_eq!(
        ambiguous.downcast_ref::<Failure>().map(|f| f.kind),
        Some(Kind::OutputAmbiguous)
    );
    assert!(graph.outputs().iter().all(|output| !output.selectable));
}

#[test]
fn a_stream_reports_the_sink_it_is_linked_to() {
    let mut graph = Graph::default();
    sink(
        &mut graph,
        20,
        "alsa_output.analog",
        Some("Internal Audio"),
        "10",
    );
    assert_eq!(graph.linked(50), None);
    let link = [("link.output.node", "50"), ("link.input.node", "20")];
    assert!(!add(&mut graph, 60, Global::Link, &link));
    assert_eq!(
        graph.linked(50),
        Some(("alsa_output.analog".into(), "Internal Audio".into()))
    );
    graph.remove(60);
    assert_eq!(graph.linked(50), None);
}
