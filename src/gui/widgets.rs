use eframe::egui::{self, Ui};

/// Section header with spacing and separator.
pub fn section_header(ui: &mut Ui, label: &str) {
    ui.add_space(8.0);
    ui.heading(label);
    ui.separator();
}

/// Integer slider (0..=max) with label. Returns true if changed.
pub fn int_slider(ui: &mut Ui, label: &str, value: &mut u8, max: u8) -> bool {
    let mut v = *value as i32;
    let resp = ui.add(
        egui::Slider::new(&mut v, 0..=(max as i32))
            .text(label)
            .integer()
            .step_by(1.0),
    );
    let changed = resp.changed();
    *value = v as u8;
    changed
}

/// Colored status label — green for success, red for error.
pub fn status_label(ui: &mut Ui, text: &str, is_error: bool) {
    let color = if is_error {
        egui::Color32::from_rgb(220, 50, 50)
    } else {
        egui::Color32::from_rgb(50, 200, 50)
    };
    ui.colored_label(color, text);
}
