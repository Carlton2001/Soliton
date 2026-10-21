//! Network discovery, renderer control and authorized media delivery. No UI or
//! local decoding: the selected receiver reads the original media over HTTP.
mod cast;
mod controller;
mod discovery;
mod dlna;
mod http;
mod media;
mod service;

use rap_core::model::Track;
pub use service::Network;
use std::{net::SocketAddr, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Upnp,
    Cast,
}

impl Protocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::Upnp => "DLNA / UPnP",
            Self::Cast => "Google Cast",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub address: SocketAddr,
    pub(crate) services: Option<dlna::Services>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Stopped,
    Playing,
    Paused,
    Buffering,
    Finished,
    Failed,
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    /// Identifies a particular load; late updates cannot advance a newer track.
    pub generation: u64,
    pub state: State,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: Option<f32>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub devices: Arc<Vec<Device>>,
    pub discovery_error: Option<String>,
    pub discovering: bool,
    pub status: Option<Status>,
}

pub enum Command {
    Connect(Device),
    Load {
        track: Track,
        position_ms: u64,
        autoplay: bool,
        volume: f32,
    },
    Play,
    Pause,
    Stop,
    Seek(u64),
    Volume(f32),
    Disconnect,
}
