//! Visual settings + monitor-arrangement window (like macOS "Displays").
//!
//! Drag the second monitor around the first to say where it sits; ShareClick
//! computes the edge adjacency so you can just push the cursor across (no
//! hotkey). Saves to the same `config.toml` the CLI uses.

#![cfg(feature = "gui")]

use eframe::egui;

use crate::config::{Config, Machine};

/// Distance between the two monitor centres along the shared edge (canvas px).
fn adjacency(side: Side, this: egui::Vec2, other: egui::Vec2) -> f32 {
    let gap = 3.0;
    match side {
        Side::Left | Side::Right => this.x / 2.0 + other.x / 2.0 + gap,
        Side::Top | Side::Bottom => this.y / 2.0 + other.y / 2.0 + gap,
    }
}

/// Second monitor's centre relative to the first, for a given side + offset.
fn seed_rel(side: Side, offset_canvas: f32, this: egui::Vec2, other: egui::Vec2) -> egui::Vec2 {
    let adj = adjacency(side, this, other);
    let perp_y = offset_canvas + (other.y - this.y) / 2.0;
    let perp_x = offset_canvas + (other.x - this.x) / 2.0;
    match side {
        Side::Right => egui::vec2(adj, perp_y),
        Side::Left => egui::vec2(-adj, perp_y),
        Side::Bottom => egui::vec2(perp_x, adj),
        Side::Top => egui::vec2(perp_x, -adj),
    }
}

/// Snap the parallel axis to adjacency; keep the perpendicular (offset) axis.
fn snap_rel(side: Side, rel: egui::Vec2, this: egui::Vec2, other: egui::Vec2) -> egui::Vec2 {
    let adj = adjacency(side, this, other);
    match side {
        Side::Right => egui::vec2(adj, rel.y),
        Side::Left => egui::vec2(-adj, rel.y),
        Side::Bottom => egui::vec2(rel.x, adj),
        Side::Top => egui::vec2(rel.x, -adj),
    }
}

/// Real-pixel offset (other screen's top/left vs this screen's) from placement.
fn offset_from_rel(side: Side, rel: egui::Vec2, this: egui::Vec2, other: egui::Vec2, scale: f32) -> i32 {
    let off_canvas = match side {
        Side::Left | Side::Right => rel.y - (other.y - this.y) / 2.0,
        Side::Top | Side::Bottom => rel.x - (other.x - this.x) / 2.0,
    };
    if scale <= 0.0 { 0 } else { (off_canvas / scale).round() as i32 }
}

/// Push `other_center` out of `this_rect` (plus a tiny gap) so the two monitors
/// never overlap — the second monitor's edge collides with the first's.
fn resolve_overlap(this: egui::Rect, other_center: egui::Pos2, other_size: egui::Vec2) -> egui::Pos2 {
    let gap = 2.0;
    let other = egui::Rect::from_center_size(other_center, other_size);
    let overlap_x = this.max.x.min(other.max.x) - this.min.x.max(other.min.x);
    let overlap_y = this.max.y.min(other.max.y) - this.min.y.max(other.min.y);
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return other_center;
    }
    let mut c = other_center;
    if overlap_x < overlap_y {
        let push = overlap_x + gap;
        if other_center.x >= this.center().x { c.x += push; } else { c.x -= push; }
    } else {
        let push = overlap_y + gap;
        if other_center.y >= this.center().y { c.y += push; } else { c.y -= push; }
    }
    c
}

fn dominant_side(rel: egui::Vec2) -> Side {
    if rel.x.abs() > rel.y.abs() {
        if rel.x > 0.0 { Side::Right } else { Side::Left }
    } else if rel.y > 0.0 {
        Side::Bottom
    } else {
        Side::Top
    }
}

/// Set one edge of a machine to point at `neighbour`.
fn set_side(m: &mut Machine, side: Side, neighbour: &str) {
    match side {
        Side::Left => m.left = Some(neighbour.into()),
        Side::Right => m.right = Some(neighbour.into()),
        Side::Top => m.top = Some(neighbour.into()),
        Side::Bottom => m.bottom = Some(neighbour.into()),
    }
}

