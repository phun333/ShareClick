//! Client-side input injection using `enigo`.
//!
//! Requires Accessibility permission on macOS and runs best on the UI thread.

#![cfg(feature = "native")]

use enigo::{Axis, Button as EButton, Coordinate, Direction, Enigo, Keyboard, Mouse, Settings};
use shareclick_protocol::{InputEvent, MouseButton};

use crate::keymap;

/// Query the main display size (width, height) in the OS coordinate space.
pub fn main_display_size() -> anyhow::Result<(u32, u32)> {
    let enigo = Enigo::new(&Settings::default())
        .map_err(|e| anyhow::anyhow!("failed to init display query: {e:?}"))?;
    let (w, h) = enigo
        .main_display()
        .map_err(|e| anyhow::anyhow!("main_display: {e:?}"))?;
    Ok((w.max(1) as u32, h.max(1) as u32))
}

pub struct Injector {
    enigo: Enigo,
}

impl Injector {
    pub fn new() -> anyhow::Result<Self> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("failed to init input injector: {e:?}"))?;
        Ok(Self { enigo })
    }

    /// Warp the cursor to an absolute screen position (used when control enters
    /// this machine, so the pointer appears where it crossed over).
    pub fn move_to(&mut self, x: i32, y: i32) -> anyhow::Result<()> {
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("move_to: {e:?}"))?;
        Ok(())
    }

    /// Apply one input event locally.
    pub fn apply(&mut self, ev: InputEvent) -> anyhow::Result<()> {
        match ev {
            InputEvent::MouseMove { dx, dy } => {
                self.enigo
                    .move_mouse(dx, dy, Coordinate::Rel)
                    .map_err(|e| anyhow::anyhow!("move_mouse: {e:?}"))?;
            }
            InputEvent::MouseButton { button, pressed } => {
                let b = to_enigo_button(button);
                let dir = if pressed {
                    Direction::Press
                } else {
                    Direction::Release
                };
                self.enigo
                    .button(b, dir)
                    .map_err(|e| anyhow::anyhow!("button: {e:?}"))?;
            }
            InputEvent::Scroll { dx, dy } => {
                if dy.abs() >= 1.0 {
                    self.enigo
                        .scroll(-(dy as i32), Axis::Vertical)
                        .map_err(|e| anyhow::anyhow!("scroll v: {e:?}"))?;
                }
                if dx.abs() >= 1.0 {
                    self.enigo
                        .scroll(dx as i32, Axis::Horizontal)
                        .map_err(|e| anyhow::anyhow!("scroll h: {e:?}"))?;
                }
            }
            InputEvent::Key { key, pressed } => {
                // Cross-platform modifier swap (standard KVM behaviour): the
                // Mac's Cmd acts as Ctrl on Windows (Cmd+C → Ctrl+C, and NOT
                // Win+C — which used to pop Copilot), and the PC's Ctrl acts as
                // Cmd on macOS (Ctrl+C → Cmd+C).
                #[cfg(target_os = "windows")]
                let key = match key {
                    shareclick_protocol::Key::LMeta => shareclick_protocol::Key::LCtrl,
                    shareclick_protocol::Key::RMeta => shareclick_protocol::Key::RCtrl,
                    k => k,
                };
                #[cfg(target_os = "macos")]
                let key = match key {
                    shareclick_protocol::Key::LCtrl => shareclick_protocol::Key::LMeta,
                    shareclick_protocol::Key::RCtrl => shareclick_protocol::Key::RMeta,
                    k => k,
                };
                if let Some(k) = keymap::to_enigo(key) {
                    let dir = if pressed {
                        Direction::Press
                    } else {
                        Direction::Release
                    };
                    self.enigo
                        .key(k, dir)
                        .map_err(|e| anyhow::anyhow!("key: {e:?}"))?;
                }
            }
        }
        Ok(())
    }
}

fn to_enigo_button(b: MouseButton) -> EButton {
    match b {
        MouseButton::Left => EButton::Left,
        MouseButton::Right => EButton::Right,
        MouseButton::Middle => EButton::Middle,
        MouseButton::Other(0) => EButton::Back,
        MouseButton::Other(_) => EButton::Forward,
    }
}
