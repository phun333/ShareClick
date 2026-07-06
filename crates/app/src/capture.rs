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

// On macOS, rdev's grab does not suppress mouse-move events, so while the client
// has control we "freeze" the local cursor by warping it back to an anchor after
// every move and forward the physical delta. Re-associating immediately clears
// the ~250 ms event-suppression deadzone that a bare warp would otherwise cause.
#[cfg(target_os = "macos")]
#[repr(C)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
    fn CGAssociateMouseAndMouseCursorPosition(connected: bool) -> i32;
}
#[cfg(target_os = "macos")]
fn warp_cursor(x: f64, y: f64) {
    unsafe {
        CGWarpMouseCursorPosition(CGPoint { x, y });
        CGAssociateMouseAndMouseCursorPosition(true);
    }
}

/// Start capturing in the current thread. Blocks until the grab loop errors.
/// Captured [`InputEvent`]s are sent on `tx` only while `active` is set.
///
/// `edges` enables automatic hand-off: when control is local and the cursor
/// reaches a bordered screen edge, control flips to the client automatically.
pub fn run(tx: Sender<InputEvent>, control: Arc<Control>, edges: EdgeConfig) -> anyhow::Result<()> {
    // grab's callback is `Fn` (not `FnMut`) so mutable state lives behind locks.
    let last_pos: Mutex<Option<(f64, f64)>> = Mutex::new(None);
    // macOS cursor-freeze anchor (see the warp helper above).
    #[cfg(target_os = "macos")]
    let anchor: Mutex<Option<(f64, f64)>> = Mutex::new(None);
    // Reliable escape hotkey: BOTH Shift keys held together toggles control.
    // Works on every keyboard and (unlike F12 on macOS, which is a media key)
    // is captured reliably by rdev. This is the "get me unstuck" combo.
    let lshift = std::sync::atomic::AtomicBool::new(false);
    let rshift = std::sync::atomic::AtomicBool::new(false);
    let combo_done = std::sync::atomic::AtomicBool::new(false);

    let toggle = move |control: &Control| {
        let now = !control.active.load(Ordering::Relaxed);
        if now {
            // Manual toggle: no edge, so the client does NOT auto-return.
            *control.entry.lock().unwrap() = None;
        }
        control.active.store(now, Ordering::Relaxed);
        tracing::info!(active = now, "control toggled (hotkey)");
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

        // Automatic edge hand-off: while control is local, a cursor touching a
        // bordered edge switches control to the client. Record where it left so
        // the client can enter at the matching spot.
        if !control.active.load(Ordering::Relaxed) {
            if let EventType::MouseMove { x, y } = event.event_type {
                let (xi, yi) = (x.round() as i32, y.round() as i32);
                if let Some(edge) = edges.hit(xi, yi) {
                    let frac = match edge {
                        Edge::Left | Edge::Right => y as f32 / edges.height.max(1) as f32,
                        Edge::Top | Edge::Bottom => x as f32 / edges.width.max(1) as f32,
                    };
                    *control.entry.lock().unwrap() = Some((edge, frac));
                    control.active.store(true, Ordering::Relaxed);
                    tracing::info!(?edge, "cursor crossed edge; control handed to client");
                }
            }
        }

        let is_active = control.active.load(Ordering::Relaxed);

        // Reset the freeze anchor whenever control is local again.
        #[cfg(target_os = "macos")]
        if !is_active {
            *anchor.lock().unwrap() = None;
        }

        let mapped = match event.event_type {
            EventType::MouseMove { x, y } => {
                #[cfg(target_os = "macos")]
                let ev = if is_active {
                    // Capture mode: freeze the local cursor, forward physical delta.
                    let mut a = anchor.lock().unwrap();
                    match *a {
                        Some((ax, ay)) => {
                            let (dx, dy) = ((x - ax).round() as i32, (y - ay).round() as i32);
                            if dx != 0 || dy != 0 {
                                warp_cursor(ax, ay);
                                Some(InputEvent::MouseMove { dx, dy })
                            } else {
                                None
                            }
                        }
                        None => {
                            *a = Some((x, y));
                            None
                        }
                    }
                } else {
                    let mut lp = last_pos.lock().unwrap();
                    let e = (*lp).map(|(px, py)| InputEvent::MouseMove {
                        dx: (x - px).round() as i32,
                        dy: (y - py).round() as i32,
                    });
                    *lp = Some((x, y));
                    e
                };
                #[cfg(not(target_os = "macos"))]
                let ev = {
                    // Other platforms: grab suppresses locally, so a plain
                    // per-event delta is correct.
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
            None // swallow locally: control belongs to the remote client
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
