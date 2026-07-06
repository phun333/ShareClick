//! Server (`serve`) and client (`connect`) run loops wiring capture + transport
//! + injection together. Native-only (needs input capture/injection).

#![cfg(feature = "native")]

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc;
use std::time::Duration;

use shareclick_protocol::InputMsg;

use crate::capture;
use crate::emit::Injector;
use crate::transport::InputChannel;

/// Server: shares this machine's keyboard & mouse. Learns the client address
/// from its first heartbeat, then streams coalesced input batches to it.
pub fn serve(bind: &str) -> anyhow::Result<()> {
    let bind_addr: SocketAddr = bind
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve bind addr {bind}"))?;
    let channel = InputChannel::bind(bind_addr, None)?;
    channel.set_read_timeout(Some(Duration::from_millis(1)))?;
    tracing::info!(%bind_addr, "serving input; grant Accessibility permission on macOS");

    // Capture runs on its own thread (rdev::listen blocks).
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Err(e) = capture::run(tx) {
            tracing::error!(error = %e, "capture thread stopped");
        }
    });

    let mut peer: Option<SocketAddr> = None;
    let mut buf = [0u8; 2048];
    loop {
        // Discover / refresh the client address from inbound heartbeats.
        if let Ok(Some((pkt, from))) = channel.recv(&mut buf) {
            if peer != Some(from) {
                tracing::info!(%from, "client connected");
                peer = Some(from);
            }
            if let InputMsg::Ping { nonce, echo_nanos } = pkt.msg {
                let _ = channel.send_to(InputMsg::Pong { nonce, echo_nanos }, from);
            }
        }

        // Coalesce everything captured since the last tick into one batch.
        let mut batch = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            batch.push(ev);
        }
        if !batch.is_empty() {
            if let Some(p) = peer {
                let _ = channel.send_to(InputMsg::Events(batch), p);
            }
        } else {
            std::thread::sleep(Duration::from_micros(500));
        }
    }
}

/// Client: receives input batches and injects them locally.
pub fn connect(server: &str) -> anyhow::Result<()> {
    let server_addr: SocketAddr = server
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve server addr {server}"))?;
    let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let channel = InputChannel::bind(bind_addr, Some(server_addr))?;
    channel.set_read_timeout(Some(Duration::from_millis(200)))?;
    tracing::info!(%server_addr, "connecting; grant Accessibility permission on macOS");

    let mut injector = Injector::new()?;

    // Announce ourselves so the server learns our address, then keep the path
    // warm by re-pinging on every read timeout (below).
    channel.send(InputMsg::Ping { nonce: 0, echo_nanos: 0 })?;

    let mut buf = [0u8; 2048];
    loop {
        match channel.recv(&mut buf) {
            Ok(Some((pkt, _))) => match pkt.msg {
                InputMsg::Events(events) => {
                    for ev in events {
                        if let Err(e) = injector.apply(ev) {
                            tracing::warn!(error = %e, "inject failed");
                        }
                    }
                }
                InputMsg::Pong { .. } | InputMsg::Ping { .. } => {}
                InputMsg::Enter { .. } | InputMsg::Leave => {}
            },
            Ok(None) => {}
            Err(_) => {
                // read timeout: re-announce so the server keeps sending here.
                let _ = channel.send(InputMsg::Ping { nonce: 0, echo_nanos: 0 });
            }
        }
    }
}
