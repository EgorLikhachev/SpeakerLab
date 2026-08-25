//! Фазоинвертор (модель 4-го порядка по Смоллу).

use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

use crate::circuit::EnclosureModel;
use crate::driver::Driver;
use crate::port::PortSpec;
use crate::{air_compliance, parallel};

/// Потери порта приняты почти нулевыми (Qp ≈ 10), как в классической модели.
const Q_PORT: f64 = 10.0;

/// Ящик с фазоинвертором.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VentedBox {
    /// Внутренний объём, л
    pub vb: f64,
    /// Частота настройки порта, Гц
    pub fb: f64,
    /// Добротность потерь ящика (утечки, абсорбция)
    pub ql: f64,
    /// Геометрия порта (нужна для скорости воздуха; настройка — по Fb)
    pub port: Option<PortSpec>,
}

impl Default for VentedBox {
    fn default() -> Self {
        Self {
            vb: 55.0,
            fb: 34.0,
            ql: 10.0,
            port: None,
        }
    }
}

impl VentedBox {
    /// Акустическая ёмкость ящика, м⁵/Н.
    pub fn cab(&self) -> f64 {
        air_compliance(self.vb / 1.0e3)
    }

    /// Акустическая масса порта, приведённая к настройке Fb, кг/м⁴.
    pub fn port_mass(&self) -> f64 {
        let wb = TAU * self.fb;
        1.0 / (wb * wb * self.cab())
    }

    fn port_branch_impedance(&self, omega: f64) -> Complex64 {
        let m = self.port_mass();
        let r_ap = TAU * self.fb * m / Q_PORT;
        Complex64::new(r_ap, omega * m)
    }

    /// Частота настройки, которая получится при данной геометрии порта.
    pub fn fb_from_port_length(&self, length_m: f64, end_correction: f64, spec: &PortSpec) -> f64 {
        let a_total = spec.area_total_m2();
        if a_total <= 0.0 {
            return self.fb;
        }
        let d_eff = spec.hydraulic_diameter_m();
        let l_eff = length_m + end_correction * d_eff;
        let m_ap = crate::AIR_DENSITY * l_eff / a_total;
        1.0 / (TAU * (m_ap * self.cab()).sqrt())
    }
}

impl EnclosureModel for VentedBox {
    fn acoustic_load(&self, _driver: &Driver, omega: f64) -> Complex64 {
        let cab = self.cab();
        let wb = TAU * self.fb;

        // Ветвь ящика: ёмкость с потерями (Ql) параллельно.
        let r_al = self.ql / (wb * cab);
        let z_c = parallel(
            Complex64::new(r_al, 0.0),
            Complex64::new(0.0, -1.0 / (omega * cab)),
        );
        // Ветвь порта: масса + потери, параллельно ветви ящика.
        parallel(z_c, self.port_branch_impedance(omega))
    }

    fn radiated_velocity(
        &self,
        _driver: &Driver,
        p_node: Complex64,
        omega: f64,
        u_diaphragm: Complex64,
    ) -> Complex64 {
        // v > 0 — диффузор движется В ящик: фронтальная сторона излучает −U_d,
        // а порт под положительным давлением выкачивает +U_p.
        let z_p = self.port_branch_impedance(omega);
        let u_p = p_node / z_p;
        u_p - u_diaphragm
    }

    fn port_flow(
        &self,
        _driver: &Driver,
        p_node: Complex64,
        omega: f64,
    ) -> Option<(Complex64, f64)> {
        let spec = self.port.as_ref()?;
        let z_p = self.port_branch_impedance(omega);
        Some((p_node / z_p, spec.area_one_m2()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::solve_point;

    #[test]
    fn impedance_min_and_excursion_dip_at_fb() {
        let d = Driver::default();
        let b = VentedBox {
            vb: 50.0,
            fb: 35.0,
            ..Default::default()
        };

        // Ищем ЛОКАЛЬНЫЕ экстремумы в окне вокруг Fb: глобальный минимум |Z|
        // может лежать выше второго пика (межпиковая яма ~Re), а минимум
        // экскурсии — на ВЧ, где ход всегда мал.
        let (lo, hi) = (0.75 * b.fb, 1.25 * b.fb);
        let mut best_z = (0.0, f64::INFINITY);
        let mut best_exc = (0.0, f64::INFINITY);
        let mut peaks: Vec<(f64, f64)> = Vec::new();
        let mut prev = (0.0, 0.0);
        let mut prev2 = (0.0, 0.0);
        for k in 0..=4000 {
            let f = 15.0 + k as f64 * 0.05; // 15..215 Гц
            let omega = TAU * f;
            let za = b.acoustic_load(&d, omega);
            let op = solve_point(&d, za, omega, 2.83);
            let z = op.z_in.norm();
            let exc = (op.cone_velocity.norm() / omega) * 1.0e3;
            if (lo..=hi).contains(&f) {
                if z < best_z.1 {
                    best_z = (f, z);
                }
                if exc < best_exc.1 {
                    best_exc = (f, exc);
                }
            }
            // локальные максимумы |Z|
            if prev2.1 > 0.0 && prev.1 > prev2.1 && prev.1 >= z && f > b.fb * 0.5 {
                peaks.push(prev);
            }
            prev2 = prev;
            prev = (f, z);
        }
        assert!(
            (best_z.0 - b.fb).abs() / b.fb < 0.02,
            "минимум |Z| на {:.1}, а Fb = 35",
            best_z.0
        );
        assert!(
            (best_exc.0 - b.fb).abs() / b.fb < 0.03,
            "минимум экскурсии на {:.1}, а Fb = 35",
            best_exc.0
        );
        // два пика импеданса по обе стороны от Fb
        let left = peaks.iter().any(|(f, _)| *f < b.fb);
        let right = peaks.iter().any(|(f, _)| *f > b.fb);
        assert!(left && right, "пики: {peaks:?}");
    }

    #[test]
    fn port_dominates_radiation_at_fb() {
        let d = Driver::default();
        let b = VentedBox {
            vb: 50.0,
            fb: 35.0,
            ..Default::default()
        };
        let omega = TAU * b.fb;
        let za = b.acoustic_load(&d, omega);
        let op = solve_point(&d, za, omega, 2.83);
        let p_node = op.u_diaphragm * za;
        let z_p = b.port_branch_impedance(omega);
        let u_p = (p_node / z_p).norm();
        assert!(
            u_p > 3.0 * op.u_diaphragm.norm(),
            "на Fb порт должен качать больше диффузора: |Up|={u_p:.3e}, |Ud|={:.3e}",
            op.u_diaphragm.norm()
        );
    }

    #[test]
    fn below_fb_excursion_grows() {
        let d = Driver::default();
        let b = VentedBox {
            vb: 50.0,
            fb: 35.0,
            ..Default::default()
        };
        let exc = |f: f64| {
            let omega = TAU * f;
            let za = b.acoustic_load(&d, omega);
            solve_point(&d, za, omega, 2.83).cone_velocity.norm() / omega * 1.0e3
        };
        assert!(
            exc(15.0) > 2.0 * exc(b.fb),
            "ниже Fb экскурсия должна расти"
        );
    }
}
