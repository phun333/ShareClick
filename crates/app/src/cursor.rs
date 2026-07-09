//! Client-side cursor tracking for automatic control return.
//!
//! While the client holds control, the server sends only *relative* motion, so
//! the client integrates those deltas to know where its cursor is. When the
//! cursor travels back across the edge it entered from (the border shared with
//! the server), the client returns control automatically — the seamless
//! counterpart to the server's edge hand-off.

use shareclick_protocol::Edge;

/// Pixels the cursor must move inward before a return is armed. Prevents an
/// instant bounce-back right after entering at the border. Deliberately large
/// so normal cursor use doesn't accidentally hand control back.
const ARM_MARGIN: i32 = 60;

/// Integrates relative motion and detects a return across the border edge.
#[derive(Debug, Clone)]
pub struct CursorTracker {
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    border: Option<Edge>,
    armed: bool,
}

impl CursorTracker {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            w: width.max(1) as i32,
            h: height.max(1) as i32,
            x: 0,
            y: 0,
            border: None,
            armed: false,
        }
    }

    /// The client gained control, entering from `edge` at perpendicular pixel
    /// `perp` (vertical for left/right, horizontal for top/bottom).
    pub fn enter(&mut self, edge: Edge, perp: i32) {
        let py = perp.clamp(0, self.h - 1);
        let px = perp.clamp(0, self.w - 1);
        let (x, y) = match edge {
            Edge::Left => (0, py),
            Edge::Right => (self.w - 1, py),
            Edge::Top => (px, 0),
            Edge::Bottom => (px, self.h - 1),
        };
        self.x = x;
        self.y = y;
        self.border = Some(edge);
        self.armed = false;
    }

    /// The client lost control (server revoked it).
    pub fn leave(&mut self) {
        self.border = None;
        self.armed = false;
    }

    /// Perpendicular pixel along the border edge (vertical for left/right,
    /// horizontal for top/bottom). Told to the server so it re-enters there.
    pub fn exit_perp(&self) -> i32 {
        match self.border {
            Some(Edge::Left) | Some(Edge::Right) => self.y,
            _ => self.x,
        }
    }

    /// Apply a relative move. Returns `Some(perp)` when the cursor has crossed
    /// back over the border edge (control should return to the server), where
    /// `perp` is the perpendicular pixel along that edge; `None` otherwise.
    pub fn moved(&mut self, dx: i32, dy: i32) -> Option<i32> {
        let nx = self.x + dx;
        let ny = self.y + dy;
        let border = match self.border {
            Some(b) => b,
            None => {
                self.store(nx, ny);
                return None;
            }
        };

        let returned = match border {
            Edge::Left => {
                if nx > ARM_MARGIN {
                    self.armed = true;
                }
                self.armed && nx <= 0
            }
            Edge::Right => {
                if nx < self.w - 1 - ARM_MARGIN {
                    self.armed = true;
                }
                self.armed && nx >= self.w - 1
            }
            Edge::Top => {
                if ny > ARM_MARGIN {
                    self.armed = true;
                }
                self.armed && ny <= 0
            }
            Edge::Bottom => {
                if ny < self.h - 1 - ARM_MARGIN {
                    self.armed = true;
                }
                self.armed && ny >= self.h - 1
            }
        };

        self.store(nx, ny);
        if returned {
            let perp = self.exit_perp();
            self.border = None;
            self.armed = false;
            Some(perp)
        } else {
            None
        }
    }

    fn store(&mut self, nx: i32, ny: i32) {
        self.x = nx.clamp(0, self.w - 1);
        self.y = ny.clamp(0, self.h - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_after_moving_in_then_back_out_left() {
        let mut t = CursorTracker::new(1920, 1080);
        t.enter(Edge::Left, 540);
        assert!(t.moved(100, 0).is_none()); // move inward → arms
        assert!(t.moved(-50, 0).is_none()); // partway back
        assert!(t.moved(-100, 0).is_some()); // cross the left border → return
    }

    #[test]
    fn does_not_return_before_arming() {
        let mut t = CursorTracker::new(1920, 1080);
        t.enter(Edge::Left, 540);
        // Immediately shoving left (never moved inward) must not bounce back.
        assert!(t.moved(-50, 0).is_none());
    }

    #[test]
    fn returns_across_right_border() {
        let mut t = CursorTracker::new(1000, 800);
        t.enter(Edge::Right, 400);
        assert!(t.moved(-200, 0).is_none()); // inward (left) arms
        assert!(t.moved(300, 0).is_some()); // back out the right edge
    }

    #[test]
    fn no_border_never_returns() {
        let mut t = CursorTracker::new(800, 600);
        assert!(t.moved(-1000, 0).is_none());
        assert!(t.moved(1000, 0).is_none());
    }
}
