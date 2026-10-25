//! SSDP and mDNS are independent of playback. Only recognized local renderer
//! services are resolved; discovery never launches a receiver or exposes files.
mod upnp;
use crate::{Device, Protocol};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};

#[derive(Clone, Default)]
pub(crate) struct Discovered {
    pub devices: Vec<Device>,
    pub error: Option<String>,
    pub scanning: bool,
}

const CAST: &str = "_googlecast._tcp.local.";

struct Mdns(ServiceDaemon);
impl Drop for Mdns {
    fn drop(&mut self) {
        let _ = self.0.shutdown();
    }
}

pub(crate) async fn run(mut refresh: mpsc::Receiver<()>, updates: watch::Sender<Discovered>) {
    let mdns = ServiceDaemon::new().and_then(|daemon| {
        let daemon = Mdns(daemon);
        daemon.0.disable_interface(mdns_sd::IfKind::IPv6)?;
        let events = daemon.0.browse(CAST)?;
        Ok((daemon, events))
    });
    let (_daemon, events, cast_error) = match mdns {
        Ok((daemon, events)) => (Some(daemon), Some(events), None),
        Err(error) => (None, None, Some(format!("Google Cast discovery: {error}"))),
    };
    let mut cast = HashMap::<String, Device>::new();
    let mut upnp = HashMap::<String, (Device, Instant)>::new();
    let mut scan = tokio::task::spawn(upnp::scan());
    let mut scanning = true;
    let mut next_scan = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut upnp_error = None;
    loop {
        tokio::select! {
            event = async {
                match &events { Some(events) => events.recv_async().await.ok(), None => std::future::pending().await }
            } => {
                match event {
                    Some(ServiceEvent::ServiceResolved(info)) => {
                        if let Some(device) = cast_device(&info) { cast.insert(info.get_fullname().into(), device); }
                    }
                    Some(ServiceEvent::ServiceRemoved(_, name)) => { cast.remove(&name); }
                    Some(_) => continue,
                    None => break,
                }
            }
            result = &mut scan, if scanning => {
                scanning = false;
                match result {
                    Ok(Ok(devices)) => {
                        for device in devices { upnp.insert(device.id.clone(), (device, Instant::now())); }
                        upnp_error = None;
                    }
                    Ok(Err(error)) => upnp_error = Some(format!("DLNA discovery: {error:#}")),
                    Err(error) => upnp_error = Some(format!("DLNA discovery: {error}")),
                }
                upnp.retain(|_, (_, seen)| seen.elapsed() < Duration::from_secs(90));
            }
            request = refresh.recv() => {
                if request.is_none() { break; }
                if !scanning {
                    scan = tokio::task::spawn(upnp::scan());
                    scanning = true;
                    next_scan = tokio::time::Instant::now() + Duration::from_secs(30);
                }
            }
            _ = tokio::time::sleep_until(next_scan), if !scanning => {
                scan = tokio::task::spawn(upnp::scan());
                scanning = true;
                next_scan = tokio::time::Instant::now() + Duration::from_secs(30);
            }
        }
        let mut devices: Vec<_> = cast
            .values()
            .cloned()
            .chain(upnp.values().map(|(d, _)| d.clone()))
            .collect();
        devices.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
        devices.dedup_by(|a, b| a.id == b.id);
        let error = cast_error
            .iter()
            .chain(upnp_error.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        if updates
            .send(Discovered {
                devices,
                error: (!error.is_empty()).then_some(error),
                scanning,
            })
            .is_err()
        {
            break;
        }
    }
    scan.abort();
}

fn cast_device(info: &mdns_sd::ResolvedService) -> Option<Device> {
    let mut addresses: Vec<_> = info
        .get_addresses_v4()
        .into_iter()
        .filter(|ip| local(*ip))
        .collect();
    addresses.sort();
    let address = std::net::SocketAddr::new((*addresses.first()?).into(), info.get_port());
    if address.port() == 0 {
        return None;
    }
    let id = info
        .get_property_val_str("id")
        .filter(|s| !s.is_empty())
        .unwrap_or(info.get_fullname());
    let name = info
        .get_property_val_str("fn")
        .filter(|s| !s.is_empty())
        .unwrap_or(info.get_fullname());
    Some(Device {
        id: format!("cast:{id}"),
        name: name.chars().take(200).collect(),
        protocol: Protocol::Cast,
        address,
        services: None,
    })
}

pub(super) fn local(ip: std::net::Ipv4Addr) -> bool {
    ip.is_private() || ip.is_link_local()
}

#[cfg(test)]
#[path = "../tests/discovery.rs"]
mod tests;
