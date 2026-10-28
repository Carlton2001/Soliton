//! Sinks, their devices and links, as announced by the PipeWire registry.
use crate::{OutputDevice, devices::describe};
use soliton_core::failure::{Failure, Kind};
use std::collections::HashMap;

pub(super) enum Global {
    Node,
    Device,
    Link,
}

struct Sink {
    name: String,
    label: String,
    device: Option<u32>,
}

#[derive(Default)]
pub(super) struct Graph {
    sinks: HashMap<u32, Sink>,
    /// Device id → whether its API is Bluetooth.
    bluetooth: HashMap<u32, bool>,
    /// Link id → (output node, input node).
    links: HashMap<u32, (u32, u32)>,
}

impl Graph {
    /// Records a global; `true` when the listed outputs may have changed.
    pub fn add(&mut self, id: u32, kind: Global, props: impl Fn(&str) -> Option<String>) -> bool {
        match kind {
            Global::Node => {
                if props("media.class").as_deref() != Some("Audio/Sink") {
                    return false;
                }
                let Some(name) = props("node.name") else {
                    return false;
                };
                let label = props("node.description")
                    .or_else(|| props("node.nick"))
                    .unwrap_or_else(|| name.clone());
                let device = props("device.id").and_then(|id| id.parse().ok());
                self.sinks.insert(
                    id,
                    Sink {
                        name,
                        label,
                        device,
                    },
                );
                true
            }
            Global::Device => {
                let bluetooth = props("device.api").as_deref() == Some("bluez5");
                self.bluetooth.insert(id, bluetooth);
                self.sinks.values().any(|sink| sink.device == Some(id))
            }
            Global::Link => {
                let node = |key| props(key).and_then(|id| id.parse().ok());
                if let (Some(output), Some(input)) =
                    (node("link.output.node"), node("link.input.node"))
                {
                    self.links.insert(id, (output, input));
                }
                false
            }
        }
    }

    /// Forgets a global; `true` when the listed outputs may have changed.
    pub fn remove(&mut self, id: u32) -> bool {
        self.links.remove(&id);
        let device = self.bluetooth.remove(&id).is_some();
        self.sinks.remove(&id).is_some()
            || (device && self.sinks.values().any(|sink| sink.device == Some(id)))
    }

    pub fn outputs(&self) -> Vec<OutputDevice> {
        describe(
            self.sinks
                .values()
                .map(|sink| {
                    let bluetooth = sink
                        .device
                        .and_then(|device| self.bluetooth.get(&device).copied())
                        .unwrap_or(false);
                    OutputDevice::new(sink.name.clone(), sink.label.clone(), bluetooth)
                })
                .collect(),
        )
    }

    /// The sink id of a node name, unless it is missing or ambiguous.
    pub fn find(&self, name: &str) -> anyhow::Result<u32> {
        let mut matching = self.sinks.iter().filter(|(_, sink)| sink.name == name);
        match (matching.next(), matching.next()) {
            (Some((&id, _)), None) => Ok(id),
            (None, _) => Err(Failure::with(Kind::OutputUnavailable, name).into()),
            (Some(_), Some(_)) => Err(Failure::with(Kind::OutputAmbiguous, name).into()),
        }
    }

    pub fn has_sinks(&self) -> bool {
        !self.sinks.is_empty()
    }

    /// Name and label of a sink.
    pub fn sink(&self, id: u32) -> Option<(String, String)> {
        self.sinks
            .get(&id)
            .map(|sink| (sink.name.clone(), sink.label.clone()))
    }

    /// Name and label of the sink a node plays to.
    pub fn linked(&self, node: u32) -> Option<(String, String)> {
        let (_, input) = self.links.values().find(|(output, _)| *output == node)?;
        self.sink(*input)
    }
}

#[cfg(test)]
#[path = "tests/graph.rs"]
mod tests;
