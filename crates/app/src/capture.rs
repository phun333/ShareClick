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

/// Start capturing in the current thread. Blocks until the grab loop errors.
/// Captured [`InputEvent`]s are sent on `tx` only while `active` is set.
///
/// `edges` enables automatic hand-off: when control is local and the cursor
/// reaches a bordered screen edge, control flips to the client automatically.
pub fn run(tx: Sender<InputEvent>, control: Arc<Control>, edges: EdgeConfig) -> anyhow::Result<()> {
    // grab's callback is `Fn` (not `FnMut`) so mutable state lives behind locks.
    let last_pos: Mutex<Option<(f64, f64)>> = Mutex::new(None);

    let callback = move |event: Event| -> Option<Event> {
        // Toggle hotkey: always consumed, never forwarded.
        if let EventType::KeyPress(k) = event.event_type {
            if k == TOGGLE_KEY {
                let now = !control.active.load(Ordering::Relaxed);
                if now {
                    *control.entry.lock().unwrap() = (Edge::Left, 0.5);
                }
                control.active.store(now, Ordering::Relaxed);
                tracing::info!(active = now, "control toggled");
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
                    *control.entry.lock().unwrap() = (edge, frac);
                    control.active.store(true, Ordering::Relaxed);
                    tracing::info!(?edge, "cursor crossed edge; control handed to client");
                }
            }
        }

        let is_active = control.active.load(Ordering::Relaxed);

        let mapped = match event.event_type {
            EventType::MouseMove { x, y } => {
                let mut lp = last_pos.lock().unwrap();
                let ev = match *lp {
                    Some((px, py)) => Some(InputEvent::MouseMove {
                        dx: (x - px).round() as i32,
                        dy: (y - py).round() as i32,
                    }),
                    None => None,
                };
                *lp = Some((x, y));
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
