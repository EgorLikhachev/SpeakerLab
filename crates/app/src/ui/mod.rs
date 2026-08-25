//! Компоновка интерфейса.

pub mod box_calc;
pub mod driver_panel;
pub mod enclosure_panel;
pub mod library_window;
pub mod plots;
pub mod port_calc;
pub mod summary_bar;
pub mod util;

use eframe::egui;
use rust_i18n::t;

use crate::project;
use crate::state::App;

pub fn show(app: &mut App, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // Живой пересчёт — до отрисовки
    app.ensure_computed();

    // Заголовок окна отражает имя проекта и несохранённые изменения
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(app.title()));

    top_bar(app, ctx);

    egui::SidePanel::left("driver")
        .resizable(true)
        .default_width(300.0)
        .width_range(240.0..=420.0)
        .show(ctx, |ui| {
            driver_panel::show(ui, app);
        });

    egui::TopBottomPanel::bottom("summary")
        .default_height(76.0)
        .show(ctx, |ui| {
            summary_bar::show(ui, app);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::TopBottomPanel::top("enclosure").show_inside(ui, |ui| enclosure_panel::show(ui, app));
        egui::CentralPanel::default().show_inside(ui, |ui| {
            plots::show(ui, app);
        });
    });

    port_calc::window(ctx, app);
    box_calc::window(ctx, app);
    library_window::window(ctx, app);
}

fn top_bar(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(t!("menu.file"), |ui| {
                if ui.button(t!("menu.new")).clicked() {
                    app.reset();
                    ui.close();
                }
                if ui.button(t!("menu.open")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SpeakerLab", &["spkproj"])
                        .pick_file()
                    {
                        match project::load(&path) {
                            Ok(p) => {
                                p.apply_to(app);
                                app.project_path = Some(path);
                            }
                            Err(e) => eprintln!("open project: {e}"),
                        }
                    }
                    ui.close();
                }
                if ui.button(t!("menu.save")).clicked() {
                    save_project(app);
                    ui.close();
                }
                if ui.button(t!("menu.saveas")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SpeakerLab", &["spkproj"])
                        .set_file_name("project.spkproj")
                        .save_file()
                    {
                        if project::save(app, &path).is_ok() {
                            app.project_path = Some(path);
                            app.modified = false;
                        }
                    }
                    ui.close();
                }
                ui.separator();
                if ui.button(t!("menu.export_csv")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .set_file_name("curves.csv")
                        .save_file()
                    {
                        let _ = project::export_csv(app, &path);
                    }
                    ui.close();
                }
            });

            if ui.button(t!("menu.library")).clicked() {
                app.show_library = !app.show_library;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .selectable_label(app.lang == "en", "EN")
                    .on_hover_text("English")
                    .clicked()
                    && app.lang != "en"
                {
                    app.set_lang("en");
                }
                if ui
                    .selectable_label(app.lang == "ru", "RU")
                    .on_hover_text("Русский")
                    .clicked()
                    && app.lang != "ru"
                {
                    app.set_lang("ru");
                }
                ui.separator();
                ui.label(t!("top.voltage").to_string());
                let mut v = app.sim.voltage;
                let resp = ui.add(
                    egui::DragValue::new(&mut v)
                        .speed(0.1)
                        .range(0.1..=200.0)
                        .suffix(" В"),
                );
                if resp.changed() {
                    app.sim.voltage = v;
                    app.mark_dirty();
                }
            });
        });
    });
}

fn save_project(app: &mut App) {
    match &app.project_path {
        Some(path) => {
            if project::save(app, path).is_ok() {
                app.modified = false;
            }
        }
        None => {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("SpeakerLab", &["spkproj"])
                .set_file_name("project.spkproj")
                .save_file()
            {
                if project::save(app, &path).is_ok() {
                    app.project_path = Some(path);
                    app.modified = false;
                }
            }
        }
    }
}
