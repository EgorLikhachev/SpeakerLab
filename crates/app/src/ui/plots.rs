//! Графики: SPL, импеданс, фаза, экскурсия, скорость порта, групповая задержка.
//! Эталонные кривые (сравнение) рисуются серым пунктиром.

use egui::Ui;
use egui_plot::{GridInput, GridMark, Legend, Line, LineStyle, Plot, PlotPoints, VLine};
use rust_i18n::t;
use speakerlab_acoustics::response::Curves;

use crate::state::{App, EnclosureKind};
use crate::ui::util::{colors, fmt_hz};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlotTab {
    #[default]
    Spl,
    Impedance,
    Phase,
    Excursion,
    PortVel,
    GroupDelay,
}

impl PlotTab {
    fn label(self) -> String {
        match self {
            PlotTab::Spl => t!("plot.spl").to_string(),
            PlotTab::Impedance => t!("plot.impedance").to_string(),
            PlotTab::Phase => t!("plot.phase").to_string(),
            PlotTab::Excursion => t!("plot.excursion").to_string(),
            PlotTab::PortVel => t!("plot.portvel").to_string(),
            PlotTab::GroupDelay => t!("plot.groupdelay").to_string(),
        }
    }

    const ALL: [PlotTab; 6] = [
        PlotTab::Spl,
        PlotTab::Impedance,
        PlotTab::Phase,
        PlotTab::Excursion,
        PlotTab::PortVel,
        PlotTab::GroupDelay,
    ];
}

pub fn show(ui: &mut Ui, app: &mut App) {
    // Выбор вкладки
    ui.horizontal(|ui| {
        for tab in PlotTab::ALL {
            let needs_port = tab == PlotTab::PortVel;
            let enabled = !needs_port
                || app
                    .curves
                    .as_ref()
                    .and_then(|c| c.port_vel_m_s.as_ref())
                    .is_some();
            let selected = app.plot_tab == tab;
            if ui
                .add_enabled(enabled, egui::Button::selectable(selected, tab.label()))
                .clicked()
            {
                app.plot_tab = tab;
            }
        }
    });

    let Some(curves) = &app.curves else {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new(t!("sim.no_data").to_string())
                    .color(colors::WARNING)
                    .size(18.0),
            );
        });
        return;
    };
    let ref_curves = app.ref_curves.as_ref().map(|(_, c)| c);

    let rect = match app.plot_tab {
        PlotTab::Spl => spl(ui, app, curves, ref_curves),
        PlotTab::Impedance => impedance(ui, curves, ref_curves),
        PlotTab::Phase => phase(ui, curves, ref_curves),
        PlotTab::Excursion => excursion(ui, app, curves, ref_curves),
        PlotTab::PortVel => port_vel(ui, curves),
        PlotTab::GroupDelay => group_delay(ui, curves, ref_curves),
    };
    if let Some(r) = rect {
        app.plot_rect = Some(r);
    }
}

fn log_points<'a>(freq: &[f64], vals: &[f64]) -> PlotPoints<'a> {
    PlotPoints::from_iter(freq.iter().zip(vals.iter()).map(|(f, v)| [f.log10(), *v]))
}

fn hz_axis_formatter(gm: GridMark, _range: &std::ops::RangeInclusive<f64>) -> String {
    fmt_hz(10.0f64.powf(gm.value))
}

fn log_grid(input: GridInput) -> Vec<GridMark> {
    let (min, max) = input.bounds;
    let mut marks = Vec::new();
    let start = min.floor() as i32;
    let end = max.ceil() as i32;
    for decade in start..=end {
        for m in [1.0f64, 2.0, 5.0] {
            let x = decade as f64 + m.log10();
            if x >= min - 1e-9 && x <= max + 1e-9 {
                marks.push(GridMark {
                    value: x,
                    step_size: 1.0,
                });
            }
        }
    }
    marks
}

