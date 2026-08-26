//! Окно-калькулятор порта фазоинвертора.

use egui::Ui;
use rust_i18n::t;
use speakerlab_acoustics::port::{
    fb_for_length, port_length_m, recommended_area_range, velocity_level, EndCorrection,
    PortGeometry, PortSpec, VelocityLevel,
};
use speakerlab_acoustics::vented::VentedBox;

use crate::state::{App, EnclosureKind};
use crate::ui::util::{colors, fnum, unit, uv};

pub struct PortCalc {
    pub open: bool,
    /// false: длина из Fb; true: Fb из длины
    pub from_length: bool,
    pub vb: f64,
    pub fb: f64,
    pub round: bool,
    pub d_mm: f64,
    pub w_mm: f64,
    pub h_mm: f64,
    pub count: u32,
    pub ec_index: usize,
    pub len_mm: f64,
}

impl Default for PortCalc {
    fn default() -> Self {
        Self {
            open: false,
            from_length: false,
            vb: 55.0,
            fb: 34.0,
            round: true,
            d_mm: 70.0,
            w_mm: 200.0,
            h_mm: 40.0,
            count: 1,
            ec_index: 0,
            len_mm: 180.0,
        }
    }
}

const END_CORRECTIONS: [EndCorrection; 3] = [
    EndCorrection::OneFlanged,
    EndCorrection::BothFlanged,
    EndCorrection::BothFree,
];

impl PortCalc {
    /// Открыть окно, подставив параметры текущего ФИ проекта.
    pub fn open_from_vented(&mut self, v: &VentedBox) {
        self.open = true;
        self.vb = v.vb;
        self.fb = v.fb;
        if let Some(spec) = &v.port {
            match spec.geometry {
                PortGeometry::Round { diameter_mm } => {
                    self.round = true;
                    self.d_mm = diameter_mm;
                }
                PortGeometry::Slot {
                    width_mm,
                    height_mm,
                } => {
                    self.round = false;
                    self.w_mm = width_mm;
                    self.h_mm = height_mm;
                }
            }
            self.count = spec.count.max(1);
        }
    }

    fn geometry(&self) -> PortGeometry {
        if self.round {
            PortGeometry::Round {
                diameter_mm: self.d_mm,
            }
        } else {
            PortGeometry::Slot {
                width_mm: self.w_mm,
                height_mm: self.h_mm,
            }
        }
    }

    fn spec(&self) -> PortSpec {
        PortSpec {
            geometry: self.geometry(),
            count: self.count.max(1),
        }
    }

    fn ec(&self) -> EndCorrection {
        END_CORRECTIONS[self.ec_index.min(2)]
    }
}

pub fn window(ctx: &egui::Context, app: &mut App) {
    if !app.port_calc.open {
        return;
    }
    let mut open = app.port_calc.open;
    egui::Window::new(t!("portcalc.title").to_string())
        .open(&mut open)
        .default_width(460.0)
        .collapsible(false)
        .show(ctx, |ui| {
            body(ui, app);
        });
    app.port_calc.open = open;
}

fn body(ui: &mut Ui, app: &mut App) {
    inputs(ui, &mut app.port_calc);
    ui.add_space(6.0);

    // Живые результаты
    let pc = &app.port_calc;
    let spec = pc.spec();
    let ec = pc.ec();
    let area_cm2 = spec.area_total_m2() * 1.0e4;
    let (len_mm, fb_result) = if pc.from_length {
        (
            pc.len_mm,
            fb_for_length(pc.vb, pc.len_mm / 1.0e3, &spec, ec),
        )
    } else {
        (port_length_m(pc.vb, pc.fb, &spec, ec) * 1.0e3, pc.fb)
    };

    ui.label(
        egui::RichText::new(format!(
            "⌀ {} → L = {}   (Fb ≈ {})",
            uv(format!("{:.0}", spec.hydraulic_diameter_m() * 1e3), "мм"),
            uv(fnum(len_mm), "мм"),
            uv(fnum(fb_result), "Гц")
        ))
        .strong()
        .size(16.0)
        .color(colors::HINT),
    );

    // Площадь: рекомендация по Sd
    let (lo, hi) = recommended_area_range(app.driver.sd_m2());
    let area_ok = area_cm2 >= lo * 1.0e4 * 0.999;
    let area_hint = format!(
        "{}: {} · {} {}–{}",
        t!("portcalc.area"),
        uv(fnum(area_cm2), "см²"),
        t!("portcalc.area_rec"),
        uv(fnum(lo * 1.0e4), "см²"),
        uv(fnum(hi * 1.0e4), "см²")
    );
    ui.colored_label(
        if area_ok {
            colors::EXCURSION
        } else {
            colors::WARNING
        },
        area_hint,
    );

    // Скорость воздуха, если порт применён к проекту
    let vel_line = app
        .summary
        .as_ref()
        .and_then(|s| s.port_vel_max_m_s)
        .map(|v| {
            let (color, key) = match velocity_level(v) {
                VelocityLevel::Ok => (colors::EXCURSION, "portcalc.vel_ok"),
                VelocityLevel::Caution => (colors::WARNING, "portcalc.vel_caution"),
                VelocityLevel::Excessive => (colors::DANGER, "portcalc.vel_excessive"),
            };
            ui.colored_label(color, t!(key, v = fnum(v)).to_string());
        });
    if vel_line.is_none() {
        ui.weak(t!("portcalc.apply_hint").to_string());
    }

    ui.add_space(6.0);
    let vb = pc.vb;
    ui.horizontal(|ui| {
        if ui
            .button(egui::RichText::new(t!("portcalc.apply").to_string()).strong())
            .clicked()
        {
            app.vented.vb = vb;
            app.vented.fb = fb_result;
            app.vented.port = Some(spec.clone());
            app.mark_dirty();
        }
        if app.kind == EnclosureKind::Vented && ui.button(t!("portcalc.sync").to_string()).clicked()
        {
            app.port_calc.open_from_vented(&app.vented);
        }
    });
}

