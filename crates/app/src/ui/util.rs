//! Мелкие помощники для UI.

use egui::{DragValue, Ui};
use rust_i18n::t;

/// Число с разумным числом знаков.
pub fn fnum(v: f64) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    let a = v.abs();
    if a >= 1000.0 {
        format!("{v:.0}")
    } else if a >= 1.0 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

/// Подпись частоты: 20, 100, 1k, 2.5k
pub fn fmt_hz(f: f64) -> String {
    if f < 1000.0 {
        format!("{f:.0}")
    } else {
        let k = f / 1000.0;
        if (k - k.round()).abs() < 1e-9 {
            format!("{:.0}k", k)
        } else {
            format!("{k:.2}k")
        }
    }
}

/// Строка ввода числа с меткой (в Grid); true, если значение изменилось.
pub fn num_field(
    ui: &mut Ui,
    label: &str,
    v: &mut f64,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
    unit: &str,
) -> bool {
    ui.label(label);
    let resp = ui.add(
        DragValue::new(v)
            .speed(speed)
            .range(range)
            .suffix(unit)
            .custom_formatter(|n, _| fnum(n)),
    );
    ui.end_row();
    resp.changed()
}

/// Заголовок секции.
pub fn section(ui: &mut Ui, key: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(t!(key).to_string()).strong());
    ui.add_space(2.0);
}

/// Цвета кривых.
pub mod colors {
    use egui::Color32;
    pub const SPL: Color32 = Color32::from_rgb(96, 165, 250);
    pub const Z: Color32 = Color32::from_rgb(250, 163, 111);
    pub const PHASE: Color32 = Color32::from_rgb(167, 139, 250);
    pub const EXCURSION: Color32 = Color32::from_rgb(74, 222, 128);
    pub const PORT_VEL: Color32 = Color32::from_rgb(34, 211, 238);
    pub const GROUP_DELAY: Color32 = Color32::from_rgb(244, 114, 182);
    pub const DANGER: Color32 = Color32::from_rgb(240, 90, 90);
    pub const WARNING: Color32 = Color32::from_rgb(235, 190, 80);
    pub const HINT: Color32 = Color32::from_rgb(110, 200, 255);
}