fn base_plot<'a>(id: &str) -> Plot<'a> {
    Plot::new(id)
        .legend(Legend::default())
        .x_axis_formatter(hz_axis_formatter)
        .x_grid_spacer(log_grid)
        .label_formatter(|name, value| {
            format!(
                "{}\n{} {}",
                name,
                fmt_hz(10.0f64.powf(value.x)),
                format_y(value.y)
            )
        })
        .x_axis_label(t!("plot.freq").to_string())
        .allow_scroll(false)
        .allow_boxed_zoom(false)
}

fn format_y(y: f64) -> String {
    if y.abs() >= 100.0 {
        format!("{y:.0}")
    } else if y.abs() >= 10.0 {
        format!("{y:.1}")
    } else {
        format!("{y:.2}")
    }
}

/// Серая пунктирная линия эталона (если задан).
fn ref_line<'a>(label: &str, freq: &[f64], vals: &[f64]) -> Line<'a> {
    Line::new(format!("({label})"), log_points(freq, vals))
        .color(egui::Color32::GRAY)
        .width(1.2_f32)
        .style(LineStyle::dashed_dense())
}

/// Вертикальный маркер частоты настройки для текущего типа оформления.
fn tuning_markers(app: &App) -> Vec<VLine> {
    let mut out = Vec::new();
    let mut push = |x: f64, key: &'static str| {
        if (5.0..=5000.0).contains(&x) {
            out.push(
                VLine::new(t!(key).to_string(), x.log10())
                    .color(colors::HINT)
                    .width(1.0_f32)
                    .style(LineStyle::dashed_dense()),
            );
        }
    };
    match app.kind {
        EnclosureKind::Sealed => push(app.sealed.fc(&app.driver), "plot.marker_fc"),
        EnclosureKind::Vented => push(app.vented.fb, "plot.marker_fb"),
        EnclosureKind::Passive => push(app.passive.tuning_hz(), "plot.marker_fb"),
        EnclosureKind::Bandpass4 => push(app.bp4.fb, "plot.marker_fb"),
        EnclosureKind::Bandpass6 => {
            push(app.bp6.fb_rear, "plot.marker_fb");
            push(app.bp6.fb_front, "plot.marker_fb2");
        }
        EnclosureKind::Line => push(app.line.quarter_wave_hz(), "plot.marker_qw"),
    }
    out
}

fn spl(ui: &mut Ui, app: &App, curves: &Curves, ref_c: Option<&Curves>) -> Option<egui::Rect> {
    let line = Line::new(
        t!("plot.spl").to_string(),
        log_points(&curves.freq, &curves.spl),
    )
    .color(colors::SPL)
    .width(1.8_f32);
    let markers = tuning_markers(app);
    let ref_l = ref_c.map(|rc| ref_line(&t!("plot.ref"), &rc.freq, &rc.spl));
    let resp = base_plot("spl")
        .y_axis_label(t!("plot.spl.y").to_string())
        .show(ui, |pu| {
            pu.line(line);
            if let Some(r) = ref_l {
                pu.line(r);
            }
            for m in markers {
                pu.vline(m);
            }
        });
    Some(resp.response.rect)
}

fn impedance(ui: &mut Ui, curves: &Curves, ref_c: Option<&Curves>) -> Option<egui::Rect> {
    let line = Line::new(
        t!("plot.impedance").to_string(),
        log_points(&curves.freq, &curves.z_mag),
    )
    .color(colors::Z)
    .width(1.8_f32);
    let ref_l = ref_c.map(|rc| ref_line(&t!("plot.ref"), &rc.freq, &rc.z_mag));
    let resp = base_plot("impedance")
        .y_axis_label(t!("plot.z.y").to_string())
        .show(ui, |pu| {
            pu.line(line);
            if let Some(r) = ref_l {
                pu.line(r);
            }
        });
    Some(resp.response.rect)
}

fn phase(ui: &mut Ui, curves: &Curves, ref_c: Option<&Curves>) -> Option<egui::Rect> {
    let line = Line::new(
        t!("plot.phase").to_string(),
        log_points(&curves.freq, &curves.z_phase),
    )
    .color(colors::PHASE)
    .width(1.8_f32);
    let ref_l = ref_c.map(|rc| ref_line(&t!("plot.ref"), &rc.freq, &rc.z_phase));
    let resp = base_plot("phase")
        .y_axis_label(t!("plot.phase.y").to_string())
        .show(ui, |pu| {
            pu.line(line);
            if let Some(r) = ref_l {
                pu.line(r);
            }
        });
    Some(resp.response.rect)
}

