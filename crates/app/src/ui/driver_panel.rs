//! Левая панель: параметры динамика.

use egui::{Grid, Ui};
use rust_i18n::t;
use speakerlab_acoustics::driver::{DriverField, DriverIssue};

use crate::state::App;
use crate::ui::util::{colors, fnum, num_field, section};

pub fn show(ui: &mut Ui, app: &mut App) {
    ui.heading(t!("driver.title").to_string());
    ui.add_space(4.0);

    egui::ScrollArea::vertical()
        .id_salt("driver_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(t!("driver.name").to_string());
                ui.text_edit_singleline(&mut app.driver.name);
            });
            ui.horizontal(|ui| {
                ui.label(t!("driver.manufacturer").to_string());
                ui.text_edit_singleline(&mut app.driver.manufacturer);
            });

            section(ui, "driver.ts");

            let mut changed = false;
            Grid::new("ts_params")
                .num_columns(2)
                .min_col_width(110.0)
                .striped(true)
                .show(ui, |ui| {
                    changed |= num_field(
                        ui,
                        &t!("driver.re"),
                        &mut app.driver.re,
                        0.05,
                        0.1..=100.0,
                        " Ом",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.le"),
                        &mut app.driver.le,
                        0.01,
                        0.0..=50.0,
                        " мГн",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.fs"),
                        &mut app.driver.fs,
                        0.25,
                        1.0..=2000.0,
                        " Гц",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.qms"),
                        &mut app.driver.qms,
                        0.05,
                        0.1..=20.0,
                        "",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.qes"),
                        &mut app.driver.qes,
                        0.005,
                        0.05..=10.0,
                        "",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.vas"),
                        &mut app.driver.vas,
                        0.5,
                        0.1..=2000.0,
                        " л",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.sd"),
                        &mut app.driver.sd,
                        2.0,
                        1.0..=2500.0,
                        " см²",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.xmax"),
                        &mut app.driver.xmax,
                        0.1,
                        0.05..=100.0,
                        " мм",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.pe"),
                        &mut app.driver.pe,
                        5.0,
                        1.0..=10000.0,
                        " Вт",
                    );
                    changed |= num_field(
                        ui,
                        &t!("driver.spl"),
                        &mut app.driver.spl,
                        0.1,
                        60.0..=120.0,
                        " дБ",
                    );
                });
            if changed {
                app.mark_dirty();
            }

            // Производные величины
            section(ui, "driver.derived");
            let d = &app.driver;
            egui::Grid::new("derived")
                .num_columns(4)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.weak(t!("driver.qts").to_string());
                    ui.label(fnum(d.qts()));
                    ui.weak(t!("driver.bl").to_string());
                    ui.label(format!("{} Т·м", fnum(d.bl_tm())));
                    ui.end_row();
                    ui.weak(t!("driver.mms").to_string());
                    ui.label(format!("{} г", fnum(d.mms_kg() * 1e3)));
                    ui.weak(t!("driver.ebp").to_string());
                    ui.label(fnum(d.ebp()));
                    ui.end_row();
                });

            // Предупреждения по вводу
            for issue in d.issues() {
                let (text, color) = match &issue {
                    DriverIssue::NonPositive(f) => (
                        t!("driver.issue.nonpositive", field = field_key(*f)).to_string(),
                        colors::DANGER,
                    ),
                    DriverIssue::QtsHigh => {
                        (t!("driver.issue.qts_high").to_string(), colors::WARNING)
                    }
                    DriverIssue::QtsLow => {
                        (t!("driver.issue.qts_low").to_string(), colors::WARNING)
                    }
                    DriverIssue::Ebp { value } => (
                        t!("driver.issue.ebp", value = format!("{value:.0}")).to_string(),
                        egui::Color32::GRAY,
                    ),
                };
                ui.colored_label(color, format!("⚠ {text}"));
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(t!("driver.save_library").to_string()).clicked() {
                    if let Err(e) = crate::library::save_driver(&app.driver) {
                        eprintln!("Не удалось сохранить в библиотеку: {e}");
                    }
                    app.library = crate::library::load_library();
                }
                if ui.button(t!("driver.load_library").to_string()).clicked() {
                    app.show_library = true;
                }
            });
        });
}

fn field_key(f: DriverField) -> &'static str {
    match f {
        DriverField::Re => "driver.re",
        DriverField::Le => "driver.le",
        DriverField::Fs => "driver.fs",
        DriverField::Qms => "driver.qms",
        DriverField::Qes => "driver.qes",
        DriverField::Vas => "driver.vas",
        DriverField::Sd => "driver.sd",
        DriverField::Xmax => "driver.xmax",
    }
}
