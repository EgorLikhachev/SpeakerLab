//! Окно расчёта размеров ящика.

use egui::Ui;
use rust_i18n::t;
use speakerlab_acoustics::boxdim::{port_displacement_l, BoxCalc, Dims, Ratio};
use speakerlab_acoustics::port::port_length_m;
use speakerlab_acoustics::port::EndCorrection;

use crate::state::{App, EnclosureKind};
use crate::ui::util::{colors, fnum, num_field, unit, uv};

pub struct BoxCalcState {
    pub open: bool,
    pub net_volume: f64,
    pub driver_l: f64,
    pub port_l: f64,
    pub brace_l: f64,
    pub wall_mm: f64,
    pub ratio: Ratio,
    pub mode_two_dims: bool,
    pub w_mm: f64,
    pub h_mm: f64,
}

impl Default for BoxCalcState {
    fn default() -> Self {
        Self {
            open: false,
            net_volume: 55.0,
            driver_l: 2.0,
            port_l: 2.5,
            brace_l: 0.5,
            wall_mm: 18.0,
            ratio: Ratio::Golden,
            mode_two_dims: false,
            w_mm: 300.0,
            h_mm: 450.0,
        }
    }
}

pub fn window(ctx: &egui::Context, app: &mut App) {
    if !app.box_calc.open {
        return;
    }
    let mut open = app.box_calc.open;
    egui::Window::new(t!("boxcalc.title").to_string())
        .open(&mut open)
        .default_width(560.0)
        .collapsible(false)
        .show(ctx, |ui| {
            body(ui, app);
        });
    app.box_calc.open = open;
}

fn body(ui: &mut Ui, app: &mut App) {
    // Синхронизация из проекта — значения готовим до взятия &mut app.box_calc.
    let mut want_sync = false;
    ui.horizontal(|ui| {
        if ui.button(t!("boxcalc.sync")).clicked() {
            want_sync = true;
        }
    });
    let synced = want_sync.then(|| {
        let bc_wall = app.box_calc.wall_mm;
        let port_l = match (app.kind, app.vented.port.as_ref()) {
            (EnclosureKind::Vented, Some(spec)) => port_displacement_l(
                spec.area_total_m2(),
                port_length_m(app.vented.vb, app.vented.fb, spec, EndCorrection::default()),
                if matches!(
                    spec.geometry,
                    speakerlab_acoustics::port::PortGeometry::Slot { .. }
                ) {
                    bc_wall
                } else {
                    0.0
                },
            ),
            _ => 0.0,
        };
        (
            app.net_volume(),
            speakerlab_acoustics::boxdim::driver_displacement_l(app.driver.sd),
            port_l,
        )
    });
    let bc = &mut app.box_calc;
    if let Some((net, drv, port)) = synced {
        bc.net_volume = net;
        bc.driver_l = drv;
        bc.port_l = port;
    }
    ui.add_space(4.0);

    let mut changed = false;
    egui::Grid::new("boxcalc_inputs")
        .num_columns(4)
        .min_col_width(110.0)
        .show(ui, |ui| {
            changed |= num_field(
                ui,
                &t!("boxcalc.net"),
                &mut bc.net_volume,
                0.25,
                0.5..=3000.0,
                " л",
            );
            changed |= num_field(
                ui,
                &t!("boxcalc.driver"),
                &mut bc.driver_l,
                0.05,
                0.0..=100.0,
                " л",
            );
            changed |= num_field(
                ui,
                &t!("boxcalc.port"),
                &mut bc.port_l,
                0.05,
                0.0..=100.0,
                " л",
            );
            changed |= num_field(
                ui,
                &t!("boxcalc.brace"),
                &mut bc.brace_l,
                0.05,
                0.0..=100.0,
                " л",
            );
            changed |= num_field(
                ui,
                &t!("boxcalc.wall"),
                &mut bc.wall_mm,
                0.5,
                5.0..=50.0,
                " мм",
            );
            ui.end_row();
        });

    ui.separator();

    // Режим: пропорции или два заданных размера
    ui.horizontal(|ui| {
        ui.label(t!("boxcalc.mode").to_string());
        ui.selectable_value(
            &mut bc.mode_two_dims,
            false,
            t!("boxcalc.by_ratio").to_string(),
        );
        ui.selectable_value(
            &mut bc.mode_two_dims,
            true,
            t!("boxcalc.two_dims").to_string(),
        );
    });

    let calc = BoxCalc {
        net_volume: bc.net_volume,
        driver_l: bc.driver_l,
        port_l: bc.port_l,
        brace_l: bc.brace_l,
        wall_mm: bc.wall_mm,
    };

    let dims: Dims = if bc.mode_two_dims {
        ui.horizontal(|ui| {
            let mut ch = false;
            let mut w = bc.w_mm;
            let mut h = bc.h_mm;
            ui.label(t!("boxcalc.width").to_string());
            ch |= ui
                .add(
                    egui::DragValue::new(&mut w)
                        .speed(2.0)
                        .range(50.0..=2000.0)
                        .suffix(format!(" {}", unit("мм"))),
                )
                .changed();
            ui.label(t!("boxcalc.height").to_string());
            ch |= ui
                .add(
                    egui::DragValue::new(&mut h)
                        .speed(2.0)
                        .range(50.0..=2500.0)
                        .suffix(" мм"),
                )
                .changed();
            if ch {
                bc.w_mm = w;
                bc.h_mm = h;
            }
        });
        Dims {
            w: bc.w_mm,
            h: bc.h_mm,
            d: calc.depth_for(bc.w_mm, bc.h_mm),
        }
    } else {
        ui.horizontal(|ui| {
            for (r, label) in [
                (Ratio::Golden, t!("ratio.golden").to_string()),
                (Ratio::Cube, t!("ratio.cube").to_string()),
                (Ratio::Tower, t!("ratio.tower").to_string()),
                (Ratio::Square, t!("ratio.square").to_string()),
            ] {
                ui.selectable_value(&mut bc.ratio, r, label);
            }
        });
        calc.dims_by_ratio(bc.ratio)
    };

    let ext = calc.external(dims);

    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(format!(
            "{} × {} × {} {} ({}) → {} × {} × {} {} ({})",
            fnum(dims.w),
            fnum(dims.h),
            fnum(dims.d),
            unit("мм"),
            t!("boxcalc.inside"),
            fnum(ext.w),
            fnum(ext.h),
            fnum(ext.d),
            unit("мм"),
            t!("boxcalc.outside")
        ))
        .strong()
        .size(15.0)
        .color(colors::HINT),
    );
    ui.label(format!(
        "{}: {} · {}: {} · {}: {}",
        t!("boxcalc.gross"),
        uv(fnum(calc.gross_volume()), "л"),
        t!("boxcalc.net"),
        uv(fnum(calc.net_for_dims(dims)), "л"),
        t!("boxcalc.panel_area"),
        fnum(panel_area(ext, bc.wall_mm))
    ));
}

/// Площадь панелей, м² (для раскроя).
fn panel_area(ext: Dims, wall_mm: f64) -> f64 {
    let t = wall_mm / 1000.0;
    let (w, h, d) = (ext.w / 1000.0, ext.h / 1000.0, ext.d / 1000.0);
    let (wi, hi, di) = (wi_of(w, t), wi_of(h, t), wi_of(d, t));
    2.0 * hi * di + 2.0 * wi * di + 2.0 * wi * hi
}
fn wi_of(v: f64, t: f64) -> f64 {
    (v - 2.0 * t).max(0.0)
}
