//! Server-side input capture using `rdev`.
//!
//! `rdev::listen` runs a blocking global event loop (needs Accessibility on
//! macOS). We translate native events into portable [`InputEvent`]s and push
//! them onto a channel for the network sender to consume.
//!
//! rdev reports **absolute** cursor positions; we convert to relative deltas so
//! the client's cursor tracks motion without coupling to screen geometry.

#![cfg(feature = "native")]

use std::sync::mpsc::Sender;

use shareclick_protocol::{InputEvent, MouseButton};

use crate::keymap;

/// Start capturing in the current thread. Blocks until the listener errors.
/// Each captured [`InputEvent`] is sent on `tx`.
pub fn run(tx: Sender<InputEvent>) -> anyhow::Result<()> {
    let mut last_pos: Option<(f64, f64)> = None;

    let callback = move |event: rdev::Event| {
        let mapped = match event.event_type {
            rdev::EventType::MouseMove { x, y } => {
                let ev = match last_pos {
                    Some((px, py)) => Some(InputEvent::MouseMove {
                        dx: (x - px).round() as i32,
                        dy: (y - py).round() as i32,
                    }),
                    None => None, // first sample only establishes the origin
                };
                last_pos = Some((x, y));
                ev
            }
            rdev::EventType::ButtonPress(b) => Some(InputEvent::MouseButton {
                button: to_button(b),
                pressed: true,
            }),
            rdev::EventType::ButtonRelease(b) => Some(InputEvent::MouseButton {
                button: to_button(b),
                pressed: false,
            }),
            rdev::EventType::Wheel { delta_x, delta_y } => Some(InputEvent::Scroll {
                dx: delta_x as f32,
                dy: delta_y as f32,
            }),
            rdev::EventType::KeyPress(k) => Some(InputEvent::Key {
                key: keymap::from_rdev(k),
                pressed: true,
            }),
            rdev::EventType::KeyRelease(k) => Some(InputEvent::Key {
                key: keymap::from_rdev(k),
                pressed: false,
            }),
        };
        if let Some(ev) = mapped {
            let _ = tx.send(ev);
        }
    };

    rdev::listen(callback).map_err(|e| anyhow::anyhow!("input capture failed: {e:?}"))?;
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
