//! Нижняя панель: сводка результатов и предупреждения.

use egui::Ui;
use rust_i18n::t;
use speakerlab_acoustics::port::velocity_level;

use crate::state::{App, EnclosureKind};
use crate::ui::util::{colors, fnum, uv};

pub fn show(ui: &mut Ui, app: &App) {
    ui.horizontal_wrapped(|ui| {
        let Some(s) = &app.summary else {
            ui.weak(t!("sim.no_data").to_string());
            return;
        };

        chip(ui, &t!("sum.peak"), uv(fnum(s.peak_spl), "дБ"));
        if let Some(f3) = s.f3_low {
            chip(ui, &t!("sum.f3"), uv(fnum(f3), "Гц"));
        }
        if let Some(f3h) = s.f3_high {
            chip(ui, &t!("sum.f3h"), uv(fnum(f3h), "Гц"));
        }
        match app.kind {
            EnclosureKind::Sealed => {
                chip(
                    ui,
                    &t!("sum.fc"),
                    uv(fnum(app.sealed.fc(&app.driver)), "Гц"),
                );
                chip(ui, &t!("sum.qtc"), fnum(app.sealed.qtc(&app.driver)));
            }
            EnclosureKind::Vented => {
                chip(ui, &t!("sum.fb"), uv(fnum(app.vented.fb), "Гц"));
                chip(ui, &t!("sum.alpha"), fnum(app.driver.vas / app.vented.vb));
            }
            EnclosureKind::Passive => {
                chip(ui, &t!("sum.fb"), uv(fnum(app.passive.tuning_hz()), "Гц"));
                chip(ui, &t!("sum.alpha"), fnum(app.driver.vas / app.passive.vb));
            }
            EnclosureKind::Bandpass4 => {
                chip(ui, &t!("sum.fb"), uv(fnum(app.bp4.fb), "Гц"));
            }
            EnclosureKind::Bandpass6 => {
                chip(ui, &t!("sum.fb"), uv(fnum(app.bp6.fb_rear), "Гц"));
                chip(ui, &t!("sum.fb2"), uv(fnum(app.bp6.fb_front), "Гц"));
            }
            EnclosureKind::Line => {
                chip(
                    ui,
                    &t!("sum.qw"),
                    uv(fnum(app.line.quarter_wave_hz()), "Гц"),
                );
                chip(ui, &t!("sum.volume"), uv(fnum(app.line.volume_l()), "л"));
            }
        }
        chip(
            ui,
            &t!("sum.zmax"),
            format!(
                "{} @ {}",
                uv(fnum(s.z_max), "Ом"),
                uv(fnum(s.z_max_freq), "Гц")
            ),
        );
        chip(
            ui,
            &t!("sum.exc_max"),
            format!(
                "{} @ {}",
                uv(fnum(s.excursion_max_mm), "мм"),
                uv(fnum(s.excursion_max_freq), "Гц")
            ),
        );
        if let Some(efb) = s.excursion_at_tuning {
            chip(ui, &t!("sum.exc_at_fb"), uv(fnum(efb), "мм"));
        }
        if let Some(v) = s.port_vel_max_m_s {
            chip(
                ui,
                &t!("sum.vel_max"),
                format!(
                    "{} @ {}",
                    uv(fnum(v), "м/с"),
                    uv(fnum(s.port_vel_max_freq), "Гц")
                ),
            );
        }
        // Предельное напряжение и мощность на нём
        if let Some(lim) = &app.limits {
            if let Some(v_limit) = lim.v_limit {
                let kind = limit_kind_key(lim.limiting);
                let p_limit = v_limit * v_limit / s.z_min.max(0.1);
                let detail = match lim.limiting {
                    speakerlab_acoustics::response::LimitKind::Xmax => lim
                        .v_xmax
                        .map(|(_, f)| format!(" @ {}", uv(fnum(f), "Гц")))
                        .unwrap_or_default(),
                    speakerlab_acoustics::response::LimitKind::Port => lim
                        .v_port
                        .map(|(_, f)| format!(" @ {} {}", fnum(f), t!("unit.hz")))
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                chip(
                    ui,
                    &t!("sum.vlimit"),
                    format!(
                        "{} ({}) — {}{detail}",
                        uv(fnum(v_limit), "В"),
                        uv(format!("{p_limit:.0}"), "Вт"),
                        t!(kind)
                    ),
                );
            }
        }
        // Приблизительная мощность на текущем напряжении
        let p = app.sim.voltage * app.sim.voltage / s.z_min.max(0.1);
        chip(ui, &t!("sum.power"), format!("≈{}", uv(fnum(p), "Вт")));
    });

    // Предупреждения
    ui.horizontal_wrapped(|ui| {
        let Some(s) = &app.summary else { return };
        if s.excursion_max_mm > app.driver.xmax {
            ui.colored_label(
                colors::DANGER,
                t!(
                    "warn.xmax",
                    value = fnum(s.excursion_max_mm),
                    limit = fnum(app.driver.xmax),
                    freq = fnum(s.excursion_max_freq)
                )
                .to_string(),
            );
        }
        // Текущее напряжение выше предельного?
        if let (Some(lim), Some(v_limit)) = (&app.limits, app.limits.and_then(|l| l.v_limit)) {
            if app.sim.voltage > v_limit {
                ui.colored_label(
                    colors::DANGER,
                    t!(
                        "warn.overlimit",
                        v = fnum(app.sim.voltage),
                        limit = fnum(v_limit),
                        kind = t!(limit_kind_key(lim.limiting))
                    )
                    .to_string(),
                );
            }
        }
        if let Some(v) = s.port_vel_max_m_s {
            let (color, key) = match velocity_level(v) {
                speakerlab_acoustics::port::VelocityLevel::Ok => (colors::EXCURSION, ""),
                speakerlab_acoustics::port::VelocityLevel::Caution => {
                    (colors::WARNING, "warn.vel.caution")
                }
                speakerlab_acoustics::port::VelocityLevel::Excessive => {
                    (colors::DANGER, "warn.vel.excessive")
                }
            };
            if !key.is_empty() {
                ui.colored_label(color, t!(key, v = fnum(v)).to_string());
            }
        }
    });
}

fn limit_kind_key(kind: speakerlab_acoustics::response::LimitKind) -> &'static str {
    use speakerlab_acoustics::response::LimitKind;
    match kind {
        LimitKind::None => "limit.none",
        LimitKind::Xmax => "limit.xmax",
        LimitKind::Port => "limit.port",
        LimitKind::Thermal => "limit.thermal",
    }
}

fn chip(ui: &mut Ui, label: &str, value: String) {
    ui.label(egui::RichText::new(label).weak());
    ui.label(egui::RichText::new(value).strong());
    ui.add_space(8.0);
}
