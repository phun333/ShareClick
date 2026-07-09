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
    /// How the client last gained control:
    ///  * `Some((edge, perp))` — the cursor crossed a screen edge; `perp` is the
    ///    server-local perpendicular pixel it left at. The client tracks its
    ///    cursor and auto-returns at the matching border.
    ///  * `None` — a manual toggle (both-Shift / F12); no edge tracking.
    pub entry: Mutex<Option<(Edge, i32)>>,
    /// Where to place the server's cursor when control returns: the server's
    /// border edge + the server-local perpendicular pixel. `None` = centre.
    pub return_to: Mutex<Option<(Edge, i32)>>,
}

impl Control {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            entry: Mutex::new(None),
            return_to: Mutex::new(None),
        }
    }
}

impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}
