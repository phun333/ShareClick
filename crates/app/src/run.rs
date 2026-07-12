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
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shareclick_protocol::crypto::{Role, Session};
use shareclick_protocol::{BulkMsg, ClipboardData, Edge, InputEvent, InputMsg};

/// A batch that releases every modifier key on the client. Sent on every
/// control hand-off so a modifier held during the switch can't stay stuck down
/// on the other machine (the classic "Alt+Tab / Ctrl stuck" bug).
fn release_all_modifiers() -> InputMsg {
    use shareclick_protocol::Key::{LAlt, LCtrl, LMeta, LShift, RAlt, RCtrl, RMeta, RShift};
    InputMsg::Events(vec![
        InputEvent::Key { key: LCtrl, pressed: false },
        InputEvent::Key { key: RCtrl, pressed: false },
        InputEvent::Key { key: LAlt, pressed: false },
        InputEvent::Key { key: RAlt, pressed: false },
        InputEvent::Key { key: LShift, pressed: false },
        InputEvent::Key { key: RShift, pressed: false },
        InputEvent::Key { key: LMeta, pressed: false },
        InputEvent::Key { key: RMeta, pressed: false },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_maps_to_opposite_edge() {
        assert_eq!(opposite(Edge::Right), Edge::Left);
        assert_eq!(opposite(Edge::Left), Edge::Right);
        assert_eq!(opposite(Edge::Top), Edge::Bottom);
        assert_eq!(opposite(Edge::Bottom), Edge::Top);
        // Leave the server's RIGHT edge at y=432 → enter the client's LEFT edge
        // at y=432 on a 2560×1440 screen.
        assert_eq!(entry_point(opposite(Edge::Right), 432, 2560, 1440), (2, 432));
        // Leave BOTTOM at x=1440 → enter TOP at x=1440 on a 1920×1080 screen.
        assert_eq!(entry_point(opposite(Edge::Bottom), 1440, 1920, 1080), (1440, 2));
    }
}

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
use crate::edge::{
    client_return_span, entry_point, map_to_client, map_to_server, perp_dim, EdgeConfig,
};
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

/// This machine's screen size. Always prefer the LIVE OS-detected size so a
/// stale value in the config can never break edge detection or the offset math;
/// a config `screen` is only a fallback when detection isn't available.
fn screen_size(cfg: &Config) -> (u32, u32) {
    crate::emit::main_display_size()
        .ok()
        .or_else(|| cfg.machine(&cfg.name).and_then(|m| m.screen))
        .unwrap_or((1920, 1080))
}

fn resolve(addr: &str) -> anyhow::Result<SocketAddr> {
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve address {addr}"))
}

/// Zero-config auto-pairing: advertise ourselves and search for a peer on the
/// LAN, then connect automatically — no IP, no manual matching. If both sides
/// have an explicit `role`, that decides who serves; otherwise a deterministic
/// name tiebreaker makes exactly one side the server. The client retries until
/// the server is up, so start order doesn't matter.
#[cfg(feature = "native")]
pub fn pair() -> anyhow::Result<()> {
    let cfg = load_config()?;
    let me = cfg.name.clone();
    let port = cfg.port;
    // Keep advertising for the whole search so the peer can find us too.
    let _advert = discovery::advertise(&me, port)
        .map_err(|e| tracing::warn!(error = %e, "mDNS advertise failed"))
        .ok();
    tracing::info!(name = %me, "auto-pairing: advertising and searching for a peer…");

    let my_prefix = format!("{me}.");
    loop {
        let peers = discovery::list(Duration::from_secs(2)).unwrap_or_default();
        if let Some((fullname, addr)) = peers.into_iter().find(|(n, _)| !n.starts_with(&my_prefix)) {
            let peer = fullname.split('.').next().unwrap_or("peer").to_string();
            let am_server = match cfg.role.as_deref() {
                Some("server") => true,
                Some("client") => false,
                // No explicit role: the lexicographically smaller name serves.
                _ => me < peer,
            };
            if am_server {
                tracing::info!(%peer, "paired — running as server");
                return serve(&format!("0.0.0.0:{port}"));
            }
            tracing::info!(%peer, %addr, "paired — running as client");
            // The server may still be starting; retry until it answers.
            loop {
                match connect(Some(&addr.to_string())) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        tracing::warn!(error = %e, "connect failed; retrying in 2s");
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
            }
        }
        tracing::info!("no peer found yet; still searching…");
    }
}

/// Persist a peer's reported screen size into our config, so the settings window
/// can show the real remote resolution. The client reports it on connect (like
/// Deskflow's DINF message).
fn record_peer_screen(name: &str, screen: (u32, u32)) {
    let path = Config::default_path();
    if let Ok(mut cfg) = Config::load(&path) {
        if let Some(m) = cfg.machines.iter_mut().find(|m| m.name == name) {
            if m.screen != Some(screen) {
                m.screen = Some(screen);
                let _ = cfg.save(&path);
                tracing::info!(%name, width = screen.0, height = screen.1, "recorded peer screen size");
            }
        }
    }
}

/// Wire clipboard + file sync onto one (already-encrypted) bulk connection.
/// Blocks on the reader loop; returns when the peer disconnects.
fn serve_bulk(
    conn: BulkConn,
    hello: Option<BulkMsg>,
    peer_screen: Option<Arc<Mutex<(u32, u32)>>>,
) -> anyhow::Result<()> {
    let last = clipboard::shared_last();
    let (out_tx, out_rx) = mpsc::channel::<BulkMsg>();
    let (in_tx, in_rx) = mpsc::channel::<ClipboardData>();

    // Send our own screen size first (client → server), before any other frame,
    // so the encrypted send-counter stays in sync with the peer's recv-counter.
    if let Some(h) = hello {
        let _ = out_tx.send(h);
    }

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
            // Peer told us its screen size — use it LIVE for the offset math and
            // remember it for the settings window (Deskflow's DINF pattern).
            Ok(BulkMsg::Hello { name, screen, .. }) => {
                tracing::info!(peer = %name, width = screen.0, height = screen.1, "peer reported its screen size (Hello)");
                if let Some(ps) = &peer_screen {
                    *ps.lock().unwrap() = screen;
                }
                record_peer_screen(&name, screen);
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
    server_border: Edge,
    offset: i32,
    server_screen: (u32, u32),
    peer_screen: Arc<Mutex<(u32, u32)>>,
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
                // Client's cursor crossed back over the border → reclaim, and
                // map its exit pixel through the offset so our cursor re-appears
                // at the matching spot.
                InputMsg::Leave { pos } => {
                    let sdim = perp_dim(server_border, server_screen.0, server_screen.1);
                    let server_perp = map_to_server(pos, offset, sdim);
                    *control.return_to.lock().unwrap() = Some((server_border, server_perp));
                    control.active.store(false, Ordering::Relaxed);
                    tracing::info!(pos, server_perp, "client returned control");
                }
                _ => {}
            }
        }

        // Announce control transitions to the client.
        let active = control.active.load(Ordering::Relaxed);
        if active != prev_active {
            if let Some(p) = peer {
                // Clear any modifiers held across the switch (anti-stuck-key).
                let _ = udp.send_to(release_all_modifiers(), p);
                if active {
                    // Only ask the client to track a return border when control
                    // was handed over by an actual edge crossing. Manual toggles
                    // (both-Shift / F12) send nothing — the user toggles back.
                    if let Some((edge, server_perp)) = *control.entry.lock().unwrap() {
                        // Apply the arrangement offset here so the client stays
                        // dumb (it just warps to the pixel we send). Read the
                        // peer's screen LIVE so a resolution change is honoured.
                        let client_screen = *peer_screen.lock().unwrap();
                        let cdim = perp_dim(edge, client_screen.0, client_screen.1);
                        let pos = map_to_client(server_perp, offset, cdim);
                        // The span (in client coords) where the client may cross
                        // back — the overlap of the two screens along the edge.
                        let sdim = perp_dim(edge, server_screen.0, server_screen.1) as i32;
                        let span = client_return_span(offset, sdim, cdim as i32);
                        let _ = udp.send_to(
                            InputMsg::Enter { edge: opposite(edge), pos, span },
                            p,
                        );
                    }
                } else {
                    let _ = udp.send_to(InputMsg::Leave { pos: 0 }, p);
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
    // Our single bordered edge (where the client sits) — used to re-place our
    // cursor at the matching spot when control returns.
    let server_border = cfg
        .machine(&cfg.name)
        .and_then(|m| {
            if m.right.is_some() {
                Some(Edge::Right)
            } else if m.left.is_some() {
                Some(Edge::Left)
            } else if m.top.is_some() {
                Some(Edge::Top)
            } else if m.bottom.is_some() {
                Some(Edge::Bottom)
            } else {
                None
            }
        })
        .unwrap_or(Edge::Right);
    // Arrangement offset + the client's recorded screen size, for seamless,
    // offset-aware edge crossings.
    let offset = cfg.offset;
    // The peer's screen is learned dynamically (Hello) and kept LIVE here, so a
    // resolution change or a stale config never breaks the offset mapping. The
    // config value is only a first-guess until the client says hello.
    let peer_screen = Arc::new(Mutex::new(
        cfg.machines
            .iter()
            .find(|m| m.name != cfg.name)
            .and_then(|m| m.screen)
            .unwrap_or((1920, 1080)),
    ));

    // Capture runs once, globally, feeding a channel. Control starts local.
    let (tx, rx) = mpsc::channel();
    let control = Arc::new(Control::new());
    let control_cap = control.clone();
    let peer_cap = peer_screen.clone();
    std::thread::spawn(move || {
        if let Err(e) = capture::run(tx, control_cap, edges, (sw, sh), offset, peer_cap) {
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

        // Bulk channel (clipboard/files) in the background; it also receives the
        // client's Hello and updates the live peer-screen size.
        let ps_bulk = peer_screen.clone();
        std::thread::spawn(move || {
            let _ = serve_bulk(conn, None, Some(ps_bulk));
        });

        // Encrypted UDP input channel for this session.
        let udp = InputChannel::bind(bind_addr, None)?.with_cipher(Arc::new(input_sess));
        udp.set_read_timeout(Some(Duration::from_millis(1)))?;
        if let Err(e) = run_server_input(
            &udp,
            &rx,
            &control,
            server_border,
            offset,
            (sw, sh),
            peer_screen.clone(),
        ) {
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

    // Bulk channel (clipboard/files). Announce our screen size to the server so
    // its settings window can show our real resolution.
    let (cw, ch) = screen_size(&cfg);
    let hello = BulkMsg::Hello {
        version: shareclick_protocol::PROTOCOL_VERSION,
        name: cfg.name.clone(),
        screen: (cw, ch),
    };
    std::thread::spawn(move || {
        if let Err(e) = serve_bulk(conn, Some(hello), None) {
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
                InputMsg::Enter { edge, pos, span } => {
                    tracker.enter(edge, pos, span);
                    controlling = true;
                    // Warp the real cursor to the exact spot the server sent
                    // (already offset-adjusted), so it appears where it crossed.
                    let (ex, ey) = entry_point(edge, pos, cw, ch);
                    let _ = injector.move_to(ex, ey);
                    tracing::info!(?edge, ex, ey, "gained control from server");
                }
                InputMsg::Leave { .. } => {
                    tracker.leave();
                    controlling = false;
                    tracing::info!("server revoked control");
                }
                InputMsg::Events(events) => {
                    for ev in events {
                        if let InputEvent::MouseMove { dx, dy } = ev {
                            if controlling {
                                if let Some(perp) = tracker.moved(dx, dy) {
                                    // Cursor crossed back → return control, telling
                                    // the server the exit pixel so its cursor
                                    // re-appears at the matching spot.
                                    controlling = false;
                                    let _ = channel.send(InputMsg::Leave { pos: perp });
                                    tracing::info!(perp, "cursor hit border; returning control");
                                }
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
