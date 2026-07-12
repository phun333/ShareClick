//! Server-side input capture using `rdev`'s global grab.
//!
//! Unlike a passive listener, `grab` lets us **consume** events so the local
//! machine does not react while control has been handed to the remote client —
//! this is what turns ShareClick from an input *mirror* into a real KVM.
//!
//! A toggle hotkey ([`TOGGLE_KEY`]) flips the shared `active` flag:
//!  * **active**   → events are forwarded to the client and swallowed locally.
//!  * **inactive** → events pass straight through to this machine, nothing sent.
//!
//! rdev reports **absolute** cursor positions; we convert to relative deltas so
//! the client's cursor tracks motion without coupling to screen geometry.

#![cfg(feature = "native")]

use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use rdev::{Event, EventType};
use shareclick_protocol::{Edge, InputEvent, MouseButton};

use crate::control::Control;
use crate::edge::EdgeConfig;
use crate::keymap;

/// Hotkey that toggles whether control is on the remote client.
pub const TOGGLE_KEY: rdev::Key = rdev::Key::F12;

// macOS cursor capture (the Deskflow technique). Lets the local cursor "leave"
// this screen (extend, not mirror) while the client has control:
//  * hide it from a background app via the private `SetsCursorInBackground`,
//  * keep warping it to the screen centre so it never hits an edge,
//  * zero the local-events suppression interval so warps don't lag.
// See references/macos-cursor-capture.md.
#[cfg(target_os = "macos")]
mod mac_cursor {
    use std::os::raw::{c_char, c_void};

    #[repr(C)]
    pub struct CGPoint {
        pub x: f64,
        pub y: f64,
    }
    type CFTypeRef = *const c_void;
    type CFStringRef = *const c_void;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
        fn CGEventCreate(source: *const c_void) -> *const c_void;
        fn CGEventGetLocation(event: *const c_void) -> CGPoint;
        fn CGDisplayHideCursor(display: u32) -> i32;
        fn CGDisplayShowCursor(display: u32) -> i32;
        fn CGAssociateMouseAndMouseCursorPosition(connected: bool) -> i32;
        fn CGSetLocalEventsSuppressionInterval(seconds: f64) -> i32;
        fn _CGSDefaultConnection() -> i32;
        fn CGSSetConnectionProperty(cid: i32, target: i32, key: CFStringRef, value: CFTypeRef) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithCString(alloc: *const c_void, s: *const c_char, enc: u32) -> CFStringRef;
        fn CFRelease(cf: CFTypeRef);
        static kCFBooleanTrue: CFTypeRef;
    }

    pub fn zero_suppression() {
        unsafe {
            CGSetLocalEventsSuppressionInterval(0.0);
        }
    }
    pub fn warp_to(x: f64, y: f64) {
        unsafe {
            CGWarpMouseCursorPosition(CGPoint { x, y });
        }
    }
    /// The ACTUAL current cursor position (not a stale event location). This is
    /// what makes the warp-to-centre delta scheme stable (Deskflow does the same).
    pub fn current_pos() -> (f64, f64) {
        unsafe {
            let e = CGEventCreate(std::ptr::null());
            let p = CGEventGetLocation(e);
            if !e.is_null() {
                CFRelease(e);
            }
            (p.x, p.y)
        }
    }
    fn set_bg_cursor_property() {
        unsafe {
            let key = CFStringCreateWithCString(
                std::ptr::null(),
                b"SetsCursorInBackground\0".as_ptr() as *const c_char,
                0, // kCFStringEncodingMacRoman
            );
            if !key.is_null() {
                let conn = _CGSDefaultConnection();
                CGSSetConnectionProperty(conn, conn, key, kCFBooleanTrue);
                CFRelease(key);
            }
        }
    }
    pub fn hide_cursor() {
        set_bg_cursor_property();
        unsafe {
            CGDisplayHideCursor(CGMainDisplayID());
            CGAssociateMouseAndMouseCursorPosition(true); // visibility bug fix
        }
    }
    pub fn show_cursor() {
        set_bg_cursor_property();
        unsafe {
            CGDisplayShowCursor(CGMainDisplayID());
            CGAssociateMouseAndMouseCursorPosition(true);
        }
    }
}

