//! Панель оформления: выбор типа, параметры, подсказки.

use egui::Ui;
use rust_i18n::t;
use speakerlab_acoustics::driver::Fill;
use speakerlab_acoustics::line::Segment;
use speakerlab_acoustics::suggest;

use crate::state::{App, EnclosureKind};
use crate::ui::util::{colors, fnum, num_field, uv};

pub fn show(ui: &mut Ui, app: &mut App) {
    type_buttons(ui, app);
    let mut changed = false;

    match app.kind {
        EnclosureKind::Sealed => changed |= sealed(ui, app),
        EnclosureKind::Vented => changed |= vented(ui, app),
        EnclosureKind::Passive => changed |= passive(ui, app),
        EnclosureKind::Bandpass4 => changed |= bandpass4(ui, app),
        EnclosureKind::Bandpass6 => changed |= bandpass6(ui, app),
        EnclosureKind::Line => changed |= line(ui, app),
    }
    if changed {
        app.mark_dirty();
    }
}

fn type_buttons(ui: &mut Ui, app: &mut App) {
    ui.horizontal(|ui| {
        let kinds = [
            (EnclosureKind::Sealed, "enc.sealed"),
            (EnclosureKind::Vented, "enc.vented"),
            (EnclosureKind::Passive, "enc.pr"),
            (EnclosureKind::Bandpass4, "enc.bp4"),
            (EnclosureKind::Bandpass6, "enc.bp6"),
            (EnclosureKind::Line, "enc.tl"),
        ];
        for (k, key) in kinds {
            ui.selectable_value(&mut app.kind, k, t!(key).to_string());
        }
    });
}

/// Кнопки «эталон/сброс эталона» и «размеры ящика».
fn common_actions(ui: &mut Ui, app: &mut App) {
    ui.horizontal(|ui| {
        if app.ref_curves.is_some() {
            if ui.button(t!("ref.clear").to_string()).clicked() {
                app.ref_curves = None;
            }
        } else if ui.button(t!("ref.set").to_string()).clicked() {
            if let Some(c) = &app.curves {
                app.ref_curves = Some((kind_label(app.kind), c.clone()));
            }
        }
        if ui.button(t!("boxcalc.title").to_string()).clicked() {
            app.box_calc.open = true;
        }
    });
}

fn kind_label(kind: EnclosureKind) -> String {
    let key = match kind {
        EnclosureKind::Sealed => "enc.sealed",
        EnclosureKind::Vented => "enc.vented",
        EnclosureKind::Passive => "enc.pr",
        EnclosureKind::Bandpass4 => "enc.bp4",
        EnclosureKind::Bandpass6 => "enc.bp6",
        EnclosureKind::Line => "enc.tl",
    };
    t!(key).to_string()
}

