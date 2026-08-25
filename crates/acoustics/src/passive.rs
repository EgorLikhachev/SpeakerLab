//! Пассивный излучатель (ПИ).

use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

use crate::circuit::EnclosureModel;
use crate::driver::Driver;
use crate::{air_compliance, parallel};

/// Добротность потерь самого ПИ (подвес + крепления).
const Q_PR: f64 = 5.0;

/// Ящик с пассивным излучателем.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PassiveBox {
    /// Внутренний объём, л
    pub vb: f64,
    /// Масса подвижной системы ПИ, г
    pub mass_g: f64,
    /// Площадь ПИ, см²
    pub sd_cm2: f64,
    /// Собственный резонанс ПИ (без ящика), Гц
    pub fs_pr: f64,
    /// Добротность потерь ящика
    pub ql: f64,
}

impl Default for PassiveBox {
    fn default() -> Self {
        Self {
            vb: 55.0,
            mass_g: 120.0,
            sd_cm2: 220.0,
            fs_pr: 22.0,
            ql: 10.0,
        }
    }
}

impl PassiveBox {
    fn sd_m2(&self) -> f64 {
        self.sd_cm2 / 1.0e4
    }

    /// Акустическая масса ПИ, кг/м⁴.
    fn mass_acoustic(&self) -> f64 {
        let sd = self.sd_m2();
        (self.mass_g / 1.0e3) / (sd * sd)
    }

    /// Акустическая податливость подвеса ПИ из его Fs.
    fn compliance_acoustic(&self) -> f64 {
        let w = TAU * self.fs_pr.max(1.0);
        1.0 / (w * w * self.mass_acoustic())
    }

    /// Фактическая частота настройки системы ПИ, Гц.
    /// Замкнутая форма: ω² = ω_pr² + 1/(M·C_ab).
    pub fn tuning_hz(&self) -> f64 {
        if self.vb <= 0.0 || self.mass_g <= 0.0 {
            return 0.0;
        }
        let cab = air_compliance(self.vb / 1.0e3);
        let w_pr = TAU * self.fs_pr.max(0.1);
        let w2 = w_pr * w_pr + 1.0 / (self.mass_acoustic() * cab);
        w2.sqrt() / TAU
    }

    /// Масса ПИ (г), дающая настройку `fb` в этом объёме.
    pub fn mass_for_tuning(&self, fb: f64) -> Option<f64> {
        if fb <= 0.0 || self.vb <= 0.0 {
            return None;
        }
        let cab = air_compliance(self.vb / 1.0e3);
        let w_pr = TAU * self.fs_pr.max(0.1);
        let w2 = (TAU * fb).powi(2) - w_pr * w_pr;
        if w2 <= 0.0 {
            return None; // настройка ниже собственного резонанса ПИ невозможна
        }
        let m_acoustic = 1.0 / (w2 * cab);
        let sd = self.sd_m2();
        Some(m_acoustic * sd * sd * 1.0e3)
    }

    fn branch_impedance(&self, omega: f64) -> Complex64 {
        let m = self.mass_acoustic();
        let c = self.compliance_acoustic();
        let r = TAU * self.fs_pr.max(1.0) * m / Q_PR;
        Complex64::new(r, omega * m - 1.0 / (omega * c))
    }
}

impl EnclosureModel for PassiveBox {
    fn acoustic_load(&self, _driver: &Driver, omega: f64) -> Complex64 {
        let cab = air_compliance(self.vb / 1.0e3);
        let wb = TAU * self.tuning_hz();
        let r_al = self.ql / (wb * cab);
        let z_c = parallel(
            Complex64::new(r_al, 0.0),
            Complex64::new(0.0, -1.0 / (omega * cab)),
        );
        parallel(z_c, self.branch_impedance(omega))
    }

    fn radiated_velocity(
        &self,
        _driver: &Driver,
        p_node: Complex64,
        omega: f64,
        u_diaphragm: Complex64,
    ) -> Complex64 {
        // Как у ФИ: фронт диффузора −U_d, ПИ излучает +U_pr
        let z_pr = self.branch_impedance(omega);
        let u_pr = p_node / z_pr;
        u_pr - u_diaphragm
    }

    fn port_flow(
        &self,
        _driver: &Driver,
        p_node: Complex64,
        omega: f64,
    ) -> Option<(Complex64, f64)> {
        let z_pr = self.branch_impedance(omega);
        Some((p_node / z_pr, self.sd_m2()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::solve_point;

    #[test]
    fn tuning_matches_impedance_dip() {
        let d = Driver::default();
        let b = PassiveBox {
            vb: 50.0,
            mass_g: 150.0,
            sd_cm2: 220.0,
            fs_pr: 20.0,
            ..Default::default()
        };
        let fb = b.tuning_hz();

        // локальный минимум |Z| между пиками ≈ tuning
        let (lo, hi) = (0.75 * fb, 1.25 * fb);
        let mut best = (0.0, f64::INFINITY);
        for k in 0..=2000 {
            let f = lo + (hi - lo) * k as f64 / 2000.0;
            let omega = TAU * f;
            let za = b.acoustic_load(&d, omega);
            let z = solve_point(&d, za, omega, 2.83).z_in.norm();
            if z < best.1 {
                best = (f, z);
            }
        }
        assert!(
            (best.0 - fb).abs() / fb < 0.08,
            "минимум |Z| на {:.1} Гц, настройка {:.1} Гц",
            best.0,
            fb
        );
    }

    #[test]
    fn mass_for_tuning_roundtrip() {
        let b = PassiveBox::default();
        let target = 30.0;
        let mass = b.mass_for_tuning(target).unwrap();
        let mut b2 = b.clone();
        b2.mass_g = mass;
        assert!((b2.tuning_hz() - target).abs() / target < 1e-6);
    }

    #[test]
    fn pr_excursion_dip_above_tuning() {
        let d = Driver::default();
        let b = PassiveBox::default();
        let fb = b.tuning_hz();
        let exc = |f: f64| {
            let omega = TAU * f;
            let za = b.acoustic_load(&d, omega);
            solve_point(&d, za, omega, 2.83).cone_velocity.norm() / omega * 1.0e3
        };
        // провал хода около настройки: ход на Fb заметно меньше, чем на 0.7·Fb
        assert!(exc(fb) < 0.8 * exc(0.7 * fb));
    }
}