/// Start capturing in the current thread. Blocks until the grab loop errors.
/// Captured [`InputEvent`]s are sent on `tx` only while `active` is set.
///
/// `edges` enables automatic hand-off: when control is local and the cursor
/// reaches a bordered screen edge, control flips to the client automatically.
pub fn run(
    tx: Sender<InputEvent>,
    control: Arc<Control>,
    arrangement: Arc<Mutex<(EdgeConfig, i32)>>,
    screen: (u32, u32),
    peer_screen: Arc<Mutex<(u32, u32)>>,
) -> anyhow::Result<()> {
    #[cfg(not(target_os = "macos"))]
    let _ = screen;
    // grab's callback is `Fn` (not `FnMut`) so mutable state lives behind locks.
    let last_pos: Mutex<Option<(f64, f64)>> = Mutex::new(None);
    // macOS: screen centre we warp the hidden cursor back to, and a transition
    // tracker so we (un)hide the cursor only when control changes.
    #[cfg(target_os = "macos")]
    let (cx, cy) = (screen.0 as f64 / 2.0, screen.1 as f64 / 2.0);
    #[cfg(target_os = "macos")]
    let was_active = std::sync::atomic::AtomicBool::new(false);
    #[cfg(target_os = "macos")]
    mac_cursor::zero_suppression();
    // Reliable escape hotkey: BOTH Shift keys held together toggles control.
    // Works on every keyboard and (unlike F12 on macOS, which is a media key)
    // is captured reliably by rdev. This is the "get me unstuck" combo.
    let lshift = std::sync::atomic::AtomicBool::new(false);
    let rshift = std::sync::atomic::AtomicBool::new(false);
    let combo_done = std::sync::atomic::AtomicBool::new(false);

    let toggle = move |control: &Control| {
        if control.my_away.load(Ordering::Relaxed) {
            // Reclaim my pointer (the pump tells the peer with PointerEnd None).
            control.my_away.store(false, Ordering::Relaxed);
            *control.return_to.lock().unwrap() = None;
            tracing::info!("pointer reclaimed (hotkey)");
        } else if control.peer_away.load(Ordering::Relaxed) {
            // Send the visiting pointer home (centre-ish).
            control.peer_away.store(false, Ordering::Relaxed);
            *control.send_peer_home.lock().unwrap() = Some(i32::MAX); // MAX = centre
            tracing::info!("visiting pointer sent home (hotkey)");
        } else {
            // Push my pointer to the peer (no edge → enter at their centre).
            *control.entry.lock().unwrap() = None;
            control.my_away.store(true, Ordering::Relaxed);
            tracing::info!("pointer pushed to peer (hotkey)");
        }
    };

    let callback = move |event: Event| -> Option<Event> {
        // --- Reliable both-Shift escape combo (does not swallow the keys, so
        //     capitals still work when typing on the remote). ---
        match event.event_type {
            EventType::KeyPress(rdev::Key::ShiftLeft) => lshift.store(true, Ordering::Relaxed),
            EventType::KeyRelease(rdev::Key::ShiftLeft) => {
                lshift.store(false, Ordering::Relaxed);
                combo_done.store(false, Ordering::Relaxed);
            }
            EventType::KeyPress(rdev::Key::ShiftRight) => rshift.store(true, Ordering::Relaxed),
            EventType::KeyRelease(rdev::Key::ShiftRight) => {
                rshift.store(false, Ordering::Relaxed);
                combo_done.store(false, Ordering::Relaxed);
            }
            _ => {}
        }
        if lshift.load(Ordering::Relaxed)
            && rshift.load(Ordering::Relaxed)
            && !combo_done.swap(true, Ordering::Relaxed)
        {
            toggle(&control);
        }

        // F12 also toggles (works well on Windows where it's a real F-key).
        if let EventType::KeyPress(k) = event.event_type {
            if k == TOGGLE_KEY {
                toggle(&control);
                return None;
            }
        }

        // SYMMETRIC edge logic. The real cursor on this screen may be driven by
        // local hands OR by the peer's injected input; either way, when it hits
        // the shared border inside the overlap:
        //  * a VISITING pointer (peer_away) crosses back home;
        //  * otherwise MY pointer goes away to the peer.
        if !control.my_away.load(Ordering::Relaxed) {
            if let EventType::MouseMove { x, y } = event.event_type {
                let (xi, yi) = (x.round() as i32, y.round() as i32);
                // Arm the visitor's return only after it moved IN from the entry
                // edge — otherwise it would bounce straight back (border jitter).
                if control.peer_away.load(Ordering::Relaxed)
                    && !control.host_armed.load(Ordering::Relaxed)
                {
                    if let Some((e, _)) = *control.host_span.lock().unwrap() {
                        const ARM: i32 = 60;
                        let armed = match e {
                            Edge::Left => xi > ARM,
                            Edge::Right => xi < screen.0 as i32 - 1 - ARM,
                            Edge::Top => yi > ARM,
                            Edge::Bottom => yi < screen.1 as i32 - 1 - ARM,
                        };
                        if armed {
                            control.host_armed.store(true, Ordering::Relaxed);
                        }
                    }
                }
                // Arrangement is LIVE: it may arrive/change via the peer's Hello.
                let (edges, offset) = *arrangement.lock().unwrap();
                if let Some(edge) = edges.hit(xi, yi) {
                    // Position along the edge (this-screen coordinates).
                    let perp = match edge {
                        Edge::Left | Edge::Right => yi,
                        Edge::Top | Edge::Bottom => xi,
                    };
                    if control.peer_away.load(Ordering::Relaxed) {
                        // The visitor leaves through the edge it entered, within
                        // the span its PointerEnter allowed — and only once armed.
                        let ok = control.host_armed.load(Ordering::Relaxed)
                            && control.host_span.lock().unwrap().map_or(false, |(e, span)| {
                                e == edge && crate::edge::in_span(perp, span)
                            });
                        if ok {
                            control.peer_away.store(false, Ordering::Relaxed);
                            *control.send_peer_home.lock().unwrap() = Some(perp);
                            tracing::info!(?edge, perp, "visiting pointer crossed home");
                        }
                    } else {
                        // My pointer leaves — only where the two screens overlap.
                        let peer = *peer_screen.lock().unwrap();
                        let (this_dim, other_dim) = match edge {
                            Edge::Left | Edge::Right => (screen.1 as i32, peer.1 as i32),
                            Edge::Top | Edge::Bottom => (screen.0 as i32, peer.0 as i32),
                        };
                        let span = crate::edge::overlap_span(this_dim, offset, other_dim);
                        if crate::edge::in_span(perp, span) {
                            *control.entry.lock().unwrap() = Some((edge, perp));
                            control.my_away.store(true, Ordering::Relaxed);
                            tracing::info!(?edge, perp, "my pointer crossed to the peer");
                        }
                    }
                }
            }
        }

        let is_active = control.my_away.load(Ordering::Relaxed);

        // macOS: on control changes, hide/show the local cursor and recentre it.
        #[cfg(target_os = "macos")]
        {
            let was = was_active.swap(is_active, Ordering::Relaxed);
            if is_active && !was {
                mac_cursor::hide_cursor();
                mac_cursor::warp_to(cx, cy);
                *last_pos.lock().unwrap() = Some((cx, cy));
            } else if !is_active && was {
                mac_cursor::show_cursor();
                // Re-place our cursor where the client crossed back, so it
                // returns at the matching spot (not the centre).
                let (wx, wy) = match control.return_to.lock().unwrap().take() {
                    Some((edge, perp)) => {
                        let (px, py) = crate::edge::entry_point(edge, perp, screen.0, screen.1);
                        (px as f64, py as f64)
                    }
                    None => (cx, cy),
                };
                mac_cursor::warp_to(wx, wy);
                *last_pos.lock().unwrap() = None;
            }
        }

        let mapped = match event.event_type {
            EventType::MouseMove { x, y } => {
                #[cfg(target_os = "macos")]
                let ev = {
                    let mut lp = last_pos.lock().unwrap();
                    if is_active {
                        let _ = (x, y); // event position is stale; we query live
                        // Deskflow technique: read the LIVE cursor position, warp
                        // it back to centre every move, and forward the delta.
                        // The warp only takes effect because we RETURN the
                        // mouse-move event below (never suppress it on macOS).
                        let (mx, my) = mac_cursor::current_pos();
                        let (px, py) = (*lp).unwrap_or((mx, my));
                        let dx = (mx - px).round() as i32;
                        let dy = (my - py).round() as i32;
                        // Skip no-motion and the post-warp "already at centre" event.
                        if (dx == 0 && dy == 0)
                            || ((mx - cx).abs() < 1.0 && (my - cy).abs() < 1.0)
                        {
                            *lp = Some((mx, my));
                            None
                        } else {
                            mac_cursor::warp_to(cx, cy);
                            *lp = Some((cx, cy));
                            // Drop warp-artifact motions (~ centre-to-edge).
                            if (dx.abs() as f64) > cx - 10.0 || (dy.abs() as f64) > cy - 10.0 {
                                None
                            } else {
                                Some(InputEvent::MouseMove { dx, dy })
                            }
                        }
                    } else {
                        let e = (*lp).map(|(px, py)| InputEvent::MouseMove {
                            dx: (x - px).round() as i32,
                            dy: (y - py).round() as i32,
                        });
                        *lp = Some((x, y));
                        e
                    }
                };
                #[cfg(not(target_os = "macos"))]
                let ev = {
                    let mut lp = last_pos.lock().unwrap();
                    let e = (*lp).map(|(px, py)| InputEvent::MouseMove {
                        dx: (x - px).round() as i32,
                        dy: (y - py).round() as i32,
                    });
                    *lp = Some((x, y));
                    e
                };
                ev
            }
            EventType::ButtonPress(b) => Some(InputEvent::MouseButton {
                button: to_button(b),
                pressed: true,
            }),
            EventType::ButtonRelease(b) => Some(InputEvent::MouseButton {
                button: to_button(b),
                pressed: false,
            }),
            EventType::Wheel { delta_x, delta_y } => Some(InputEvent::Scroll {
                dx: delta_x as f32,
                dy: delta_y as f32,
            }),
            EventType::KeyPress(k) => Some(InputEvent::Key {
                key: keymap::from_rdev(k),
                pressed: true,
            }),
            EventType::KeyRelease(k) => Some(InputEvent::Key {
                key: keymap::from_rdev(k),
                pressed: false,
            }),
        };

        if is_active {
            if let Some(ev) = mapped {
                let _ = tx.send(ev);
            }
            // macOS: mouse-move MUST pass through so the re-centre warp is
            // honoured by the window server (Deskflow does the same); everything
            // else (clicks, keys, scroll) is suppressed locally. On other
            // platforms rdev suppresses cleanly, so we drop everything.
            #[cfg(target_os = "macos")]
            {
                if matches!(event.event_type, EventType::MouseMove { .. }) {
                    Some(event)
                } else {
                    None
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                None
            }
        } else {
            Some(event) // let this machine handle it normally
        }
    };

    rdev::grab(callback).map_err(|e| anyhow::anyhow!("input capture failed: {e:?}"))?;
    Ok(())
}

fn to_button(b: rdev::Button) -> MouseButton {
    match b {
        rdev::Button::Left => MouseButton::Left,
        rdev::Button::Right => MouseButton::Right,
        rdev::Button::Middle => MouseButton::Middle,
        rdev::Button::Unknown(n) => MouseButton::Other(n),
    }
}