/// Build the two machine entries for an arrangement: `this` borders `other` on
/// `side`, and reciprocally `other` borders `this` on the opposite side. Pure so
/// it can be unit-tested without the GUI.
fn layout(this_name: &str, other_name: &str, other_res: (u32, u32), side: Side) -> (Machine, Machine) {
    let mut this = Machine {
        name: this_name.into(),
        screen: None,
        left: None,
        right: None,
        top: None,
        bottom: None,
    };
    let mut other = Machine {
        name: other_name.into(),
        screen: Some(other_res),
        left: None,
        right: None,
        top: None,
        bottom: None,
    };
    set_side(&mut this, side, other_name);
    set_side(&mut other, side.opposite(), this_name);
    (this, other)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrangement_maps_to_reciprocal_neighbours() {
        let (t, o) = layout("mac", "win", (2560, 1440), Side::Right);
        assert_eq!(t.right.as_deref(), Some("win"));
        assert_eq!(o.left.as_deref(), Some("mac"));
        assert!(t.left.is_none() && t.top.is_none() && t.bottom.is_none());
        assert_eq!(o.screen, Some((2560, 1440)));
        assert!(t.screen.is_none());
        let (t, o) = layout("mac", "win", (1920, 1080), Side::Top);
        assert_eq!(t.top.as_deref(), Some("win"));
        assert_eq!(o.bottom.as_deref(), Some("mac"));
    }
}

/// Launch the settings window (blocks until closed).
pub fn run() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 640.0])
            .with_min_inner_size([460.0, 560.0])
            .with_icon(std::sync::Arc::new(egui::IconData {
                rgba: include_bytes!("tray_icon_64.rgba").to_vec(),
                width: 64,
                height: 64,
            }))
            .with_title("ShareClick — Settings & Monitor Manager"),
        ..Default::default()
    };
    eframe::run_native(
        "ShareClick Settings",
        options,
        Box::new(|cc| {
            // Clean light theme with the ShareClick blue accent.
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            let mut style = (*cc.egui_ctx.style()).clone();
            style.spacing.item_spacing = egui::vec2(10.0, 10.0);
            style.spacing.button_padding = egui::vec2(12.0, 6.0);
            let blue = egui::Color32::from_rgb(0x2b, 0x7a, 0xff);
            style.visuals.selection.bg_fill = blue;
            style.visuals.hyperlink_color = blue;
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, blue);
            cc.egui_ctx.set_style(style);
            Ok(Box::new(SettingsApp::new()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("settings window failed: {e}"))
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}
impl Side {
    fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
        }
    }
}

struct SettingsApp {
    psk: String,
    port: String,
    this_name: String,
    other_name: String,
    server_host: String,
    role: String,
    this_res: (u32, u32),
    other_w: String,
    other_h: String,
    /// Where the second monitor sits relative to the first.
    side: Side,
    /// Second monitor's centre relative to the first's (canvas px). The parallel
    /// axis is snapped to the chosen side; the perpendicular axis is the offset.
    drag: egui::Vec2,
    /// Perpendicular offset in real pixels (the other screen's top/left relative
    /// to this screen's), written to the config.
    offset: i32,
    /// Have we seeded `drag` from the saved offset yet?
    placed: bool,
    status: String,
}

