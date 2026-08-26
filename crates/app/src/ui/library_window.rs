//! Окно библиотеки динамиков.

use egui::{Context, Ui};
use rust_i18n::t;

use crate::library;
use crate::state::App;
use crate::ui::util::{colors, fnum, uv};

pub fn window(ctx: &Context, app: &mut App) {
    if !app.show_library {
        return;
    }
    let mut open = app.show_library;
    egui::Window::new(t!("lib.title").to_string())
        .open(&mut open)
        .default_size([420.0, 480.0])
        .show(ctx, |ui| {
            body(ui, app);
        });
    app.show_library = open;
}

fn body(ui: &mut Ui, app: &mut App) {
    ui.horizontal(|ui| {
        if ui.button(t!("lib.save_current").to_string()).clicked()
            && library::save_driver(&app.driver).is_ok()
        {
            app.library = library::load_library();
        }
        if ui.button(t!("lib.import").to_string()).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                if let Some(d) = library::import_driver(&path) {
                    if library::save_driver(&d).is_ok() {
                        app.library = library::load_library();
                    }
                }
            }
        }
    });
    ui.separator();

    if app.library.is_empty() {
        ui.colored_label(colors::WARNING, t!("lib.empty").to_string());
        return;
    }

    let mut action: Option<LibAction> = None;
    let mut selected = app.library_selected;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (i, d) in app.library.iter().enumerate() {
            let is_sel = selected == Some(i);
            let title = if d.manufacturer.is_empty() {
                d.name.clone()
            } else {
                format!("{} {}", d.manufacturer, d.name)
            };
            let sub = format!(
                "Fs={} · Qts={} · Vas={} · Sd={}",
                uv(fnum(d.fs), "Гц"),
                fnum(d.qts()),
                uv(fnum(d.vas), "л"),
                uv(fnum(d.sd), "см²")
            );
            let resp = ui.selectable_label(is_sel, egui::RichText::new(title).strong());
            if resp.clicked() {
                selected = Some(i);
            }
            if is_sel {
                ui.indent("lib_item", |ui| {
                    ui.weak(sub);
                    ui.horizontal(|ui| {
                        if ui.button(t!("lib.load").to_string()).clicked() {
                            action = Some(LibAction::Load(i));
                        }
                        if ui.button(t!("lib.export").to_string()).clicked() {
                            action = Some(LibAction::Export(i));
                        }
                        if ui
                            .button(
                                egui::RichText::new(t!("lib.delete").to_string())
                                    .color(colors::DANGER),
                            )
                            .clicked()
                        {
                            action = Some(LibAction::Delete(i));
                        }
                    });
                    ui.add_space(4.0);
                });
            }
        }
    });
    app.library_selected = selected;

    match action {
        Some(LibAction::Load(i)) => {
            if let Some(d) = app.library.get(i).cloned() {
                app.driver = d;
                app.mark_dirty();
            }
        }
        Some(LibAction::Export(i)) => {
            if let Some(d) = app.library.get(i) {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name(format!("{}.json", sanitize(&d.name)))
                    .save_file()
                {
                    let _ = library::export_driver(d, &path);
                }
            }
        }
        Some(LibAction::Delete(i)) => {
            if let Some(d) = app.library.get(i) {
                if library::delete_driver(d).is_ok() {
                    app.library = library::load_library();
                    app.library_selected = None;
                }
            }
        }
        None => {}
    }
}

enum LibAction {
    Load(usize),
    Export(usize),
    Delete(usize),
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
