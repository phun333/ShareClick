//! Server (`serve`) and client (`connect`) run loops wiring capture + transport
//! + injection together. Native-only (needs input capture/injection).

#![cfg(feature = "native")]

use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use shareclick_protocol::{BulkMsg, ClipboardData, InputMsg};

use crate::bulk::BulkConn;
use crate::capture;
use crate::clipboard;
use crate::filexfer::FileReceiver;
use crate::emit::Injector;
use crate::transport::InputChannel;

/// Wire clipboard (and later file) sync onto one bulk connection. Blocks on the
/// reader loop and returns when the peer disconnects; spawns writer/watch/apply
/// helpers alongside.
fn serve_bulk(conn: BulkConn) -> anyhow::Result<()> {
    let last = clipboard::shared_last();
    let (out_tx, out_rx) = mpsc::channel::<BulkMsg>();
    let (in_tx, in_rx) = mpsc::channel::<ClipboardData>();

    // Writer: drains outbound messages onto the socket.
    let mut wconn = conn.try_clone()?;
    std::thread::spawn(move || {
        while let Ok(msg) = out_rx.recv() {
            if wconn.send(&msg).is_err() {
                break;
            }
        }
    });
    // Apply + watch clipboard.
    let last_apply = last.clone();
    std::thread::spawn(move || clipboard::apply(in_rx, last_apply));
    std::thread::spawn(move || clipboard::watch(out_tx, last));

    // Incoming files land in ./received next to the binary's working dir.
    let mut receiver = FileReceiver::new("received");

    // Reader loop (this thread) routes inbound messages.
    let mut rconn = conn;
    loop {
        match rconn.recv() {
            Ok(BulkMsg::Clipboard(data)) => {
                let _ = in_tx.send(data);
            }
            Ok(msg @ (BulkMsg::FileBegin { .. }
            | BulkMsg::FileChunk { .. }
            | BulkMsg::FileEnd { .. })) => {
                if let Err(e) = receiver.handle(&msg) {
                    tracing::warn!(error = %e, "file receive failed");
                }
            }
            Ok(_) => {} // Hello/Heartbeat handled later
            Err(_) => return Ok(()), // peer gone
        }
    }
}

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

    // Bulk channel (clipboard/files) on the same port over TCP.
    if let Ok(listener) = TcpListener::bind(bind_addr) {
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                if let Ok(conn) = BulkConn::new(stream) {
                    std::thread::spawn(move || {
                        let _ = serve_bulk(conn);
                    });
                }
            }
        });
    }

    // Capture runs on its own thread (rdev::grab blocks). Control starts on the
    // local machine; press F12 to hand it to the client (and again to reclaim).
    let (tx, rx) = mpsc::channel();
    let active = Arc::new(AtomicBool::new(false));
    let active_cap = active.clone();
    tracing::info!("press F12 to toggle control between this machine and the client");
    std::thread::spawn(move || {
        if let Err(e) = capture::run(tx, active_cap) {
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

    // Bulk channel (clipboard/files): connect over TCP, auto-reconnect.
    std::thread::spawn(move || loop {
        match TcpStream::connect(server_addr) {
            Ok(stream) => {
                if let Ok(conn) = BulkConn::new(stream) {
                    let _ = serve_bulk(conn);
                }
            }
            Err(_) => {}
        }
        std::thread::sleep(Duration::from_secs(2));
    });

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
