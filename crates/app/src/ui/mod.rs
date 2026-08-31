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
    // Горячие клавиши
    handle_hotkeys(app, ctx);
    app.auto_snapshot();

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
    table_window(ctx, app);
    draw_toasts(app, ctx);
}

/// Окно с табличным видом кривых (до ~150 строк).
fn table_window(ctx: &egui::Context, app: &mut App) {
    if !app.show_table {
        return;
    }
    let mut open = app.show_table;
    egui::Window::new(t!("table.title").to_string())
        .open(&mut open)
        .default_size([520.0, 500.0])
        .show(ctx, |ui| {
            let Some(c) = &app.curves else {
                ui.weak(t!("sim.no_data").to_string());
                return;
            };
            let step = (c.freq.len() / 150).max(1);
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("curve_table")
                    .striped(true)
                    .num_columns(7)
                    .show(ui, |ui| {
                        ui.strong(t!("plot.freq"));
                        ui.strong(t!("plot.spl"));
                        ui.strong(t!("plot.impedance"));
                        ui.strong(t!("plot.phase"));
                        ui.strong(t!("plot.excursion"));
                        ui.strong(t!("plot.groupdelay"));
                        ui.strong(t!("plot.portvel"));
                        ui.end_row();
                        for i in (0..c.freq.len()).step_by(step) {
                            ui.monospace(format!("{:8.1}", c.freq[i]));
                            ui.monospace(format!("{:7.2}", c.spl[i]));
                            ui.monospace(format!("{:7.2}", c.z_mag[i]));
                            ui.monospace(format!("{:7.1}", c.z_phase[i]));
                            ui.monospace(format!("{:7.3}", c.excursion_mm[i]));
                            ui.monospace(format!("{:7.2}", c.group_delay_ms[i]));
                            if let Some(v) = &c.port_vel_m_s {
                                ui.monospace(format!("{:6.2}", v[i]));
                            } else {
                                ui.weak("—");
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    app.show_table = open;
}

/// Всплывающие уведомления об ошибках (нижний правый угол, 5 секунд).
fn draw_toasts(app: &mut App, ctx: &egui::Context) {
    app.toasts.retain(|(_, t)| t.elapsed().as_secs_f32() < 5.0);
    if app.toasts.is_empty() {
        return;
    }
    egui::Area::new(egui::Id::new("toasts"))
        .anchor(egui::Align2::RIGHT_BOTTOM, (-12.0, -12.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(60, 26, 26))
                .stroke(egui::Stroke::new(1.0_f32, crate::ui::util::colors::DANGER))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    for (msg, _) in app.toasts.clone() {
                        ui.colored_label(crate::ui::util::colors::DANGER, msg);
                    }
                });
        });
}

/// Горячие клавиши: Ctrl+Z/Y — undo/redo, Ctrl+N/O/S — проект.
fn handle_hotkeys(app: &mut App, ctx: &egui::Context) {
    let ctrl = ctx.input(|i| i.modifiers.ctrl);
    if !ctrl {
        return;
    }
    let (z, y, n, o, s_) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::Z),
            i.key_pressed(egui::Key::Y),
            i.key_pressed(egui::Key::N),
            i.key_pressed(egui::Key::O),
            i.key_pressed(egui::Key::S),
        )
    });
    if z && app.can_undo() {
        app.undo();
    } else if y && app.can_redo() {
        app.redo();
    } else if n {
        app.reset();
    } else if o {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SpeakerLab", &["spkproj"])
            .pick_file()
        {
            match project::load(&path) {
                Ok(p) => {
                    p.apply_to(app);
                    app.project_path = Some(path);
                }
                Err(e) => app.push_toast(t!("err.open_project", msg = e.to_string()).to_string()),
            }
        }
    } else if s_ {
        save_project(app);
    }
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
                            Err(e) => app.push_toast(
                                t!("err.open_project", msg = e.to_string()).to_string(),
                            ),
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
                if ui.button(t!("menu.export_png")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PNG", &["png"])
                        .set_file_name("plot.png")
                        .save_file()
                    {
                        app.png_path = Some(path);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                            egui::UserData::default(),
                        ));
                    }
                    ui.close();
                }
                if ui.button(t!("menu.table")).clicked() {
                    app.show_table = !app.show_table;
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

            ui.menu_button(t!("menu.view"), |ui| {
                ui.label(t!("view.theme").to_string());
                for (key, label) in [
                    ("dark", t!("theme.dark").to_string()),
                    ("light", t!("theme.light").to_string()),
                    ("system", t!("theme.system").to_string()),
                ] {
                    if ui.radio(app.theme == key, label).clicked() {
                        app.set_theme(key);
                    }
                }
                ui.separator();
                ui.label(t!("view.scale").to_string());
                let mut scale = app.font_scale;
                let r = ui.add(
                    egui::DragValue::new(&mut scale)
                        .speed(0.05)
                        .range(1.0..=1.6)
                        .fixed_decimals(2),
                );
                if r.changed() {
                    app.set_font_scale(scale);
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
                ui.label(t!("top.baffle").to_string());
                let mut b = app.baffle_m;
                let rb = ui.add(
                    egui::DragValue::new(&mut b)
                        .speed(0.01)
                        .range(0.0..=2.0)
                        .suffix(" m"),
                );
                if rb.changed() {
                    app.baffle_m = b;
                    app.mark_dirty();
                }
                ui.separator();
                ui.label(t!("top.voltage").to_string());
                let mut v = app.sim.voltage;
                let resp = ui.add(
                    egui::DragValue::new(&mut v)
                        .speed(0.1)
                        .range(0.1..=200.0)
                        .suffix(format!(" {}", crate::ui::util::unit("В"))),
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
