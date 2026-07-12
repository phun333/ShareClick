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
use crate::edge::{client_return_span, entry_point, map_to_client, perp_dim, EdgeConfig};
use crate::filexfer::FileReceiver;
use crate::transport::InputChannel;

/// Everything one symmetric peer session shares between the capture thread,
/// the bulk (Hello/clipboard) thread and the input pump. Both machines build
/// exactly the same thing — there is no server/client asymmetry at runtime.
#[derive(Clone)]
struct Shared {
    control: Arc<Control>,
    /// (edge config, arrangement offset) — LIVE: the peer's Hello can install
    /// or update it (configure the layout once, on either machine).
    arrangement: Arc<Mutex<(EdgeConfig, i32)>>,
    /// My bordered edge (where the peer's screen sits). LIVE, like above.
    border: Arc<Mutex<Option<Edge>>>,
    /// The peer's screen size (LIVE, learned from its Hello).
    peer_screen: Arc<Mutex<(u32, u32)>>,
    /// My own screen size.
    screen: (u32, u32),
}

/// Build the shared state from the local config (arrangement may be absent —
/// the peer's Hello can supply it later).
fn build_shared(cfg: &Config) -> Shared {
    let (sw, sh) = screen_size(cfg);
    tracing::info!(width = sw, height = sh, "screen size (auto-detected)");
    let border = cfg.machine(&cfg.name).and_then(|m| {
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
    });
    let edges = match (cfg.machine(&cfg.name), cfg.auto_edge_switch) {
        (Some(m), true) => EdgeConfig::new(
            sw,
            sh,
            m.left.is_some(),
            m.right.is_some(),
            m.top.is_some(),
            m.bottom.is_some(),
        ),
        _ => EdgeConfig::none(),
    };
    let peer_screen = cfg
        .machines
        .iter()
        .find(|m| m.name != cfg.name)
        .and_then(|m| m.screen)
        .unwrap_or((1920, 1080));
    Shared {
        control: Arc::new(Control::new()),
        arrangement: Arc::new(Mutex::new((edges, cfg.offset))),
        border: Arc::new(Mutex::new(border)),
        peer_screen: Arc::new(Mutex::new(peer_screen)),
        screen: (sw, sh),
    }
}

/// My own Hello: name + screen + my arrangement (so a peer with none adopts it).
fn my_hello(cfg: &Config, sh: &Shared) -> BulkMsg {
    BulkMsg::Hello {
        version: shareclick_protocol::PROTOCOL_VERSION,
        name: cfg.name.clone(),
        screen: sh.screen,
        edge: *sh.border.lock().unwrap(),
        offset: sh.arrangement.lock().unwrap().1,
    }
}

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

/// Persist an adopted arrangement: my machine gets the peer on `my_edge`, the
/// peer machine gets the reciprocal, and the offset is stored — so the layout
/// survives restarts even though it was only ever configured on the peer.
fn record_peer_layout(peer: &str, my_edge: Edge, my_offset: i32) {
    let path = Config::default_path();
    let Ok(mut cfg) = Config::load(&path) else { return };
    let me = cfg.name.clone();
    let set = |m: &mut crate::config::Machine, e: Edge, n: &str| {
        m.left = None;
        m.right = None;
        m.top = None;
        m.bottom = None;
        match e {
            Edge::Left => m.left = Some(n.into()),
            Edge::Right => m.right = Some(n.into()),
            Edge::Top => m.top = Some(n.into()),
            Edge::Bottom => m.bottom = Some(n.into()),
        }
    };
    let peer_owned = peer.to_string();
    for m in cfg.machines.iter_mut() {
        if m.name == me {
            set(m, my_edge, &peer_owned);
        } else if m.name == peer_owned {
            set(m, opposite(my_edge), &me);
        }
    }
    cfg.offset = my_offset;
    let _ = cfg.save(&path);
}

/// Wire clipboard + file sync onto one (already-encrypted) bulk connection.
/// Blocks on the reader loop; returns when the peer disconnects.
/// `adopt` = always take the peer's arrangement (the dialer does; the listener
/// only takes it when it has none of its own).
fn serve_bulk(conn: BulkConn, hello: Option<BulkMsg>, sh: Shared, adopt: bool) -> anyhow::Result<()> {
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
            // Peer's Hello: its screen size (kept LIVE for the offset maths,
            // Deskflow's DINF pattern) and optionally its monitor arrangement —
            // adopt the mirrored version so the layout is only ever configured
            // on ONE machine.
            Ok(BulkMsg::Hello { name, screen, edge, offset, .. }) => {
                tracing::info!(peer = %name, width = screen.0, height = screen.1, "peer reported its screen size (Hello)");
                *sh.peer_screen.lock().unwrap() = screen;
                record_peer_screen(&name, screen);
                if let Some(their_edge) = edge {
                    let have_own = sh.border.lock().unwrap().is_some();
                    if adopt || !have_own {
                        let my_edge = opposite(their_edge);
                        let my_offset = -offset;
                        *sh.border.lock().unwrap() = Some(my_edge);
                        *sh.arrangement.lock().unwrap() = (
                            EdgeConfig::new(
                                sh.screen.0,
                                sh.screen.1,
                                my_edge == Edge::Left,
                                my_edge == Edge::Right,
                                my_edge == Edge::Top,
                                my_edge == Edge::Bottom,
                            ),
                            my_offset,
                        );
                        record_peer_layout(&name, my_edge, my_offset);
                        tracing::info!(?my_edge, my_offset, "adopted the peer's monitor arrangement");
                    }
                }
            }
            Ok(_) => {}
            Err(_) => return Ok(()),
        }
    }
}

