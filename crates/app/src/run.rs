//! Server (`serve`) and client (`connect`) run loops wiring capture + transport
//! + injection + encryption together. Native-only (needs input capture).
//!
//! Session bring-up:
//!  1. A TCP handshake (X25519 + PSK) authenticates the peers and derives two
//!     encrypted sessions — one for the UDP input channel, one for the TCP bulk
//!     channel (clipboard + files).
//!  2. The input session keys the UDP channel; the bulk session keys the TCP
//!     connection. From then on every byte on the wire is authenticated
//!     ChaCha20-Poly1305.

#![cfg(feature = "native")]

use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Duration;

use shareclick_protocol::crypto::{Role, Session};
use shareclick_protocol::{BulkMsg, ClipboardData, Edge, InputEvent, InputMsg};

/// The client enters from the edge opposite the one the server's cursor left by
/// (leave the Mac's right edge → arrive at the PC's left edge).
fn opposite(e: Edge) -> Edge {
    match e {
        Edge::Left => Edge::Right,
        Edge::Right => Edge::Left,
        Edge::Top => Edge::Bottom,
        Edge::Bottom => Edge::Top,
    }
}

use crate::bulk::BulkConn;
use crate::capture;
use crate::clipboard;
use crate::config::Config;
use crate::control::Control;
use crate::discovery;
use crate::cursor::CursorTracker;
use crate::edge::EdgeConfig;
use crate::filexfer::FileReceiver;
use crate::transport::InputChannel;

/// Load the config or explain how to create one.
fn load_config() -> anyhow::Result<Config> {
    let path = Config::default_path();
    if !path.exists() {
        anyhow::bail!(
            "no config at {} — run `shareclick init-config` and edit the PSK + layout first",
            path.display()
        );
    }
    Config::load(&path)
}

/// This machine's screen size: a config override if set, else auto-detected
/// from the OS, else a safe fallback.
fn screen_size(cfg: &Config) -> (u32, u32) {
    if let Some(s) = cfg.machine(&cfg.name).and_then(|m| m.screen) {
        return s;
    }
    crate::emit::main_display_size().unwrap_or((1920, 1080))
}

fn resolve(addr: &str) -> anyhow::Result<SocketAddr> {
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve address {addr}"))
}

/// Wire clipboard + file sync onto one (already-encrypted) bulk connection.
/// Blocks on the reader loop; returns when the peer disconnects.
fn serve_bulk(conn: BulkConn) -> anyhow::Result<()> {
    let last = clipboard::shared_last();
    let (out_tx, out_rx) = mpsc::channel::<BulkMsg>();
    let (in_tx, in_rx) = mpsc::channel::<ClipboardData>();

    let mut wconn = conn.try_clone()?;
    std::thread::spawn(move || {
        while let Ok(msg) = out_rx.recv() {
            if wconn.send(&msg).is_err() {
                break;
            }
        }
    });
    let last_apply = last.clone();
    std::thread::spawn(move || clipboard::apply(in_rx, last_apply));
    std::thread::spawn(move || clipboard::watch(out_tx, last));

    let mut receiver = FileReceiver::new("received");
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
            Ok(_) => {}
            Err(_) => return Ok(()),
        }
    }
}

/// The server's encrypted input pump: learn the client's UDP address from its
/// pings, then stream coalesced, encrypted input batches while control is held
/// by the client. Returns on socket error (peer likely gone).
fn run_server_input(
    udp: &InputChannel,
    rx: &Receiver<InputEvent>,
    control: &Control,
) -> anyhow::Result<()> {
    let mut peer: Option<SocketAddr> = None;
    let mut prev_active = false;
    let mut buf = [0u8; 2048];
    loop {
        if let Ok(Some((pkt, from))) = udp.recv(&mut buf) {
            if peer != Some(from) {
                tracing::info!(%from, "client input channel online");
                peer = Some(from);
            }
            match pkt.msg {
                InputMsg::Ping { nonce, echo_nanos } => {
                    let _ = udp.send_to(InputMsg::Pong { nonce, echo_nanos }, from);
                }
                // Client's cursor crossed back over the border → reclaim.
                InputMsg::Leave => {
                    control.active.store(false, Ordering::Relaxed);
                    tracing::info!("client returned control");
                }
                _ => {}
            }
        }

        // Announce control transitions to the client.
        let active = control.active.load(Ordering::Relaxed);
        if active != prev_active {
            if let Some(p) = peer {
                if active {
                    // Only ask the client to track a return border when control
                    // was handed over by an actual edge crossing. Manual toggles
                    // (both-Shift / F12) send nothing — the user toggles back.
                    if let Some((edge, entry)) = *control.entry.lock().unwrap() {
                        let _ = udp.send_to(InputMsg::Enter { edge: opposite(edge), entry }, p);
                    }
                } else {
                    let _ = udp.send_to(InputMsg::Leave, p);
                }
            }
            prev_active = active;
        }

        let mut batch = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            batch.push(ev);
        }
        if !batch.is_empty() {
            if let Some(p) = peer {
                let _ = udp.send_to(InputMsg::Events(batch), p);
            }
        } else {
            std::thread::sleep(Duration::from_micros(500));
        }
    }
}

