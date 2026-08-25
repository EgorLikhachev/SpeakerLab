#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod library;
mod project;
mod state;
mod ui;

rust_i18n::i18n!("locales", fallback = "en");

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([1024.0, 700.0])
            .with_title("SpeakerLab"),
        ..Default::default()
    };
    eframe::run_native(
        "SpeakerLab",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(1.2);
            Ok(Box::new(state::App::new()))
        }),
    )
}

impl eframe::App for state::App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ui::show(self, ctx, frame);
    }
}
