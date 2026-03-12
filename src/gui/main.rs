mod app;
mod state;
mod widgets;
mod actions;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("AK820 Pro Control Panel")
            .with_inner_size([440.0, 560.0])
            .with_min_inner_size([400.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ak820-gui",
        options,
        Box::new(|cc| Ok(Box::new(app::Ak820App::new(cc)))),
    )
}