/// Server: shares this machine's keyboard & mouse.
pub fn serve(bind: &str) -> anyhow::Result<()> {
    let cfg = load_config()?;
    let psk = cfg.psk.clone().into_bytes();
    let bind_addr = resolve(bind)?;
    tracing::info!(%bind_addr, name = %cfg.name, "serving; press F12 to hand control to the client");
    tracing::info!("grant Accessibility permission on macOS for capture to work");

    // Screen size: auto-detected from the OS, or a config override if present.
    let (sw, sh) = screen_size(&cfg);
    tracing::info!(width = sw, height = sh, "screen size (auto-detected)");

    // Monitor manager: which of our screen edges border another machine?
    let edges = match cfg.machine(&cfg.name) {
        Some(m) if cfg.auto_edge_switch => {
            EdgeConfig::new(sw, sh, m.left.is_some(), m.right.is_some(), m.top.is_some(), m.bottom.is_some())
        }
        _ => EdgeConfig::none(),
    };

    // Capture runs once, globally, feeding a channel. Control starts local.
    let (tx, rx) = mpsc::channel();
    let control = Arc::new(Control::new());
    let control_cap = control.clone();
    std::thread::spawn(move || {
        if let Err(e) = capture::run(tx, control_cap, edges) {
            tracing::error!(error = %e, "capture thread stopped");
        }
    });

    // Advertise over mDNS so clients can find us without an IP (kept alive for
    // the process lifetime).
    let _advert = discovery::advertise(&cfg.name, bind_addr.port())
        .map_err(|e| tracing::warn!(error = %e, "mDNS advertise failed"))
        .ok();

    let listener = TcpListener::bind(bind_addr)?;
    loop {
        let (stream, _) = listener.accept()?;
        let peer_ip = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
        let (conn, input_sess) = match BulkConn::handshake(stream, &psk, Role::Responder) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(%peer_ip, error = %e, "handshake/auth failed; check the PSK");
                continue;
            }
        };
        tracing::info!(%peer_ip, "client authenticated (encrypted session established)");

        // Bulk channel (clipboard/files) in the background.
        std::thread::spawn(move || {
            let _ = serve_bulk(conn);
        });

        // Encrypted UDP input channel for this session.
        let udp = InputChannel::bind(bind_addr, None)?.with_cipher(Arc::new(input_sess));
        udp.set_read_timeout(Some(Duration::from_millis(1)))?;
        if let Err(e) = run_server_input(&udp, &rx, &control) {
            tracing::warn!(error = %e, "input session ended; awaiting new client");
        }
    }
}

/// Client: receives input batches and injects them locally. `server` overrides
/// the config's `server_host`; either may omit the port (config `port` used).
pub fn connect(server: Option<&str>) -> anyhow::Result<()> {
    let cfg = load_config()?;
    let psk = cfg.psk.clone().into_bytes();
    let with_port = |h: &str| -> String {
        if h.contains(':') { h.to_string() } else { format!("{h}:{}", cfg.port) }
    };
    let server_addr = match server.map(|s| s.to_string()).or_else(|| cfg.server_host.clone()) {
        Some(host) => resolve(&with_port(&host))?,
        None => {
            tracing::info!("no server configured; searching via mDNS (3s)…");
            discovery::discover(Duration::from_secs(3))?.ok_or_else(|| {
                anyhow::anyhow!("no server found via mDNS; pass a host or set `server_host`")
            })?
        }
    };
    tracing::info!(%server_addr, name = %cfg.name, "connecting; grant Accessibility permission on macOS");

    // Handshake over TCP first, then key both channels from it.
    let stream = TcpStream::connect(server_addr)?;
    let (conn, input_sess): (BulkConn, Session) =
        BulkConn::handshake(stream, &psk, Role::Initiator)?;
    tracing::info!("authenticated with server (encrypted session established)");

    // Bulk channel (clipboard/files).
    std::thread::spawn(move || {
        if let Err(e) = serve_bulk(conn) {
            tracing::warn!(error = %e, "bulk channel closed");
        }
    });

    // Encrypted UDP input channel.
    let channel = InputChannel::bind("0.0.0.0:0".parse().unwrap(), Some(server_addr))?
        .with_cipher(Arc::new(input_sess));
    channel.set_read_timeout(Some(Duration::from_millis(200)))?;
    let mut injector = crate::emit::Injector::new()?;

    // Track our cursor so we can auto-return control at the border edge.
    let (cw, ch) = screen_size(&cfg);
    tracing::info!(width = cw, height = ch, "screen size (auto-detected)");
    let mut tracker = CursorTracker::new(cw, ch);
    let mut controlling = false;

    // Announce ourselves; re-ping on timeout to keep the path warm.
    channel.send(InputMsg::Ping { nonce: 0, echo_nanos: 0 })?;

    let mut buf = [0u8; 2048];
    loop {
        match channel.recv(&mut buf) {
            Ok(Some((pkt, _))) => match pkt.msg {
                InputMsg::Enter { edge, entry } => {
                    tracker.enter(edge, entry);
                    controlling = true;
                    tracing::info!(?edge, "gained control from server");
                }
                InputMsg::Leave => {
                    tracker.leave();
                    controlling = false;
                    tracing::info!("server revoked control");
                }
                InputMsg::Events(events) => {
                    for ev in events {
                        if let InputEvent::MouseMove { dx, dy } = ev {
                            if controlling && tracker.moved(dx, dy) {
                                // Cursor left back toward the server → return it.
                                controlling = false;
                                let _ = channel.send(InputMsg::Leave);
                                tracing::info!("cursor hit border; returning control");
                            }
                        }
                        if let Err(e) = injector.apply(ev) {
                            tracing::warn!(error = %e, "inject failed");
                        }
                    }
                }
                _ => {}
            },
            Ok(None) => {}
            Err(_) => {
                let _ = channel.send(InputMsg::Ping { nonce: 0, echo_nanos: 0 });
            }
        }
    }
}
