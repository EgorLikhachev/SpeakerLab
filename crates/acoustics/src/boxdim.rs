//! Расчёт размеров ящика: чистый/габаритный объём, вытеснение, панели.

use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_PI_4;

/// Внутренние размеры, мм.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dims {
    pub w: f64, // ширина
    pub h: f64, // высота
    pub d: f64, // глубина
}

/// Пропорции сторон (к высоте).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ratio {
    Golden, // 0.62 : 1 : 1.62
    Cube,   // 0.85 : 1 : 1.15
    Tower,  // 0.55 : 1 : 1.1
    Square, // 1 : 1 : 1
}

impl Ratio {
    pub fn factors(self) -> (f64, f64, f64) {
        match self {
            Ratio::Golden => (0.62, 1.0, 1.62),
            Ratio::Cube => (0.85, 1.0, 1.15),
            Ratio::Tower => (0.55, 1.0, 1.1),
            Ratio::Square => (1.0, 1.0, 1.0),
        }
    }
}

/// Оценка вытеснения динамика (л) по Sd: конус+корзина ≈ половина цилиндра
/// глубиной D_eff.
pub fn driver_displacement_l(sd_cm2: f64) -> f64 {
    let d_eff_cm = (4.0 * sd_cm2 / std::f64::consts::PI).sqrt();
    0.5 * FRAC_PI_4 * d_eff_cm * d_eff_cm * d_eff_cm / 1000.0
}

/// Вытеснение порта (л): труба/щель + стены щелевого порта.
pub fn port_displacement_l(area_total_m2: f64, length_m: f64, slot_wall_mm: f64) -> f64 {
    // объём прохода + удвоенная площадь стен × длина (для щели)
    let channel = area_total_m2 * length_m * 1000.0;
    let walls = if slot_wall_mm > 0.0 && area_total_m2 > 0.0 {
        // периметр ≈ 2·(W+H); оценим из площади при пропорции 4:1
        let h = (area_total_m2 / 4.0).sqrt();
        let w = 4.0 * h;
        2.0 * (w + h) * (slot_wall_mm / 1000.0) * length_m * 1000.0
    } else {
        0.0
    };
    channel + walls
}

/// Входные данные расчёта размеров.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BoxCalc {
    /// Целевой ЧИСТЫЙ объём, л
    pub net_volume: f64,
    /// Вытеснение динамика, л
    pub driver_l: f64,
    /// Вытеснение порта, л
    pub port_l: f64,
    /// Распорки, л
    pub brace_l: f64,
    /// Толщина стенок, мм
    pub wall_mm: f64,
}

impl Default for BoxCalc {
    fn default() -> Self {
        Self {
            net_volume: 55.0,
            driver_l: 2.0,
            port_l: 2.5,
            brace_l: 0.5,
            wall_mm: 18.0,
        }
    }
}

impl BoxCalc {
    /// Габаритный внутренний объём, л.
    pub fn gross_volume(&self) -> f64 {
        (self.net_volume + self.driver_l + self.port_l + self.brace_l).max(0.0)
    }

    /// Размеры по пропорции: высота из объёма.
    pub fn dims_by_ratio(&self, ratio: Ratio) -> Dims {
        let (fw, fh, fd) = ratio.factors();
        let v_mm3 = self.gross_volume() * 1.0e6;
        let h = (v_mm3 / (fw * fh * fd)).cbrt();
        Dims {
            w: fw * h,
            h: fh * h,
            d: fd * h,
        }
    }

    /// Глубина при заданных ширине/высоте (внутренние, мм).
    pub fn depth_for(&self, w_mm: f64, h_mm: f64) -> f64 {
        if w_mm <= 0.0 || h_mm <= 0.0 {
            return 0.0;
        }
        self.gross_volume() * 1.0e6 / (w_mm * h_mm)
    }

    /// Внешние размеры, мм.
    pub fn external(&self, d: Dims) -> Dims {
        let t = self.wall_mm;
        Dims {
            w: d.w + 2.0 * t,
            h: d.h + 2.0 * t,
            d: d.d + 2.0 * t,
        }
    }

    /// Фактический чистый объём при данных внутренних размерах, л.
    pub fn net_for_dims(&self, d: Dims) -> f64 {
        d.w * d.h * d.d / 1.0e6 - self.driver_l - self.port_l - self.brace_l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_volume_roundtrip() {
        let c = BoxCalc {
            net_volume: 50.0,
            driver_l: 2.0,
            port_l: 2.0,
            brace_l: 1.0,
            wall_mm: 18.0,
        };
        let dims = c.dims_by_ratio(Ratio::Golden);
        let vol = dims.w * dims.h * dims.d / 1.0e6;
        assert!((vol - c.gross_volume()).abs() < 1e-6);
        assert!((c.net_for_dims(dims) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn depth_solves() {
        let c = BoxCalc {
            net_volume: 50.0,
            driver_l: 0.0,
            port_l: 0.0,
            brace_l: 0.0,
            wall_mm: 18.0,
        };
        let d = c.depth_for(300.0, 400.0);
        assert!((300.0 * 400.0 * d / 1.0e6 - 50.0).abs() < 1e-6);
    }

    #[test]
    fn driver_disp_reasonable() {
        // 8" (Sd=220 см²): ожидаем 1.5–2.5 л
        let v = driver_displacement_l(220.0);
        assert!(v > 1.0 && v < 3.0, "{v}");
    }
}
