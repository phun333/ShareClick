//! Shared control state between the capture thread and the server input pump.
//!
//! `active` = "the client currently holds the keyboard & mouse". When capture
//! flips it (edge cross or F12) it also records where the cursor left from, so
//! the input pump can tell the client where to enter.

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use shareclick_protocol::Edge;

/// Shared, thread-safe control state.
pub struct Control {
    pub active: AtomicBool,
    /// Edge + normalized (0..1) position the cursor last left this screen from.
    pub entry: Mutex<(Edge, f32)>,
}

impl Control {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            entry: Mutex::new((Edge::Left, 0.5)),
        }
    }
}

impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}
