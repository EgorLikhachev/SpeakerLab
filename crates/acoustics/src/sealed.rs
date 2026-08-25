//! Закрытый ящик (модель 2-го порядка).

use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

use crate::circuit::EnclosureModel;
use crate::driver::{Driver, Fill};
use crate::{air_compliance, parallel};

/// Закрытый ящик.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SealedBox {
    /// Внутренний объём, л
    pub vb: f64,
    /// Добротность потерь в ящике (абсорбция/утечки)
    pub qa: f64,
    /// Заполнение демпфирующим материалом
    pub fill: Fill,
}

impl Default for SealedBox {
    fn default() -> Self {
        Self {
            vb: 30.0,
            qa: 10.0,
            fill: Fill::None,
        }
    }
}

impl SealedBox {
    /// Эффективный (с учётом заполнения) объём, м³.
    pub fn effective_vb_m3(&self) -> f64 {
        (self.vb / 1.0e3) * self.fill.volume_factor()
    }

    /// Резонансная частота системы, Гц (аналитически).
    pub fn fc(&self, driver: &Driver) -> f64 {
        let alpha = driver.vas / (self.effective_vb_m3() * 1.0e3);
        driver.fs * (1.0 + alpha).sqrt()
    }

    /// Полная добротность системы (аналитически).
    pub fn qtc(&self, driver: &Driver) -> f64 {
        let alpha = driver.vas / (self.effective_vb_m3() * 1.0e3);
        driver.qts() * (1.0 + alpha).sqrt()
    }

    /// Объём ящика (л) для целевой Qtc.
    pub fn vb_for_qtc(driver: &Driver, qtc: f64) -> Option<f64> {
        let qts = driver.qts();
        if qtc <= qts {
            return None;
        }
        let alpha = (qtc / qts).powi(2) - 1.0;
        Some(driver.vas / alpha)
    }
}

impl EnclosureModel for SealedBox {
    fn acoustic_load(&self, driver: &Driver, omega: f64) -> Complex64 {
        let cab = air_compliance(self.effective_vb_m3());
        // Потери — параллельный резистор, приведённый к резонансу системы
        // (классическое допущение Смолла о частотно-независимой добротности).
        let r_ab = self.qa / (TAU * self.fc(driver) * cab);
        let c = Complex64::new(0.0, -1.0 / (omega * cab));
        if self.qa > 1.0e6 {
            c
        } else {
            parallel(Complex64::new(r_ab, 0.0), c)
        }
    }

    fn radiated_velocity(
        &self,
        _driver: &Driver,
        _p_node: Complex64,
        _omega: f64,
        u_diaphragm: Complex64,
    ) -> Complex64 {
        // v > 0 — диффузор движется в ящик, фронт излучает с обратным знаком.
        -u_diaphragm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::solve_point;

    fn mag_z_over_freq(driver: &Driver, box_: &SealedBox) -> Vec<(f64, f64)> {
        (0..2001)
            .map(|k| {
                let f = 10.0 + k as f64 * 0.1; // 10..210 Гц
                let omega = TAU * f;
                let za = box_.acoustic_load(driver, omega);
                let op = solve_point(driver, za, omega, 2.83);
                (f, op.z_in.norm())
            })
            .collect()
    }

    #[test]
    fn fc_matches_impedance_peak() {
        let d = Driver::default();
        let b = SealedBox {
            vb: d.vas,
            ..Default::default()
        }; // α = 1 → fc = fs·√2
        let analytic = b.fc(&d);
        let pts = mag_z_over_freq(&d, &b);
        let (f_peak, _) = pts
            .iter()
            .skip(1)
            .take(1999)
            .fold(
                (0.0, 0.0),
                |best, (f, z)| {
                    if *z > best.1 {
                        (*f, *z)
                    } else {
                        best
                    }
                },
            );
        assert!(
            (f_peak - analytic).abs() / analytic < 0.02,
            "пик |Z| на {f_peak:.1} Гц, аналитика {analytic:.1} Гц"
        );
    }

    #[test]
    fn vb_for_qtc_roundtrip() {
        let d = Driver::default();
        let vb = SealedBox::vb_for_qtc(&d, 0.707).unwrap();
        let b = SealedBox {
            vb,
            ..Default::default()
        };
        assert!((b.qtc(&d) - 0.707).abs() < 1e-9);
    }
}