fn sealed(ui: &mut Ui, app: &mut App) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        egui::Grid::new("sealed_params")
            .num_columns(4)
            .show(ui, |ui| {
                changed |= num_field(
                    ui,
                    &t!("enc.sealed.vb"),
                    &mut app.sealed.vb,
                    0.2,
                    0.5..=2000.0,
                    " л",
                );
                ui.label(t!("enc.sealed.fill").to_string());
                let mut fill = app.sealed.fill;
                egui::ComboBox::from_id_salt("fill")
                    .selected_text(fill_label(fill))
                    .show_ui(ui, |ui| {
                        for f in [Fill::None, Fill::Light, Fill::Medium, Fill::Heavy] {
                            ui.selectable_value(&mut fill, f, fill_label(f));
                        }
                    });
                if app.sealed.fill != fill {
                    app.sealed.fill = fill;
                    changed = true;
                }
                ui.end_row();
            });
    });

    if let Some(s) = &app.summary {
        let d = &app.driver;
        let f3 = s
            .f3_low
            .map(|f| format!("F3 = {} Гц", fnum(f)))
            .unwrap_or_default();
        ui.label(
            egui::RichText::new(format!(
                "fc = {}   Qtc = {}   {f3}",
                uv(fnum(app.sealed.fc(d)), "Гц"),
                fnum(app.sealed.qtc(d))
            ))
            .color(colors::HINT),
        );
    }

    let mut apply: Option<(f64, Option<f64>)> = None;
    let suggestions = suggest::sealed_suggestions(&app.driver);
    if !suggestions.is_empty() {
        ui.horizontal(|ui| {
            ui.weak(t!("suggest.title").to_string());
            for s in &suggestions {
                let label = format!("{}: {}", t!(s.label_key), uv(fnum(s.vb), "л"));
                if ui
                    .button(egui::RichText::new(label).color(colors::HINT))
                    .on_hover_text(t!("suggest.apply").to_string())
                    .clicked()
                {
                    apply = Some((s.vb, None));
                }
            }
        });
    }
    if let Some((vb, _)) = apply {
        app.sealed.vb = vb;
        changed = true;
    }

    egui::CollapsingHeader::new(t!("enc.adv").to_string())
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("sealed_adv").num_columns(2).show(ui, |ui| {
                num_field(
                    ui,
                    &t!("enc.sealed.qa"),
                    &mut app.sealed.qa,
                    0.1,
                    1.0..=1.0e6,
                    "",
                );
            });
        });

    common_actions(ui, app);
    changed
}

fn vented(ui: &mut Ui, app: &mut App) -> bool {
    let mut changed = false;
    egui::Grid::new("vented_params")
        .num_columns(4)
        .show(ui, |ui| {
            changed |= num_field(
                ui,
                &t!("enc.vented.vb"),
                &mut app.vented.vb,
                0.2,
                1.0..=2000.0,
                " л",
            );
            changed |= num_field(
                ui,
                &t!("enc.vented.fb"),
                &mut app.vented.fb,
                0.2,
                10.0..=200.0,
                " Гц",
            );
        });

    ui.horizontal(|ui| {
        if let Some(s) = &app.summary {
            let f3 = s
                .f3_low
                .map(|f| format!("F3 = {}", uv(fnum(f), "Гц")))
                .unwrap_or_default();
            ui.label(
                egui::RichText::new(format!(
                    "α = {}   {f3}   |Z|min = {}",
                    fnum(app.driver.vas / app.vented.vb),
                    uv(fnum(s.z_min), "Ом")
                ))
                .color(colors::HINT),
            );
        }
        if ui.button(t!("enc.vented.port_open").to_string()).clicked() {
            app.port_calc.open_from_vented(&app.vented);
        }
    });

    let mut apply: Option<(f64, Option<f64>)> = None;
    let suggestions = suggest::vented_suggestions(&app.driver);
    if !suggestions.is_empty() {
        ui.horizontal(|ui| {
            ui.weak(t!("suggest.title").to_string());
            for s in &suggestions {
                let label = format!(
                    "{}: {} / {}",
                    t!(s.label_key),
                    uv(fnum(s.vb), "л"),
                    uv(fnum(s.fb.unwrap_or(0.0)), "Гц")
                );
                if ui
                    .button(egui::RichText::new(label).color(colors::HINT))
                    .on_hover_text(t!("suggest.apply").to_string())
                    .clicked()
                {
                    apply = Some((s.vb, s.fb));
                }
            }
        });
    }
    if let Some((vb, fb)) = apply {
        app.vented.vb = vb;
        if let Some(fb) = fb {
            app.vented.fb = fb;
        }
        changed = true;
    }

    egui::CollapsingHeader::new(t!("enc.adv").to_string())
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("vented_adv").num_columns(2).show(ui, |ui| {
                num_field(
                    ui,
                    &t!("enc.vented.ql"),
                    &mut app.vented.ql,
                    0.1,
                    1.0..=100.0,
                    "",
                );
            });
        });

    common_actions(ui, app);
    changed
}

