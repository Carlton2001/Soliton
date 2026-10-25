use crate::{Command, Device, Protocol, State, Status, cast, dlna, http::Server, media};
use anyhow::{Context, Result};

enum Connection {
    Cast(cast::Session),
    Upnp(dlna::Session),
}

#[cfg(test)]
#[path = "tests/controller.rs"]
mod tests;

impl Connection {
    async fn poll(&mut self) -> Result<Status> {
        match self {
            Self::Cast(s) => s.poll().await,
            Self::Upnp(s) => s.poll().await,
        }
    }
    async fn disconnect(&mut self) {
        match self {
            Self::Cast(s) => s.disconnect().await,
            Self::Upnp(s) => {
                let _ = s.stop().await;
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct Controller {
    device: Option<Device>,
    connection: Option<Connection>,
    server: Option<Server>,
    pub status: Option<Status>,
    generation: u64,
}

impl Controller {
    pub async fn command(&mut self, command: Command) -> Result<Option<Status>> {
        match command {
            Command::Connect(device) => {
                if self.device.as_ref().is_some_and(|d| d.id == device.id)
                    && self.connection.is_some()
                    && self
                        .status
                        .as_ref()
                        .is_none_or(|s| s.state != State::Failed)
                {
                    return Ok(self.status.clone());
                }
                let connection = match device.protocol {
                    Protocol::Cast => Connection::Cast(cast::Session::connect(&device).await?),
                    Protocol::Upnp => Connection::Upnp(dlna::Session::connect(&device).await?),
                };
                self.disconnect().await;
                self.device = Some(device);
                self.connection = Some(connection);
                self.status = None;
                Ok(None)
            }
            Command::Disconnect => {
                self.disconnect().await;
                self.device = None;
                Ok(None)
            }
            Command::Load {
                track,
                position_ms,
                autoplay,
                volume,
            } => {
                // Reconnect only following an explicit playback request.
                if self.connection.is_none()
                    || self
                        .status
                        .as_ref()
                        .is_some_and(|s| s.state == State::Failed)
                {
                    let device = self.device.clone().context("No network output selected")?;
                    let connection = match device.protocol {
                        Protocol::Cast => Connection::Cast(cast::Session::connect(&device).await?),
                        Protocol::Upnp => Connection::Upnp(dlna::Session::connect(&device).await?),
                    };
                    self.connection = Some(connection);
                }
                let connection = self
                    .connection
                    .as_mut()
                    .context("Network output is disconnected")?;
                let mime = match connection {
                    Connection::Cast(_) => media::mime(&track)?.into(),
                    Connection::Upnp(s) => s.mime(&track)?,
                };
                if self.server.is_none() {
                    self.server = Some(
                        Server::start(
                            self.device
                                .as_ref()
                                .context("No network output selected")?
                                .address
                                .ip(),
                        )
                        .await?,
                    );
                }
                let urls = self
                    .server
                    .as_ref()
                    .unwrap()
                    .authorize(&track, mime)
                    .await?;
                self.generation = self.generation.wrapping_add(1);
                let mut status = match connection {
                    Connection::Cast(s) => {
                        s.load(&track, &urls, position_ms, autoplay, volume).await?
                    }
                    Connection::Upnp(s) => {
                        s.load(&track, &urls, position_ms, autoplay, volume).await?
                    }
                };
                if status.duration_ms == 0 {
                    status.duration_ms = track.metadata.duration_ms;
                }
                self.status = None;
                Ok(Some(self.accept(status)))
            }
            command => {
                let connection = self
                    .connection
                    .as_mut()
                    .context("Network output is disconnected")?;
                macro_rules! operate { ($operation:ident $(, $argument:expr)?) => {
                    match connection {
                        Connection::Cast(s) => s.$operation($($argument)?).await?,
                        Connection::Upnp(s) => s.$operation($($argument)?).await?,
                    }
                }; }
                let status = match command {
                    Command::Play => operate!(play),
                    Command::Pause => operate!(pause),
                    Command::Stop => {
                        let status = operate!(stop);
                        self.generation = self.generation.wrapping_add(1);
                        if let Some(server) = &self.server {
                            server.revoke().await;
                        }
                        status
                    }
                    Command::Seek(position) => operate!(seek, position),
                    Command::Volume(value) => operate!(volume, value),
                    _ => unreachable!(),
                };
                Ok(Some(self.accept(status)))
            }
        }
    }

    fn accept(&mut self, mut status: Status) -> Status {
        status.generation = self.generation;
        if status.duration_ms == 0 {
            status.duration_ms = self.status.as_ref().map_or(0, |s| s.duration_ms);
        }
        self.status = Some(status.clone());
        status
    }

    pub async fn poll(&mut self) -> Result<()> {
        if self
            .status
            .as_ref()
            .is_none_or(|s| matches!(s.state, State::Stopped | State::Finished | State::Failed))
        {
            return Ok(());
        }
        let status = self
            .connection
            .as_mut()
            .context("Network output is disconnected")?
            .poll()
            .await?;
        if matches!(status.state, State::Finished | State::Stopped)
            && let Some(server) = &self.server
        {
            server.revoke().await;
        }
        self.accept(status);
        Ok(())
    }

    pub async fn fail(&mut self, error: anyhow::Error) {
        if let Some(server) = &self.server {
            server.revoke().await;
        }
        let mut status = self.status.clone().unwrap_or_default();
        status.generation = self.generation;
        status.state = State::Failed;
        status.error = Some(format!("{error:#}"));
        self.status = Some(status);
    }

    pub async fn disconnect(&mut self) {
        if let Some(mut connection) = self.connection.take() {
            let _ =
                tokio::time::timeout(std::time::Duration::from_secs(2), connection.disconnect())
                    .await;
        }
        if let Some(server) = self.server.take() {
            server.revoke().await;
        }
        self.status = None;
    }
}
