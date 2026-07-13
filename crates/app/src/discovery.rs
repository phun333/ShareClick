//! Zero-config peer discovery over mDNS (`_shareclick._udp.local.`).
//!
//! The server advertises its name + port; a client can find it without anyone
//! typing an IP address. Discovery only locates the peer — the encrypted
//! handshake still authenticates it, so an imposter advertising the same
//! service cannot impersonate the real server without the PSK.

#![cfg(feature = "native")]

use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

const SERVICE_TYPE: &str = "_shareclick._udp.local.";

/// Keeps the advertised service alive; drop to stop advertising.
pub struct Advertiser {
    _daemon: ServiceDaemon,
}

/// Advertise this server so clients can discover it by name.
pub fn advertise(name: &str, port: u16, id: &str) -> anyhow::Result<Advertiser> {
    let daemon = ServiceDaemon::new()?;
    let host_name = format!("{name}.local.");
    let info = ServiceInfo::new(
        SERVICE_TYPE,
        name,
        &host_name,
        "",
        port,
        &[] as &[(&str, &str)],
    )?
    .enable_addr_auto();
    daemon.register(info)?;
    tracing::info!(%name, port, "advertising on mDNS as {SERVICE_TYPE}");
    Ok(Advertiser { _daemon: daemon })
}

/// Browse for a ShareClick server for up to `timeout`, returning the first
/// resolved IPv4 socket address.
pub fn discover(timeout: Duration) -> anyhow::Result<Option<SocketAddr>> {
    let daemon = ServiceDaemon::new()?;
    let receiver = daemon.browse(SERVICE_TYPE)?;
    let deadline = Instant::now() + timeout;

    let found = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break None;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(ip) = first_v4(&info) {
                    break Some(SocketAddr::new(IpAddr::V4(ip), info.get_port()));
                }
            }
            Ok(_) => {}
            Err(_) => break None, // timed out
        }
    };

    let _ = daemon.shutdown();
    Ok(found)
}

/// List every server seen within `timeout` (used by the `discover` CLI).
pub fn list(timeout: Duration) -> anyhow::Result<Vec<(String, SocketAddr, String)>> {
    let daemon = ServiceDaemon::new()?;
    let receiver = daemon.browse(SERVICE_TYPE)?;
    let deadline = Instant::now() + timeout;
    let mut out = Vec::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(ip) = first_v4(&info) {
                    let id = info
                        .get_property_val_str("id")
                        .unwrap_or_default()
                        .to_string();
                    out.push((
                        info.get_fullname().to_string(),
                        SocketAddr::new(IpAddr::V4(ip), info.get_port()),
                        id,
                    ));
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = daemon.shutdown();
    Ok(out)
}

fn first_v4(info: &ServiceInfo) -> Option<std::net::Ipv4Addr> {
    info.get_addresses_v4().into_iter().next().copied()
}