impl SettingsApp {
    fn new() -> Self {
        let cfg = Config::load(&Config::default_path()).ok();
        let this_res = crate::emit::main_display_size().unwrap_or((1470, 956));

        // Derive current arrangement from an existing config, if any.
        let (this_name, other_name, side, other_res, psk, port, server_host, offset, role) = match &cfg {
            Some(c) => {
                let this = c.name.clone();
                let other = c
                    .machines
                    .iter()
                    .map(|m| m.name.clone())
                    .find(|n| n != &this)
                    .unwrap_or_else(|| "windows".into());
                let side = c
                    .machine(&this)
                    .map(|m| {
                        if m.right.is_some() {
                            Side::Right
                        } else if m.left.is_some() {
                            Side::Left
                        } else if m.top.is_some() {
                            Side::Top
                        } else {
                            Side::Bottom
                        }
                    })
                    .unwrap_or(Side::Right);
                let other_res = c
                    .machine(&other)
                    .and_then(|m| m.screen)
                    .unwrap_or((1920, 1080));
                (
                    this,
                    other,
                    side,
                    other_res,
                    c.psk.clone(),
                    c.port.to_string(),
                    c.server_host.clone().unwrap_or_default(),
                    c.offset,
                    c.role.clone().unwrap_or_else(|| "server".into()),
                )
            }
            None => (
                "mac".into(),
                "windows".into(),
                Side::Right,
                (1920, 1080),
                "change-me-to-a-long-passphrase".into(),
                "24800".into(),
                String::new(),
                0,
                "server".into(),
            ),
        };

        Self {
            psk,
            port,
            this_name,
            other_name,
            server_host,
            this_res,
            other_w: other_res.0.to_string(),
            other_h: other_res.1.to_string(),
            side,
            drag: egui::Vec2::ZERO,
            offset,
            placed: false,
            role,
            status: String::new(),
        }
    }

    fn build_config(&self) -> anyhow::Result<Config> {
        let port: u16 = self.port.trim().parse().unwrap_or(24800);
        let other_res = (
            self.other_w.trim().parse().unwrap_or(1920),
            self.other_h.trim().parse().unwrap_or(1080),
        );
        let (this, other) =
            layout(self.this_name.trim(), self.other_name.trim(), other_res, self.side);

        let cfg = Config {
            name: this.name.clone(),
            psk: self.psk.clone(),
            port,
            auto_edge_switch: true,
            server_host: {
                let s = self.server_host.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            },
            offset: self.offset,
            role: Some(self.role.clone()),
            machines: vec![this, other],
        };
        cfg.validate()?;
        Ok(cfg)
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ShareClick settings");
            ui.add_space(6.0);

            egui::Grid::new("fields").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label("Shared passphrase");
                ui.add(egui::TextEdit::singleline(&mut self.psk).password(true).desired_width(280.0));
                ui.end_row();
                ui.label("Port");
                ui.add(egui::TextEdit::singleline(&mut self.port).desired_width(100.0));
                ui.end_row();
                ui.label("This machine name");
                ui.text_edit_singleline(&mut self.this_name);
                ui.end_row();
                ui.label("This machine runs as");
                egui::ComboBox::from_id_source("role")
                    .selected_text(if self.role == "client" {
                        "Client (controlled)"
                    } else {
                        "Server (shares keyboard & mouse)"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.role, "server".into(), "Server (shares keyboard & mouse)");
                        ui.selectable_value(&mut self.role, "client".into(), "Client (controlled)");
                    });
                ui.end_row();
                ui.label("Other machine name");
                ui.text_edit_singleline(&mut self.other_name);
                ui.end_row();
                ui.label("Server host (client only)");
                ui.add(egui::TextEdit::singleline(&mut self.server_host).hint_text("blank = auto-discover"));
                ui.end_row();
                // Resolutions are never typed by hand: this machine is detected
                // live, and the other machine reports its size on connect (like
                // Deskflow's DINF). Shown read-only so they can't drift.
                ui.label("This screen");
                ui.label(format!("{} × {}  · auto-detected", self.this_res.0, self.this_res.1));
                ui.end_row();
                ui.label("Other screen");
                ui.label(format!(
                    "{} × {}  · reported on connect",
                    self.other_w.trim(),
                    self.other_h.trim()
                ));
                ui.end_row();
            });

            ui.add_space(10.0);
            ui.label("Arrange the screens — drag the second monitor to where it sits:");
            self.arrangement(ui);

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    match self.build_config() {
                        Ok(cfg) => match cfg.save(&Config::default_path()) {
                            Ok(_) => self.status = format!("Saved to {}", Config::default_path().display()),
                            Err(e) => self.status = format!("Save failed: {e}"),
                        },
                        Err(e) => self.status = format!("Invalid: {e}"),
                    }
                }
                if ui.button("Close").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            if !self.status.is_empty() {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::from_rgb(0x2b, 0x7a, 0xff), &self.status);
            }
        });
    }
}