fn passive(ui: &mut Ui, app: &mut App) -> bool {
    let mut changed = false;
    egui::Grid::new("pr_params").num_columns(8).show(ui, |ui| {
        changed |= num_field(
            ui,
            &t!("enc.pr.vb"),
            &mut app.passive.vb,
            0.2,
            1.0..=2000.0,
            " л",
        );
        changed |= num_field(
            ui,
            &t!("enc.pr.mass"),
            &mut app.passive.mass_g,
            1.0,
            10.0..=2000.0,
            " г",
        );
        changed |= num_field(
            ui,
            &t!("enc.pr.sd"),
            &mut app.passive.sd_cm2,
            1.0,
            10.0..=1500.0,
            " см²",
        );
        changed |= num_field(
            ui,
            &t!("enc.pr.fs"),
            &mut app.passive.fs_pr,
            0.2,
            5.0..=120.0,
            " Гц",
        );
    });

    let tuning = app.passive.tuning_hz();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "F({}) = {}   α = {}",
                t!("enc.pr.abbr"),
                uv(fnum(tuning), "Гц"),
                fnum(app.driver.vas / app.passive.vb)
            ))
            .color(colors::HINT),
        );
        // подсказка массы ПИ под «плоскую» настройку
        if let Some(s) = suggest::vented_suggestions(&app.driver).first() {
            if let Some(fb) = s.fb {
                if let Some(mass) = app.passive.mass_for_tuning(fb) {
                    let label = format!(
                        "{}: {} → F = {}",
                        t!("suggest.pr_mass"),
                        uv(fnum(mass), "г"),
                        uv(fnum(fb), "Гц")
                    );
                    if ui
                        .button(egui::RichText::new(label).color(colors::HINT))
                        .on_hover_text(t!("suggest.apply").to_string())
                        .clicked()
                        && mass > 0.0
                    {
                        app.passive.mass_g = mass;
                        changed = true;
                    }
                }
            }
        }
    });

    egui::CollapsingHeader::new(t!("enc.adv").to_string())
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("pr_adv").num_columns(2).show(ui, |ui| {
                num_field(
                    ui,
                    &t!("enc.vented.ql"),
                    &mut app.passive.ql,
                    0.1,
                    1.0..=100.0,
                    "",
                );
            });
        });

    common_actions(ui, app);
    changed
}

fn bandpass4(ui: &mut Ui, app: &mut App) -> bool {
    let mut changed = false;
    egui::Grid::new("bp4_params").num_columns(6).show(ui, |ui| {
        changed |= num_field(
            ui,
            &t!("enc.bp.rear"),
            &mut app.bp4.vb_rear,
            0.2,
            1.0..=2000.0,
            " л",
        );
        changed |= num_field(
            ui,
            &t!("enc.bp.front"),
            &mut app.bp4.vb_front,
            0.2,
            1.0..=2000.0,
            " л",
        );
        changed |= num_field(
            ui,
            &t!("enc.bp.fb"),
            &mut app.bp4.fb,
            0.2,
            20.0..=300.0,
            " Гц",
        );
    });
    if let Some(s) = &app.summary {
        ui.label(egui::RichText::new(band_text(s.f3_low, s.f3_high)).color(colors::HINT));
    }
    common_actions(ui, app);
    changed
}