/// Блок ввода параметров порта.
fn inputs(ui: &mut Ui, pc: &mut PortCalc) {
    ui.horizontal(|ui| {
        ui.label(t!("portcalc.mode").to_string());
        ui.selectable_value(
            &mut pc.from_length,
            false,
            t!("portcalc.mode.tolength").to_string(),
        );
        ui.selectable_value(
            &mut pc.from_length,
            true,
            t!("portcalc.mode.tofb").to_string(),
        );
    });

    egui::Grid::new("port_inputs")
        .num_columns(4)
        .min_col_width(90.0)
        .show(ui, |ui| {
            ui.label(t!("portcalc.vb").to_string());
            ui.add(
                egui::DragValue::new(&mut pc.vb)
                    .speed(0.2)
                    .range(1.0..=2000.0)
                    .suffix(format!(" {}", unit("л"))),
            );
            if !pc.from_length {
                ui.label(t!("portcalc.fb").to_string());
                ui.add(
                    egui::DragValue::new(&mut pc.fb)
                        .speed(0.2)
                        .range(10.0..=200.0)
                        .suffix(format!(" {}", unit("Гц"))),
                );
            }
            ui.end_row();

            ui.label(t!("portcalc.shape").to_string());
            ui.horizontal(|ui| {
                ui.selectable_value(&mut pc.round, true, t!("portcalc.round").to_string());
                ui.selectable_value(&mut pc.round, false, t!("portcalc.slot").to_string());
            });
            ui.end_row();

            if pc.round {
                ui.label(t!("portcalc.diameter").to_string());
                ui.add(
                    egui::DragValue::new(&mut pc.d_mm)
                        .speed(1.0)
                        .range(10.0..=500.0)
                        .suffix(format!(" {}", unit("мм"))),
                );
            } else {
                ui.label(t!("portcalc.width").to_string());
                ui.add(
                    egui::DragValue::new(&mut pc.w_mm)
                        .speed(2.0)
                        .range(10.0..=1000.0)
                        .suffix(" мм"),
                );
                ui.label(t!("portcalc.height").to_string());
                ui.add(
                    egui::DragValue::new(&mut pc.h_mm)
                        .speed(1.0)
                        .range(5.0..=500.0)
                        .suffix(" мм"),
                );
            }
            ui.end_row();

            ui.label(t!("portcalc.count").to_string());
            ui.add(egui::DragValue::new(&mut pc.count).range(1..=8));
            ui.end_row();

            ui.label(t!("portcalc.ec").to_string());
            let names = [
                t!("ec.one_flanged").to_string(),
                t!("ec.both_flanged").to_string(),
                t!("ec.both_free").to_string(),
            ];
            egui::ComboBox::from_id_salt("ec")
                .selected_text(names[pc.ec_index.min(2)].clone())
                .show_ui(ui, |ui| {
                    for (i, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut pc.ec_index, i, n.clone());
                    }
                });
            ui.end_row();

            if pc.from_length {
                ui.label(t!("portcalc.length").to_string());
                ui.add(
                    egui::DragValue::new(&mut pc.len_mm)
                        .speed(2.0)
                        .range(20.0..=3000.0)
                        .suffix(" мм"),
                );
                ui.end_row();
            }
        });
}
