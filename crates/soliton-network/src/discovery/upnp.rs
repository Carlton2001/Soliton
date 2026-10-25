use crate::{
    Device, Protocol,
    dlna::{self, Service, Services},
};
use anyhow::{Context, Result, ensure};
use reqwest::Url;
use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::{net::UdpSocket, task::JoinSet};

const SEARCH: &[u8] = b"M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\r\n";

pub(super) async fn scan() -> Result<Vec<Device>> {
    let interfaces = tokio::task::spawn_blocking(if_addrs::get_if_addrs).await??;
    let mut addresses: Vec<_> = interfaces
        .iter()
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(ip) if super::local(ip) => Some(ip),
            _ => None,
        })
        .collect();
    addresses.sort();
    addresses.dedup();
    addresses.truncate(8);
    let mut searches = JoinSet::new();
    for ip in addresses {
        searches.spawn(search(ip));
    }
    let mut locations = HashSet::new();
    let mut any_success = false;
    let mut last_error = None;
    while let Some(result) = searches.join_next().await {
        match result {
            Ok(Ok(found)) => {
                any_success = true;
                locations.extend(found);
            }
            Ok(Err(error)) => last_error = Some(error),
            Err(error) => last_error = Some(error.into()),
        }
    }
    if !any_success && let Some(error) = last_error {
        return Err(error);
    }
    let client = dlna::client()?;
    let mut requests = JoinSet::new();
    let mut locations = locations.into_iter().take(128);
    let mut devices = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        while requests.len() < 4 {
            let Some((url, peer)) = locations.next() else {
                break;
            };
            let client = client.clone();
            requests.spawn(async move {
                let url = checked_url(&url, peer)?;
                let response = client.get(url.clone()).send().await?.error_for_status()?;
                let xml = dlna::text(response).await?;
                description(&xml, &url, peer)
            });
        }
        let Ok(Some(result)) = tokio::time::timeout_at(deadline, requests.join_next()).await else {
            requests.abort_all();
            break;
        };
        // One malformed or disappearing device cannot hide other receivers.
        if let Ok(Ok(device)) = result {
            devices.push(device);
        }
    }
    Ok(devices)
}

async fn search(ip: Ipv4Addr) -> Result<Vec<(String, Ipv4Addr)>> {
    let socket = std::net::UdpSocket::bind((ip, 0))?;
    socket.set_nonblocking(true)?;
    socket.set_multicast_ttl_v4(2)?;
    socket2::SockRef::from(&socket).set_multicast_if_v4(&ip)?;
    let socket = UdpSocket::from_std(socket)?;
    socket.send_to(SEARCH, "239.255.255.250:1900").await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut buffer = [0; 16 * 1024];
    let mut found = Vec::new();
    for _ in 0..256 {
        let Ok(received) = tokio::time::timeout_at(deadline, socket.recv_from(&mut buffer)).await
        else {
            break;
        };
        let (length, sender) = received?;
        if let SocketAddr::V4(sender) = sender
            && super::local(*sender.ip())
            && let Some(url) = location(&buffer[..length])
        {
            found.push((url, *sender.ip()));
        }
    }
    Ok(found)
}

fn location(data: &[u8]) -> Option<String> {
    let data = std::str::from_utf8(data).ok()?;
    let mut lines = data.lines();
    if !lines.next()?.starts_with("HTTP/1.1 200") {
        return None;
    }
    lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("location"))
        .map(|(_, value)| value.trim().to_string())
}

fn checked_url(value: &str, peer: Ipv4Addr) -> Result<Url> {
    let url = Url::parse(value)?;
    ensure!(
        url.scheme() == "http" && url.username().is_empty() && url.password().is_none(),
        "Invalid UPnP URL"
    );
    ensure!(
        url.host_str().and_then(|h| h.parse::<Ipv4Addr>().ok()) == Some(peer),
        "UPnP URL must address its announcing device"
    );
    Ok(url)
}

fn description(xml: &str, location: &Url, peer: Ipv4Addr) -> Result<Device> {
    let doc = roxmltree::Document::parse(xml)?;
    let base = doc
        .descendants()
        .find(|n| n.has_tag_name("URLBase"))
        .and_then(|n| n.text())
        .map(|u| checked_url(u.trim(), peer))
        .transpose()?
        .unwrap_or_else(|| location.clone());
    let renderer = doc
        .descendants()
        .filter(|n| n.has_tag_name("device"))
        .find(|device| {
            child(*device, "deviceType")
                .is_some_and(|t| t.starts_with("urn:schemas-upnp-org:device:MediaRenderer:"))
        })
        .context("No UPnP MediaRenderer")?;
    let mut transport = None;
    let mut rendering = None;
    let mut connection = None;
    for service in renderer
        .children()
        .filter(|n| n.has_tag_name("serviceList"))
        .flat_map(|n| n.children())
        .filter(|n| n.has_tag_name("service"))
    {
        let Some(kind) = child(service, "serviceType") else {
            continue;
        };
        let Some(control) = child(service, "controlURL") else {
            continue;
        };
        let url = checked_url(base.join(control)?.as_str(), peer)?.to_string();
        let spec = Service {
            kind: kind.into(),
            url,
        };
        if kind.starts_with("urn:schemas-upnp-org:service:AVTransport:") {
            transport = Some(spec);
        } else if kind.starts_with("urn:schemas-upnp-org:service:RenderingControl:") {
            rendering = Some(spec);
        } else if kind.starts_with("urn:schemas-upnp-org:service:ConnectionManager:") {
            connection = Some(spec);
        }
    }
    let udn = child(renderer, "UDN").context("Missing UPnP device identity")?;
    ensure!(
        !udn.is_empty() && udn.len() <= 512,
        "Invalid UPnP device identity"
    );
    Ok(Device {
        id: format!("upnp:{udn}"),
        name: child(renderer, "friendlyName")
            .unwrap_or(udn)
            .chars()
            .take(200)
            .collect(),
        protocol: Protocol::Upnp,
        address: SocketAddr::new(peer.into(), location.port_or_known_default().unwrap_or(80)),
        services: Some(Services {
            transport: transport.context("Missing AVTransport")?,
            rendering,
            connection,
        }),
    })
}

fn child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.children()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .map(str::trim)
}

#[cfg(test)]
#[path = "../tests/upnp_discovery.rs"]
mod tests;