impl SettingsApp {
    fn arrangement(&mut self, ui: &mut egui::Ui) {
        let (canvas, _) = ui.allocate_exact_size(egui::vec2(500.0, 280.0), egui::Sense::hover());
        let painter = ui.painter_at(canvas);
        painter.rect_filled(canvas, 10.0, egui::Color32::from_gray(238));
        painter.rect_stroke(canvas, 10.0, egui::Stroke::new(1.0, egui::Color32::from_gray(220)));

        let this_res = egui::vec2(self.this_res.0 as f32, self.this_res.1 as f32);
        let other_res = egui::vec2(
            self.other_w.trim().parse().unwrap_or(1920) as f32,
            self.other_h.trim().parse().unwrap_or(1080) as f32,
        );

        // Scale so BOTH monitors fit inside the canvas, whatever side they're on.
        let bbox = egui::vec2(this_res.x + other_res.x, this_res.y + other_res.y);
        let margin = 0.72;
        let scale = (canvas.width() * margin / bbox.x).min(canvas.height() * margin / bbox.y);
        let this_size = this_res * scale;
        let other_size = other_res * scale;

        // Seed the relative placement from the saved side + offset (once).
        if !self.placed {
            self.drag = seed_rel(self.side, self.offset as f32 * scale, this_size, other_size);
            self.placed = true;
        }
        // The second monitor can never OVERLAP the first — its edge collides
        // and slides (macOS Displays). We keep `self.drag` as the relative
        // placement, then re-centre the PAIR in the canvas each frame so nothing
        // ever spills outside the box.
        let c0 = canvas.center();
        let this_rect0 = egui::Rect::from_center_size(c0, this_size);
        let mut other0 = c0 + self.drag;
        other0 = resolve_overlap(this_rect0, other0, other_size);
        self.drag = other0 - c0;
        // Shift both so the bounding box of the two screens is centred.
        let bbox = this_rect0.union(egui::Rect::from_center_size(other0, other_size));
        let shift = canvas.center() - bbox.center();
        let this_center = c0 + shift;
        let other_center = other0 + shift;
        let this_rect = egui::Rect::from_center_size(this_center, this_size);
        let other_rect = egui::Rect::from_center_size(other_center, other_size);

        let blue = egui::Color32::from_rgb(0x2b, 0x7a, 0xff);
        let label = |p: &egui::Painter, r: egui::Rect, name: &str, w: u32, h: u32, col| {
            p.text(
                r.center() - egui::vec2(0.0, 7.0),
                egui::Align2::CENTER_CENTER,
                name,
                egui::FontId::proportional(13.0),
                col,
            );
            p.text(
                r.center() + egui::vec2(0.0, 9.0),
                egui::Align2::CENTER_CENTER,
                format!("{w}×{h}"),
                egui::FontId::proportional(11.0),
                col,
            );
        };

        // This monitor (blue, fixed).
        painter.rect_filled(this_rect, 6.0, blue);
        label(&painter, this_rect, &self.this_name, self.this_res.0, self.this_res.1, egui::Color32::WHITE);

        // Other monitor (draggable, grey).
        let resp = ui.interact(other_rect, ui.make_persistent_id("other_mon"), egui::Sense::drag());
        if resp.dragged() {
            self.drag += resp.drag_delta();
        }
        if resp.drag_stopped() {
            self.side = dominant_side(self.drag);
            self.drag = snap_rel(self.side, self.drag, this_size, other_size);
        }
        // Keep the saved offset in sync with the current placement.
        self.offset = offset_from_rel(self.side, self.drag, this_size, other_size, scale);
        let fill = if resp.dragged() {
            egui::Color32::from_gray(120)
        } else {
            egui::Color32::from_gray(150)
        };
        painter.rect_filled(other_rect, 6.0, fill);
        painter.rect_stroke(other_rect, 6.0, egui::Stroke::new(1.5, egui::Color32::from_gray(90)));
        label(&painter, other_rect, &self.other_name, other_res.x as u32, other_res.y as u32, egui::Color32::WHITE);
    }
}
