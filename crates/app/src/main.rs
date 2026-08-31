#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod library;
mod project;
mod state;
#[cfg(test)]
mod tests;
mod ui;

rust_i18n::i18n!("locales", fallback = "en");

use eframe::egui;

/// Применить тему к контексту egui.
fn apply_theme(ctx: &egui::Context, theme: &str) {
    let visuals = match theme {
        "light" => egui::Visuals::light(),
        "system" => {
            if ctx.style().visuals.dark_mode {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            }
        }
        _ => egui::Visuals::dark(),
    };
    ctx.set_visuals(visuals);
}

fn icon_data() -> Option<egui::IconData> {
    let bytes = include_bytes!("../../../assets/icon256.png");
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width: w,
        height: h,
    })
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([1024.0, 700.0])
            .with_title("SpeakerLab")
            .with_icon(icon_data().unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native(
        "SpeakerLab",
        options,
        Box::new(|cc| {
            let app = state::App::new(cc);
            cc.egui_ctx.set_pixels_per_point(app.font_scale);
            apply_theme(&cc.egui_ctx, &app.theme);
            Ok(Box::new(app))
        }),
    )
}

impl eframe::App for state::App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_screenshot(ctx);
        // тема и масштаб применяются каждый кадр (дёшево)
        apply_theme(ctx, &self.theme);
        if (ctx.pixels_per_point() - self.font_scale).abs() > 0.01 {
            ctx.set_pixels_per_point(self.font_scale);
        }
        ui::show(self, ctx, frame);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.persist(storage);
    }
}