fn bandpass6(ui: &mut Ui, app: &mut App) -> bool {
    let mut changed = false;
    egui::Grid::new("bp6_params").num_columns(8).show(ui, |ui| {
        changed |= num_field(
            ui,
            &t!("enc.bp.rear"),
            &mut app.bp6.vb_rear,
            0.2,
            1.0..=2000.0,
            " л",
        );
        changed |= num_field(
            ui,
            &t!("enc.bp.fb_rear"),
            &mut app.bp6.fb_rear,
            0.2,
            15.0..=200.0,
            " Гц",
        );
        changed |= num_field(
            ui,
            &t!("enc.bp.front"),
            &mut app.bp6.vb_front,
            0.2,
            1.0..=2000.0,
            " л",
        );
        changed |= num_field(
            ui,
            &t!("enc.bp.fb_front"),
            &mut app.bp6.fb_front,
            0.2,
            20.0..=300.0,
            " Гц",
        );
    });
    if let Some(s) = &app.summary {
        ui.label(egui::RichText::new(band_text(s.f3_low, s.f3_high)).color(colors::HINT));
    }
    common_actions(ui, app);
    changed
}

fn band_text(lo: Option<f64>, hi: Option<f64>) -> String {
    match (lo, hi) {
        (Some(lo), Some(hi)) => format!(
            "{}: {}–{}",
            t!("sum.band"),
            uv(fnum(lo), "Гц"),
            uv(fnum(hi), "Гц")
        ),
        (Some(lo), None) => format!("{}: {}", t!("sum.f3"), uv(fnum(lo), "Гц")),
        _ => String::new(),
    }
}

fn line(ui: &mut Ui, app: &mut App) -> bool {
    let mut changed = false;

    // Пресеты
    ui.horizontal(|ui| {
        ui.label(t!("line.presets").to_string());
        let sd = app.driver.sd;
        if ui.button(t!("line.preset.straight").to_string()).clicked() {
            set_preset(&mut app.line.segments, sd, Preset::Straight);
            changed = true;
        }
        if ui.button(t!("line.preset.tapered").to_string()).clicked() {
            set_preset(&mut app.line.segments, sd, Preset::Tapered);
            changed = true;
        }
        if ui.button(t!("line.preset.horn").to_string()).clicked() {
            set_preset(&mut app.line.segments, sd, Preset::Horn);
            changed = true;
        }
        if ui.button(t!("line.preset.qw").to_string()).clicked() {
            set_preset(&mut app.line.segments, sd, Preset::QuarterWave);
            changed = true;
        }
    });

    // Сегменты
    let mut remove: Option<usize> = None;
    egui::Grid::new("line_segments")
        .num_columns(6)
        .min_col_width(70.0)
        .striped(true)
        .show(ui, |ui| {
            ui.strong("#");
            ui.strong(t!("line.length").to_string());
            ui.strong(t!("line.area_start").to_string());
            ui.strong(t!("line.area_end").to_string());
            ui.strong(t!("line.stuffing").to_string());
            ui.strong("");
            ui.end_row();
            for (i, seg) in app.line.segments.iter_mut().enumerate() {
                ui.label(format!("{}", i + 1));
                changed |= drag_digits(ui, &mut seg.length_m, 0.01, 0.05..=6.0, " м", 2);
                changed |= drag_digits(ui, &mut seg.area_start_cm2, 2.0, 5.0..=5000.0, " см²", 0);
                changed |= drag_digits(ui, &mut seg.area_end_cm2, 2.0, 5.0..=5000.0, " см²", 0);
                changed |= drag_digits(ui, &mut seg.stuffing_kgm3, 0.1, 0.0..=60.0, "", 1);
                if ui.button("✕").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        if app.line.segments.len() > 1 {
            app.line.segments.remove(i);
            changed = true;
        }
    }
    ui.horizontal(|ui| {
        if ui.button(t!("line.add").to_string()).clicked() {
            let last = app.line.segments.last().cloned().unwrap_or_default();
            app.line.segments.push(Segment {
                area_start_cm2: last.area_end_cm2,
                ..Default::default()
            });
            changed = true;
        }
        ui.label(
            egui::RichText::new(format!(
                "L = {} · f(λ/4) ≈ {} · V = {}",
                uv(fnum(app.line.total_length()), "м"),
                uv(fnum(app.line.quarter_wave_hz()), "Гц"),
                uv(fnum(app.line.volume_l()), "л")
            ))
            .color(colors::HINT),
        );
    });

    egui::CollapsingHeader::new(t!("enc.adv").to_string())
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("line_adv").num_columns(2).show(ui, |ui| {
                ui.label(t!("line.wall_loss").to_string());
                ui.add(
                    egui::DragValue::new(&mut app.line.wall_loss)
                        .speed(0.01)
                        .range(0.0..=1.5)
                        .custom_formatter(|n, _| format!("{n:.2}")),
                );
                ui.end_row();
            });
        });

    common_actions(ui, app);
    changed
}

