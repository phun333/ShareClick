//! Visual settings + monitor-arrangement window (like macOS "Displays").
//!
//! Drag the second monitor around the first to say where it sits; ShareClick
//! computes the edge adjacency so you can just push the cursor across (no
//! hotkey). Saves to the same `config.toml` the CLI uses.

#![cfg(feature = "gui")]

use eframe::egui;

use crate::config::{Config, Machine};

/// Launch the settings window (blocks until closed).
pub fn run() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 640.0])
            .with_min_inner_size([460.0, 560.0])
            .with_title("ShareClick — Settings & Monitor Manager"),
        ..Default::default()
    };
    eframe::run_native(
        "ShareClick Settings",
        options,
        Box::new(|_cc| Ok(Box::new(SettingsApp::new()))),
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
    this_res: (u32, u32),
    other_w: String,
    other_h: String,
    /// Where the second monitor sits relative to the first.
    side: Side,
    /// Free-drag offset of the second monitor (canvas px); snapped on release.
    drag: egui::Vec2,
    status: String,
}

impl SettingsApp {
    fn new() -> Self {
        let cfg = Config::load(&Config::default_path()).ok();
        let this_res = crate::emit::main_display_size().unwrap_or((1470, 956));

        // Derive current arrangement from an existing config, if any.
        let (this_name, other_name, side, other_res, psk, port, server_host) = match &cfg {
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
            status: String::new(),
        }
    }

    fn build_config(&self) -> anyhow::Result<Config> {
        let port: u16 = self.port.trim().parse().unwrap_or(24800);
        let other_res = (
            self.other_w.trim().parse().unwrap_or(1920),
            self.other_h.trim().parse().unwrap_or(1080),
        );
        let set_side = |m: &mut Machine, side: Side, neighbor: &str| match side {
            Side::Left => m.left = Some(neighbor.into()),
            Side::Right => m.right = Some(neighbor.into()),
            Side::Top => m.top = Some(neighbor.into()),
            Side::Bottom => m.bottom = Some(neighbor.into()),
        };
        let mut this = Machine {
            name: self.this_name.trim().into(),
            screen: None, // auto-detected at runtime
            left: None,
            right: None,
            top: None,
            bottom: None,
        };
        let mut other = Machine {
            name: self.other_name.trim().into(),
            screen: Some(other_res),
            left: None,
            right: None,
            top: None,
            bottom: None,
        };
        set_side(&mut this, self.side, &other.name);
        set_side(&mut other, self.side.opposite(), &this.name);

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
                ui.label("Other machine name");
                ui.text_edit_singleline(&mut self.other_name);
                ui.end_row();
                ui.label("Server host (client only)");
                ui.add(egui::TextEdit::singleline(&mut self.server_host).hint_text("blank = auto-discover"));
                ui.end_row();
                ui.label("Other screen size");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.other_w).desired_width(60.0));
                    ui.label("×");
                    ui.add(egui::TextEdit::singleline(&mut self.other_h).desired_width(60.0));
                });
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
        let (canvas, _) = ui.allocate_exact_size(egui::vec2(500.0, 260.0), egui::Sense::hover());
        let painter = ui.painter_at(canvas);
        painter.rect_filled(canvas, 6.0, egui::Color32::from_gray(28));

        // Scale so both monitors fit side by side with margin.
        let other_res = (
            self.other_w.trim().parse().unwrap_or(1920) as f32,
            self.other_h.trim().parse().unwrap_or(1080) as f32,
        );
        let total_w = self.this_res.0 as f32 + other_res.0 + 400.0;
        let scale = (canvas.width() / total_w).min(canvas.height() / (self.this_res.1 as f32 + 200.0));
        let this_size = egui::vec2(self.this_res.0 as f32 * scale, self.this_res.1 as f32 * scale);
        let other_size = egui::vec2(other_res.0 * scale, other_res.1 * scale);

        // "This" monitor fixed at centre.
        let center = canvas.center();
        let this_rect = egui::Rect::from_center_size(center, this_size);

        // "Other" monitor position: snapped side + live drag.
        let snap_center = snapped_center(center, this_size, other_size, self.side);
        let other_center = snap_center + self.drag;
        let other_rect = egui::Rect::from_center_size(other_center, other_size);

        // Draw this monitor.
        painter.rect_filled(this_rect, 4.0, egui::Color32::from_rgb(0x2b, 0x7a, 0xff));
        painter.text(
            this_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}\n{}×{}", self.this_name, self.this_res.0, self.this_res.1),
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );

        // Draggable other monitor.
        let resp = ui.interact(other_rect, ui.make_persistent_id("other_mon"), egui::Sense::drag());
        if resp.dragged() {
            self.drag += resp.drag_delta();
        }
        if resp.drag_stopped() {
            // Snap to the nearest side of "this".
            let d = other_center - center;
            self.side = if d.x.abs() > d.y.abs() {
                if d.x > 0.0 { Side::Right } else { Side::Left }
            } else if d.y > 0.0 {
                Side::Bottom
            } else {
                Side::Top
            };
            self.drag = egui::Vec2::ZERO;
        }
        let fill = if resp.dragged() {
            egui::Color32::from_gray(150)
        } else {
            egui::Color32::from_gray(110)
        };
        painter.rect_filled(other_rect, 4.0, fill);
        painter.rect_stroke(other_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
        painter.text(
            other_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}\n{}×{}", self.other_name, other_res.0 as u32, other_res.1 as u32),
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );
    }
}

/// Centre point for the second monitor snapped to `side` of the first.
fn snapped_center(center: egui::Pos2, this: egui::Vec2, other: egui::Vec2, side: Side) -> egui::Pos2 {
    let gap = 2.0;
    match side {
        Side::Right => center + egui::vec2(this.x / 2.0 + other.x / 2.0 + gap, 0.0),
        Side::Left => center - egui::vec2(this.x / 2.0 + other.x / 2.0 + gap, 0.0),
        Side::Bottom => center + egui::vec2(0.0, this.y / 2.0 + other.y / 2.0 + gap),
        Side::Top => center - egui::vec2(0.0, this.y / 2.0 + other.y / 2.0 + gap),
    }
}
