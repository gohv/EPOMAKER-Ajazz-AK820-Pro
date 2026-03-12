use eframe::egui;
use ak820_ctl::protocol::{LightingMode, MAX_BRIGHTNESS, MAX_SPEED};

use crate::actions;
use crate::state::*;
use crate::widgets;

pub struct Ak820App {
    lighting: LightingState,
    sleep: SleepState,
    clock: ClockState,
    status: ConnectionStatus,
    status_message: Option<(String, bool)>, // (message, is_error)
}

impl Ak820App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let status = match actions::probe_device() {
            Ok(_) => ConnectionStatus::Connected,
            Err(e) => ConnectionStatus::Error(e),
        };
        Self {
            lighting: LightingState::default(),
            sleep: SleepState::default(),
            clock: ClockState { last_sync: None },
            status,
            status_message: None,
        }
    }

    fn set_status(&mut self, result: Result<String, String>) {
        match result {
            Ok(msg) => {
                self.status = ConnectionStatus::Connected;
                self.status_message = Some((msg, false));
            }
            Err(msg) => {
                self.status = ConnectionStatus::Error(msg.clone());
                self.status_message = Some((msg, true));
            }
        }
    }
}

impl eframe::App for Ak820App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // ---- Connection status bar ----
            ui.horizontal(|ui| {
                match &self.status {
                    ConnectionStatus::Connected => {
                        widgets::status_label(ui, "⬤ Keyboard connected", false);
                    }
                    ConnectionStatus::Error(e) => {
                        widgets::status_label(ui, &format!("⬤ {}", e), true);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reconnect").clicked() {
                        self.set_status(actions::probe_device());
                    }
                });
            });
            ui.separator();

            // ---- Last action result ----
            if let Some((msg, is_error)) = &self.status_message {
                widgets::status_label(ui, msg, *is_error);
                ui.add_space(4.0);
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                // ==== LIGHTING ====
                widgets::section_header(ui, "💡 Lighting");

                // Mode dropdown
                let current_mode = self.lighting.current_mode();
                egui::ComboBox::from_label("Mode")
                    .selected_text(current_mode.name())
                    .show_ui(ui, |ui| {
                        for (i, mode) in LightingMode::ALL.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.lighting.mode_index,
                                i,
                                mode.name(),
                            );
                        }
                    });

                // Color picker
                let mut color_f32 = [
                    self.lighting.color[0] as f32 / 255.0,
                    self.lighting.color[1] as f32 / 255.0,
                    self.lighting.color[2] as f32 / 255.0,
                ];
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    if ui.color_edit_button_rgb(&mut color_f32).changed() {
                        self.lighting.color[0] = (color_f32[0] * 255.0) as u8;
                        self.lighting.color[1] = (color_f32[1] * 255.0) as u8;
                        self.lighting.color[2] = (color_f32[2] * 255.0) as u8;
                    }
                    ui.label(format!(
                        "#{:02x}{:02x}{:02x}",
                        self.lighting.color[0],
                        self.lighting.color[1],
                        self.lighting.color[2],
                    ));
                });

                // Rainbow toggle
                ui.checkbox(&mut self.lighting.rainbow, "Rainbow mode");

                // Brightness & Speed sliders
                widgets::int_slider(ui, "Brightness", &mut self.lighting.brightness, MAX_BRIGHTNESS);
                widgets::int_slider(ui, "Speed", &mut self.lighting.speed, MAX_SPEED);

                // Direction (conditional)
                let mode = self.lighting.current_mode();
                let dirs = mode.supported_directions();
                if !dirs.is_empty() {
                    if self.lighting.direction_index >= dirs.len() {
                        self.lighting.direction_index = 0;
                    }
                    let current_dir = dirs[self.lighting.direction_index];
                    egui::ComboBox::from_label("Direction")
                        .selected_text(format!("{:?}", current_dir))
                        .show_ui(ui, |ui| {
                            for (i, dir) in dirs.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.lighting.direction_index,
                                    i,
                                    format!("{:?}", dir),
                                );
                            }
                        });
                }

                ui.add_space(4.0);
                if ui.button("Apply Lighting").clicked() {
                    let result = actions::apply_lighting(
                        self.lighting.current_mode(),
                        self.lighting.color[0],
                        self.lighting.color[1],
                        self.lighting.color[2],
                        self.lighting.rainbow,
                        self.lighting.brightness,
                        self.lighting.speed,
                        self.lighting.current_direction(),
                    );
                    self.set_status(result);
                }

                // ==== SLEEP TIMER ====
                widgets::section_header(ui, "😴 Sleep Timer");

                egui::ComboBox::from_label("Sleep after")
                    .selected_text(SleepState::OPTIONS[self.sleep.selected].0)
                    .show_ui(ui, |ui| {
                        for (i, (label, _)) in SleepState::OPTIONS.iter().enumerate() {
                            ui.selectable_value(&mut self.sleep.selected, i, *label);
                        }
                    });

                ui.add_space(4.0);
                if ui.button("Apply Sleep Timer").clicked() {
                    let result = actions::apply_sleep(self.sleep.current());
                    self.set_status(result);
                }

                // ==== CLOCK ====
                widgets::section_header(ui, "🕐 Clock");

                ui.horizontal(|ui| {
                    if ui.button("Sync Time").clicked() {
                        let result = actions::sync_time();
                        if let Ok(ref msg) = result {
                            self.clock.last_sync = Some(msg.clone());
                        }
                        self.set_status(result);
                    }
                    if let Some(ref sync_msg) = self.clock.last_sync {
                        ui.label(sync_msg);
                    }
                });
            });
        });
    }
}
