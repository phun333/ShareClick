//! Screen-edge detection for automatic control hand-off.
//!
//! When the server cursor reaches an edge that has a neighbour in the monitor
//! manager layout, control is handed to that neighbour — the seamless
//! "just push the mouse to the other screen" behaviour, no hotkey needed.

use shareclick_protocol::Edge;

/// Which edges of this machine's screen border another machine, plus the
/// screen size used to test the cursor position.
#[derive(Debug, Clone, Copy)]
pub struct EdgeConfig {
    pub width: i32,
    pub height: i32,
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl EdgeConfig {
    /// Build from a screen size and which edges have neighbours.
    pub fn new(width: u32, height: u32, left: bool, right: bool, top: bool, bottom: bool) -> Self {
        Self {
            width: width as i32,
            height: height as i32,
            left,
            right,
            top,
            bottom,
        }
    }

    /// No neighbours anywhere (auto edge-switching effectively disabled).
    pub fn none() -> Self {
        Self { width: 0, height: 0, left: false, right: false, top: false, bottom: false }
    }

    /// If the cursor at `(x, y)` sits on an edge that has a neighbour, return
    /// that edge. Left/right take priority over top/bottom at the corners.
    pub fn hit(&self, x: i32, y: i32) -> Option<Edge> {
        if self.left && x <= 0 {
            return Some(Edge::Left);
        }
        if self.right && self.width > 0 && x >= self.width - 1 {
            return Some(Edge::Right);
        }
        if self.top && y <= 0 {
            return Some(Edge::Top);
        }
        if self.bottom && self.height > 0 && y >= self.height - 1 {
            return Some(Edge::Bottom);
        }
        None
    }
}

/// Absolute pixel where the cursor should appear when control enters a screen
/// of size `w`×`h` from `edge`, at perpendicular pixel `perp` along that edge
/// (vertical for left/right, horizontal for top/bottom).
pub fn entry_point(edge: Edge, perp: i32, w: u32, h: u32) -> (i32, i32) {
    let (wi, hi) = (w as i32, h as i32);
    let py = perp.clamp(0, hi - 1);
    let px = perp.clamp(0, wi - 1);
    match edge {
        Edge::Left => (2, py),
        Edge::Right => (wi - 3, py),
        Edge::Top => (px, 2),
        Edge::Bottom => (px, hi - 3),
    }
}

/// The perpendicular axis length of `edge` on a `w`×`h` screen: height for a
/// left/right edge, width for a top/bottom edge.
pub fn perp_dim(edge: Edge, w: u32, h: u32) -> u32 {
    match edge {
        Edge::Left | Edge::Right => h,
        Edge::Top | Edge::Bottom => w,
    }
}

/// Map a perpendicular position on the SERVER's exit edge to the CLIENT's local
/// entry position, given `offset` (the client screen's top/left relative to the
/// server's, in server pixels). Clamped to the client's dimension.
pub fn map_to_client(server_perp: i32, offset: i32, client_dim: u32) -> i32 {
    (server_perp - offset).clamp(0, client_dim as i32 - 1)
}

/// Inverse of [`map_to_client`]: map the client's local exit position back to
/// the server's local entry position.
pub fn map_to_server(client_perp: i32, offset: i32, server_dim: u32) -> i32 {
    (client_perp + offset).clamp(0, server_dim as i32 - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_point_lands_on_the_right_edge() {
        // Enter the LEFT edge 400px down an 800px-tall screen.
        assert_eq!(entry_point(Edge::Left, 400, 1000, 800), (2, 400));
        // Enter the BOTTOM edge 250px across a 1000px-wide screen.
        assert_eq!(entry_point(Edge::Bottom, 250, 1000, 800), (250, 797));
        // Out-of-range perp is clamped onto the screen.
        assert_eq!(entry_point(Edge::Left, 5000, 1000, 800), (2, 799));
    }

    #[test]
    fn offset_mapping_is_reciprocal() {
        // Client shifted DOWN by 200px relative to the server.
        // Server leaves at y=500 => client enters at 500-200 = 300.
        assert_eq!(map_to_client(500, 200, 1440), 300);
        // Client leaves at y=300 => server enters at 300+200 = 500.
        assert_eq!(map_to_server(300, 200, 956), 500);
        // Beyond the client screen is clamped.
        assert_eq!(map_to_client(100, 200, 1440), 0); // 100-200 = -100 -> 0
    }

    #[test]
    fn detects_right_edge_only_when_neighbour_present() {
        let cfg = EdgeConfig::new(1920, 1080, false, true, false, false);
        assert_eq!(cfg.hit(1919, 500), Some(Edge::Right));
        assert_eq!(cfg.hit(1920, 500), Some(Edge::Right));
        assert_eq!(cfg.hit(0, 500), None); // left has no neighbour
        assert_eq!(cfg.hit(960, 500), None); // middle
    }

    #[test]
    fn detects_all_configured_edges() {
        let cfg = EdgeConfig::new(1000, 800, true, true, true, true);
        assert_eq!(cfg.hit(0, 400), Some(Edge::Left));
        assert_eq!(cfg.hit(999, 400), Some(Edge::Right));
        assert_eq!(cfg.hit(500, 0), Some(Edge::Top));
        assert_eq!(cfg.hit(500, 799), Some(Edge::Bottom));
    }

    #[test]
    fn none_config_never_hits() {
        let cfg = EdgeConfig::none();
        assert_eq!(cfg.hit(0, 0), None);
        assert_eq!(cfg.hit(-5, -5), None);
    }
}