/// The SYMMETRIC input pump — identical on both machines. Injects the peer's
/// forwarded input, forwards ours while our pointer is away, and translates
/// capture-thread state flips into `PointerEnter` / `PointerEnd` messages.
fn run_peer_input(
    udp: &InputChannel,
    rx: &Receiver<InputEvent>,
    sh: &Shared,
    mut peer: Option<SocketAddr>,
) -> anyhow::Result<()> {
    let control = &sh.control;
    let mut injector = crate::emit::Injector::new()?;
    let mut prev_my_away = false;
    let mut idle_ticks: u32 = 0;
    let mut buf = [0u8; 2048];
    loop {
        // ---- receive from the peer ----
        if let Ok(Some((pkt, from))) = udp.recv(&mut buf) {
            if peer != Some(from) {
                tracing::info!(%from, "peer input channel online");
                peer = Some(from);
            }
            match pkt.msg {
                InputMsg::Ping { nonce, echo_nanos } => {
                    let _ = udp.send_to(InputMsg::Pong { nonce, echo_nanos }, from);
                }
                // The peer's physical input drives MY real cursor.
                InputMsg::Events(events) => {
                    for ev in events {
                        if let Err(e) = injector.apply(ev) {
                            tracing::warn!(error = %e, "inject failed");
                        }
                    }
                }
                // The peer's pointer arrives on my screen.
                InputMsg::PointerEnter { edge, pos, span } => {
                    if control.my_away.swap(false, Ordering::Relaxed) {
                        prev_my_away = false; // crossed paths: mine implicitly came home
                    }
                    control.peer_away.store(true, Ordering::Relaxed);
                    *control.host_span.lock().unwrap() = Some((edge, span));
                    let (ex, ey) = entry_point(edge, pos, sh.screen.0, sh.screen.1);
                    let _ = injector.move_to(ex, ey);
                    tracing::info!(?edge, ex, ey, "peer pointer entered my screen");
                }
                InputMsg::Pong { .. } => {}
                // An away-state ends.
                InputMsg::PointerEnd { pos } => {
                    if control.my_away.swap(false, Ordering::Relaxed) {
                        prev_my_away = false;
                        let border = sh.border.lock().unwrap().unwrap_or(Edge::Right);
                        *control.return_to.lock().unwrap() = pos.map(|p| (border, p));
                        // macOS re-shows + warps in capture; elsewhere warp here.
                        #[cfg(not(target_os = "macos"))]
                        if let Some(p) = pos {
                            let (ex, ey) = entry_point(border, p, sh.screen.0, sh.screen.1);
                            let _ = injector.move_to(ex, ey);
                        }
                        tracing::info!("my pointer came home");
                    } else if control.peer_away.swap(false, Ordering::Relaxed) {
                        *control.host_span.lock().unwrap() = None;
                        tracing::info!("peer reclaimed its pointer");
                    }
                }
            }
        } else {
            // Idle keep-alive so the path stays warm and the peer learns our
            // address (the dialer pings first; NAT/firewall state stays open).
            idle_ticks += 1;
            if idle_ticks > 2000 {
                idle_ticks = 0;
                if let Some(p) = peer {
                    let _ = udp.send_to(InputMsg::Ping { nonce: 0, echo_nanos: 0 }, p);
                }
            }
        }

        // ---- capture: the visiting pointer crossed home ----
        if let Some(perp) = control.send_peer_home.lock().unwrap().take() {
            if let Some(p) = peer {
                let _ = udp.send_to(release_all_modifiers(), p);
                let msg = if perp == i32::MAX {
                    InputMsg::PointerEnd { pos: None } // hotkey: no position
                } else {
                    let (_, offset) = *sh.arrangement.lock().unwrap();
                    let ps = *sh.peer_screen.lock().unwrap();
                    let border = sh.border.lock().unwrap().unwrap_or(Edge::Right);
                    let cdim = perp_dim(border, ps.0, ps.1);
                    InputMsg::PointerEnd { pos: Some(map_to_client(perp, offset, cdim)) }
                };
                let _ = udp.send_to(msg, p);
            }
            *control.host_span.lock().unwrap() = None;
        }

        // ---- capture: my pointer went away / was reclaimed ----
        let my_away = control.my_away.load(Ordering::Relaxed);
        if my_away != prev_my_away {
            if let Some(p) = peer {
                let _ = udp.send_to(release_all_modifiers(), p);
                if my_away {
                    let (_, offset) = *sh.arrangement.lock().unwrap();
                    let ps = *sh.peer_screen.lock().unwrap();
                    let (edge_out, pos, span) = match *control.entry.lock().unwrap() {
                        Some((edge, perp)) => {
                            let cdim = perp_dim(edge, ps.0, ps.1);
                            let sdim = perp_dim(edge, sh.screen.0, sh.screen.1) as i32;
                            (
                                edge,
                                map_to_client(perp, offset, cdim),
                                client_return_span(offset, sdim, cdim as i32),
                            )
                        }
                        None => {
                            // Hotkey push: enter at the peer's centre.
                            let border = sh.border.lock().unwrap().unwrap_or(Edge::Right);
                            let cdim = perp_dim(border, ps.0, ps.1);
                            let sdim = perp_dim(border, sh.screen.0, sh.screen.1) as i32;
                            (
                                border,
                                cdim as i32 / 2,
                                client_return_span(offset, sdim, cdim as i32),
                            )
                        }
                    };
                    let _ = udp.send_to(
                        InputMsg::PointerEnter { edge: opposite(edge_out), pos, span },
                        p,
                    );
                } else {
                    // Hotkey reclaim — tell the peer the visit ended.
                    let _ = udp.send_to(InputMsg::PointerEnd { pos: None }, p);
                }
            }
            prev_my_away = my_away;
        }

        // ---- forward my captured input while my pointer is away ----
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

/// Listener peer: accepts the connection. At runtime both sides are identical
/// (symmetric ShareMouse-style control) — "server" only means "listens".
pub fn serve(bind: &str) -> anyhow::Result<()> {
    let cfg = load_config()?;
    let psk = cfg.psk.clone().into_bytes();
    let bind_addr = resolve(bind)?;
    tracing::info!(%bind_addr, name = %cfg.name, "listening; both machines' mice/keyboards work — push through the shared edge");
    tracing::info!("grant Accessibility permission on macOS for capture to work");

    let sh = build_shared(&cfg);

    // Capture runs once, globally. Both peers capture their own input.
    let (tx, rx) = mpsc::channel();
    {
        let c = sh.control.clone();
        let arr = sh.arrangement.clone();
        let ps = sh.peer_screen.clone();
        let screen = sh.screen;
        std::thread::spawn(move || {
            if let Err(e) = capture::run(tx, c, arr, screen, ps) {
                tracing::error!(error = %e, "capture thread stopped");
            }
        });
    }

    // Advertise over mDNS so peers can find us without an IP.
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
        tracing::info!(%peer_ip, "peer authenticated (encrypted session established)");

        // Bulk channel: clipboard/files + Hello exchange (we send ours too, so
        // the dialer can adopt our arrangement).
        let hello = my_hello(&cfg, &sh);
        let sh_bulk = sh.clone();
        std::thread::spawn(move || {
            // Listener only adopts the peer's layout when it has none itself.
            let _ = serve_bulk(conn, Some(hello), sh_bulk, false);
        });

        // Encrypted UDP input channel for this session.
        let udp = InputChannel::bind(bind_addr, None)?.with_cipher(Arc::new(input_sess));
        udp.set_read_timeout(Some(Duration::from_millis(1)))?;
        if let Err(e) = run_peer_input(&udp, &rx, &sh, None) {
            tracing::warn!(error = %e, "input session ended; awaiting a new peer");
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
    tracing::info!("authenticated with peer (encrypted session established)");

    let sh = build_shared(&cfg);

    // SYMMETRIC: the dialer captures its own input too — both machines' mice
    // and keyboards work, whichever you grab (ShareMouse-style).
    let (tx, rx) = mpsc::channel();
    {
        let c = sh.control.clone();
        let arr = sh.arrangement.clone();
        let ps = sh.peer_screen.clone();
        let screen = sh.screen;
        std::thread::spawn(move || {
            if let Err(e) = capture::run(tx, c, arr, screen, ps) {
                tracing::error!(error = %e, "capture thread stopped");
            }
        });
    }

    // Bulk channel: clipboard/files + Hello exchange. The dialer adopts the
    // listener's arrangement, so you only configure the layout on one machine.
    let hello = my_hello(&cfg, &sh);
    let sh_bulk = sh.clone();
    std::thread::spawn(move || {
        if let Err(e) = serve_bulk(conn, Some(hello), sh_bulk, true) {
            tracing::warn!(error = %e, "bulk channel closed");
        }
    });

    // Encrypted UDP input channel; announce ourselves so the listener learns
    // our address, then run the same symmetric pump as the listener.
    let channel = InputChannel::bind("0.0.0.0:0".parse().unwrap(), Some(server_addr))?
        .with_cipher(Arc::new(input_sess));
    channel.set_read_timeout(Some(Duration::from_millis(1)))?;
    channel.send(InputMsg::Ping { nonce: 0, echo_nanos: 0 })?;

    run_peer_input(&channel, &rx, &sh, Some(server_addr))
}