enum Preset {
    Straight,
    Tapered,
    Horn,
    QuarterWave,
}

fn set_preset(segments: &mut Vec<Segment>, sd_cm2: f64, preset: Preset) {
    let s1 = (sd_cm2 * 1.1).clamp(20.0, 1200.0);
    *segments = match preset {
        Preset::Straight => vec![
            Segment {
                length_m: 0.85,
                area_start_cm2: s1,
                area_end_cm2: s1,
                stuffing_kgm3: 0.0,
            },
            Segment {
                length_m: 0.85,
                area_start_cm2: s1,
                area_end_cm2: s1,
                stuffing_kgm3: 0.0,
            },
        ],
        Preset::Tapered => vec![
            Segment {
                length_m: 0.6,
                area_start_cm2: s1,
                area_end_cm2: s1 * 0.9,
                stuffing_kgm3: 8.0,
            },
            Segment {
                length_m: 0.6,
                area_start_cm2: s1 * 0.9,
                area_end_cm2: s1 * 0.8,
                stuffing_kgm3: 5.0,
            },
            Segment {
                length_m: 0.5,
                area_start_cm2: s1 * 0.8,
                area_end_cm2: s1 * 0.7,
                stuffing_kgm3: 0.0,
            },
        ],
        Preset::Horn => vec![
            Segment {
                length_m: 0.5,
                area_start_cm2: s1 * 0.35,
                area_end_cm2: s1 * 0.7,
                stuffing_kgm3: 0.0,
            },
            Segment {
                length_m: 0.5,
                area_start_cm2: s1 * 0.7,
                area_end_cm2: s1 * 1.4,
                stuffing_kgm3: 0.0,
            },
            Segment {
                length_m: 0.5,
                area_start_cm2: s1 * 1.4,
                area_end_cm2: s1 * 2.5,
                stuffing_kgm3: 0.0,
            },
            Segment {
                length_m: 0.4,
                area_start_cm2: s1 * 2.5,
                area_end_cm2: s1 * 4.0,
                stuffing_kgm3: 0.0,
            },
        ],
        Preset::QuarterWave => vec![
            Segment {
                length_m: 0.8,
                area_start_cm2: s1,
                area_end_cm2: s1,
                stuffing_kgm3: 10.0,
            },
            Segment {
                length_m: 0.8,
                area_start_cm2: s1,
                area_end_cm2: s1,
                stuffing_kgm3: 10.0,
            },
        ],
    };
}

fn fill_label(f: Fill) -> String {
    match f {
        Fill::None => t!("fill.none").to_string(),
        Fill::Light => t!("fill.light").to_string(),
        Fill::Medium => t!("fill.medium").to_string(),
        Fill::Heavy => t!("fill.heavy").to_string(),
    }
}

/// DragValue с фиксированным числом знаков.
fn drag_digits(
    ui: &mut Ui,
    v: &mut f64,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
    unit: &str,
    digits: usize,
) -> bool {
    let suffix = if unit.is_empty() {
        String::new()
    } else {
        format!(" {}", crate::ui::util::unit(unit))
    };
    ui.add(
        egui::DragValue::new(v)
            .speed(speed)
            .range(range)
            .suffix(&suffix)
            .custom_formatter(move |n, _| format!("{n:.digits$}")),
    )
    .changed()
}
