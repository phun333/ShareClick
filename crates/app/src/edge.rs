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

#[cfg(test)]
mod tests {
    use super::*;

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
