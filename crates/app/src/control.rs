//! Shared control state between the capture thread and the network pump.
//!
//! SYMMETRIC (ShareMouse-style) model: both machines always capture their own
//! input AND inject the peer's. Exactly one pointer is "away" at a time:
//!
//!  * `my_away`   — MY pointer crossed onto the peer's screen: my physical
//!    input is suppressed locally and forwarded; my cursor is hidden/parked.
//!  * `peer_away` — the PEER's pointer is on MY screen: their forwarded input
//!    is injected here and drives my real cursor. My own physical input still
//!    works locally (local-first, inputs merge like ShareMouse).
//!
//! Capture flips these on edge hits / hotkeys; the network pump diff-detects
//! and sends the matching `PointerEnter` / `PointerEnd` messages.

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use shareclick_protocol::Edge;

/// Shared, thread-safe control state.
pub struct Control {
    /// My pointer is on the peer's screen (forward my input, hide my cursor).
    pub my_away: AtomicBool,
    /// The peer's pointer is on my screen (their input is injected here).
    pub peer_away: AtomicBool,
    /// How my pointer last went away:
    ///  * `Some((edge, perp))` — crossed a screen edge at that perpendicular px.
    ///  * `None` — manual hotkey toggle (enter at the peer's centre).
    pub entry: Mutex<Option<(Edge, i32)>>,
    /// Where my cursor re-appears when my pointer comes home: border edge +
    /// perpendicular pixel. `None` = stay where it is / centre.
    pub return_to: Mutex<Option<(Edge, i32)>>,
    /// Set by capture when the VISITING pointer (peer's) crossed back home at
    /// this my-local perpendicular pixel; the pump maps + sends `PointerEnd`.
    pub send_peer_home: Mutex<Option<i32>>,
    /// While `peer_away`: the edge the visitor entered through + the span along
    /// it where crossing back is allowed (from its `PointerEnter`).
    pub host_span: Mutex<Option<(Edge, (i32, i32))>>,
}

impl Control {
    pub fn new() -> Self {
        Self {
            my_away: AtomicBool::new(false),
            peer_away: AtomicBool::new(false),
            entry: Mutex::new(None),
            return_to: Mutex::new(None),
            send_peer_home: Mutex::new(None),
            host_span: Mutex::new(None),
        }
    }
}

impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}