fn excursion(
    ui: &mut Ui,
    app: &App,
    curves: &Curves,
    ref_c: Option<&Curves>,
) -> Option<egui::Rect> {
    let line = Line::new(
        t!("plot.excursion").to_string(),
        log_points(&curves.freq, &curves.excursion_mm),
    )
    .color(colors::EXCURSION)
    .width(1.8_f32);
    let pr_line = if app.kind == EnclosureKind::Passive {
        curves.port_disp_mm.as_ref().map(|d| {
            Line::new(t!("plot.pr_exc").to_string(), log_points(&curves.freq, d))
                .color(colors::PORT_VEL)
                .width(1.6_f32)
        })
    } else {
        None
    };
    let pr_limit = if app.kind == EnclosureKind::Passive {
        Some(
            egui_plot::HLine::new(t!("plot.pr_xmax").to_string(), app.passive.xmax_mm)
                .color(colors::DANGER)
                .width(1.0_f32)
                .style(LineStyle::dashed_dense()),
        )
    } else {
        None
    };
    let xmax = app.driver.xmax;
    let xmax_line = egui_plot::HLine::new(t!("plot.xmax_line").to_string(), xmax)
        .color(colors::DANGER)
        .width(1.2_f32)
        .style(LineStyle::dashed_dense());
    let ref_l = ref_c.map(|rc| ref_line(&t!("plot.ref"), &rc.freq, &rc.excursion_mm));
    let markers = tuning_markers(app);
    let resp = base_plot("excursion")
        .y_axis_label(t!("plot.exc.y").to_string())
        .show(ui, |pu| {
            pu.line(line);
            if let Some(pr) = pr_line {
                pu.line(pr);
            }
            if let Some(r) = ref_l {
                pu.line(r);
            }
            pu.hline(xmax_line);
            if let Some(l) = pr_limit {
                pu.hline(l);
            }
            for m in markers {
                pu.vline(m);
            }
        });
    Some(resp.response.rect)
}

fn port_vel(ui: &mut Ui, curves: &Curves) -> Option<egui::Rect> {
    let Some(vel) = &curves.port_vel_m_s else {
        ui.label(t!("enc.vented.port_none").to_string());
        return None;
    };
    let line = Line::new(
        t!("plot.portvel").to_string(),
        log_points(&curves.freq, vel),
    )
    .color(colors::PORT_VEL)
    .width(1.8_f32);
    let caution = egui_plot::HLine::new(t!("plot.vel.caution").to_string(), 17.0)
        .color(colors::WARNING)
        .width(1.0_f32)
        .style(LineStyle::dashed_dense());
    let limit = egui_plot::HLine::new(t!("plot.vel.excessive").to_string(), 22.0)
        .color(colors::DANGER)
        .width(1.0_f32)
        .style(LineStyle::dashed_dense());
    let resp = base_plot("port_vel")
        .y_axis_label(t!("plot.vel.y").to_string())
        .show(ui, |pu| {
            pu.line(line);
            pu.hline(caution);
            pu.hline(limit);
        });
    Some(resp.response.rect)
}

fn group_delay(ui: &mut Ui, curves: &Curves, ref_c: Option<&Curves>) -> Option<egui::Rect> {
    let line = Line::new(
        t!("plot.groupdelay").to_string(),
        log_points(&curves.freq, &curves.group_delay_ms),
    )
    .color(colors::GROUP_DELAY)
    .width(1.8_f32);
    let ref_l = ref_c.map(|rc| ref_line(&t!("plot.ref"), &rc.freq, &rc.group_delay_ms));
    let resp = base_plot("group_delay")
        .y_axis_label(t!("plot.gd.y").to_string())
        .show(ui, |pu| {
            pu.line(line);
            if let Some(r) = ref_l {
                pu.line(r);
            }
        });
    Some(resp.response.rect)
}
