#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod agent;
mod app;
mod catalog;
mod guides;
mod install;
mod skills;
mod view;

fn main() -> eframe::Result<()> {
    if let Some(code) = agent::run_helper() {
        std::process::exit(code);
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1120.0, 760.0])
        .with_min_inner_size([420.0, 400.0])
        .with_title("bench suite")
        .with_app_id("bench-suite");
    if let Ok(icon) =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/bench-suite-256.png"))
    {
        viewport = viewport.with_icon(icon);
    }
    let opts = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("bench-suite", opts, Box::new(|cc| Ok(Box::new(app::Hub::new(cc)))))
}
