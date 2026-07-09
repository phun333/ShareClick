//! Settings + monitor manager, persisted as TOML.
//!
//! The **monitor manager** is the layout of machines: which peer sits on each
//! screen edge. Auto edge-switching uses it to decide where the cursor goes
//! when it leaves a screen. The same file holds the pre-shared key used to
//! authenticate + encrypt the session.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use shareclick_protocol::Edge;

/// Top-level configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// This machine's name (must match one entry in `machines`).
    pub name: String,
    /// Pre-shared key / passphrase — authenticates peers and derives the
    /// session encryption keys. Keep it secret and identical on both machines.
    pub psk: String,
    /// Port used by both the input (UDP) and bulk (TCP) channels.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Enable moving control by pushing the cursor into a screen edge that has
    /// a neighbour (in addition to the F12 hotkey).
    #[serde(default = "default_true")]
    pub auto_edge_switch: bool,
    /// For a client: the server's host (or `host:port`) to connect to. Lets
    /// `connect` and the tray "Start Client" run without a CLI argument.
    #[serde(default)]
    pub server_host: Option<String>,
    /// Arrangement offset (pixels): the *other* screen's top (for left/right
    /// adjacency) or left (for top/bottom) relative to this screen's, so the
    /// cursor crosses at the exact placed position. 0 = tops/edges aligned.
    #[serde(default)]
    pub offset: i32,
    /// The machines participating and their geometry/neighbours.
    pub machines: Vec<Machine>,
}

/// One machine in the layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub name: String,
    /// Screen size in pixels. Optional — auto-detected at runtime if omitted.
    /// Set it only to override a wrong auto-detection.
    #[serde(default)]
    pub screen: Option<(u32, u32)>,
    /// Neighbour machine names by edge (any may be absent).
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
    #[serde(default)]
    pub top: Option<String>,
    #[serde(default)]
    pub bottom: Option<String>,
}

fn default_port() -> u16 {
    24800
}
fn default_true() -> bool {
    true
}

impl Config {
    /// Default config path, e.g. `~/.config/shareclick/config.toml`.
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("shareclick")
            .join("config.toml")
    }

    /// Load and parse a config file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot read config {}: {e}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("invalid config {}: {e}", path.display()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Serialize to a TOML string.
    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Write to `path`, creating parent directories.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.to_toml()?)?;
        Ok(())
    }

    /// A ready-to-edit two-machine example (mac ↔ windows, side by side).
    pub fn example() -> Self {
        Config {
            name: "mac".into(),
            psk: "change-me-to-a-long-random-passphrase".into(),
            port: default_port(),
            auto_edge_switch: true,
            server_host: Some("192.168.1.20".into()),
            offset: 0,
            machines: vec![
                Machine {
                    name: "mac".into(),
                    screen: None,
                    left: None,
                    right: Some("windows".into()),
                    top: None,
                    bottom: None,
                },
                Machine {
                    name: "windows".into(),
                    screen: None,
                    left: Some("mac".into()),
                    right: None,
                    top: None,
                    bottom: None,
                },
            ],
        }
    }

    /// Basic sanity checks.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.psk.len() < 8 {
            anyhow::bail!("psk must be at least 8 characters");
        }
        if self.machine(&self.name).is_none() {
            anyhow::bail!("name '{}' is not present in [machines]", self.name);
        }
        // Every referenced neighbour must exist.
        for m in &self.machines {
            for n in [&m.left, &m.right, &m.top, &m.bottom].into_iter().flatten() {
                if self.machine(n).is_none() {
                    anyhow::bail!("machine '{}' references unknown neighbour '{}'", m.name, n);
                }
            }
        }
        Ok(())
    }

    /// Look up a machine by name.
    pub fn machine(&self, name: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.name == name)
    }

    /// The neighbour of `machine` across `edge`, if any.
    pub fn neighbor(&self, machine: &str, edge: Edge) -> Option<&str> {
        let m = self.machine(machine)?;
        match edge {
            Edge::Left => m.left.as_deref(),
            Edge::Right => m.right.as_deref(),
            Edge::Top => m.top.as_deref(),
            Edge::Bottom => m.bottom.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_roundtrips_through_toml() {
        let cfg = Config::example();
        let text = cfg.to_toml().unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed, cfg);
        parsed.validate().unwrap();
    }

    #[test]
    fn neighbor_lookup_works() {
        let cfg = Config::example();
        assert_eq!(cfg.neighbor("mac", Edge::Right), Some("windows"));
        assert_eq!(cfg.neighbor("mac", Edge::Left), None);
        assert_eq!(cfg.neighbor("windows", Edge::Left), Some("mac"));
    }

    #[test]
    fn validate_rejects_unknown_neighbour() {
        let mut cfg = Config::example();
        cfg.machines[0].right = Some("ghost".into());
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_short_psk() {
        let mut cfg = Config::example();
        cfg.psk = "short".into();
        assert!(cfg.validate().is_err());
    }
}
