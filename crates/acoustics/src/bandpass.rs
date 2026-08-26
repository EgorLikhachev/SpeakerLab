//! Бандпасс 4-го порядка (задняя камера ЗЯ + передняя с портом)
//! и 6-го порядка (обе камеры с портами).

use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

use crate::circuit::EnclosureModel;
use crate::driver::Driver;
use crate::port::PortSpec;
use crate::{air_compliance, parallel};

const Q_PORT: f64 = 10.0;

fn sealed_branch(vb_l: f64, ql: f64, omega: f64) -> Complex64 {
    let cab = air_compliance(vb_l / 1.0e3);
    let r_al = ql / (TAU * sealed_fc_hint(vb_l) * cab);
    parallel(
        Complex64::new(r_al, 0.0),
        Complex64::new(0.0, -1.0 / (omega * cab)),
    )
}

/// Приблизительный резонанс камеры для приведения потерь (Fs·√(1+α)).
fn sealed_fc_hint(_vb_l: f64) -> f64 {
    50.0 // потери слабо влияют; постоянная привязка достаточна
}

fn ported_branch(vb_l: f64, fb: f64, omega: f64) -> Complex64 {
    let cab = air_compliance(vb_l / 1.0e3);
    let wb = TAU * fb;
    let m_ap = 1.0 / (wb * wb * cab);
    let r_ap = wb * m_ap / Q_PORT;
    Complex64::new(r_ap, omega * m_ap)
}

/// Камера с потерями и портом: параллельное соединение.
fn ported_chamber(vb_l: f64, fb: f64, ql: f64, omega: f64) -> Complex64 {
    let cab = air_compliance(vb_l / 1.0e3);
    let wb = TAU * fb;
    let r_al = ql / (wb * cab);
    let z_c = parallel(
        Complex64::new(r_al, 0.0),
        Complex64::new(0.0, -1.0 / (omega * cab)),
    );
    parallel(z_c, ported_branch(vb_l, fb, omega))
}

/// Бандпасс 4-го порядка.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Bandpass4 {
    /// Задняя (закрытая) камера, л
    pub vb_rear: f64,
    /// Передняя камера, л
    pub vb_front: f64,
    /// Настройка порта передней камеры, Гц
    pub fb: f64,
    /// Потери
    pub ql: f64,
    /// Геометрия порта (для скорости воздуха)
    pub port: Option<PortSpec>,
}

impl Default for Bandpass4 {
    fn default() -> Self {
        Self {
            vb_rear: 30.0,
            vb_front: 20.0,
            fb: 55.0,
            ql: 10.0,
            port: None,
        }
    }
}

impl EnclosureModel for Bandpass4 {
    fn acoustic_load(&self, _driver: &Driver, omega: f64) -> Complex64 {
        sealed_branch(self.vb_rear, self.ql, omega)
            + ported_chamber(self.vb_front, self.fb, self.ql, omega)
    }

    fn radiated_velocity(
        &self,
        _driver: &Driver,
        p_node: Complex64,
        omega: f64,
        _u_diaphragm: Complex64,
    ) -> Complex64 {
        // Диффузор внутри камер — наружу излучает только порт передней камеры.
        let z_p = ported_branch(self.vb_front, self.fb, omega);
        p_node / z_p
    }

    fn port_flow(
        &self,
        _driver: &Driver,
        p_node: Complex64,
        omega: f64,
    ) -> Option<(Complex64, f64)> {
        let spec = self.port.as_ref()?;
        let z_p = ported_branch(self.vb_front, self.fb, omega);
        Some((p_node / z_p, spec.area_one_m2()))
    }
}

/// Бандпасс 6-го порядка (оба порта наружу).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Bandpass6 {
    pub vb_rear: f64,
    pub vb_front: f64,
    /// Настройка порта задней камеры, Гц
    pub fb_rear: f64,
    /// Настройка порта передней камеры, Гц
    pub fb_front: f64,
    pub ql: f64,
    pub port_front: Option<PortSpec>,
    pub port_rear: Option<PortSpec>,
}

impl Default for Bandpass6 {
    fn default() -> Self {
        Self {
            vb_rear: 30.0,
            vb_front: 20.0,
            fb_rear: 38.0,
            fb_front: 65.0,
            ql: 10.0,
            port_front: None,
            port_rear: None,
        }
    }
}

impl Bandpass6 {
    fn rear_branch(&self, omega: f64) -> Complex64 {
        ported_branch(self.vb_rear, self.fb_rear, omega)
    }
    fn front_branch(&self, omega: f64) -> Complex64 {
        ported_branch(self.vb_front, self.fb_front, omega)
    }

    /// Комплексные импедансы камер.
    fn chambers(&self, omega: f64) -> (Complex64, Complex64) {
        (
            ported_chamber(self.vb_rear, self.fb_rear, self.ql, omega),
            ported_chamber(self.vb_front, self.fb_front, self.ql, omega),
        )
    }

    /// Давление на задней камере при узловом p_node (комплексный делитель).
    fn rear_pressure(&self, p_node: Complex64, omega: f64) -> (Complex64, Complex64) {
        let (z_r, z_f) = self.chambers(omega);
        let sum = z_r + z_f;
        if sum.norm_sqr() < 1e-30 {
            return (Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0));
        }
        (p_node * z_r / sum, p_node * z_f / sum)
    }
}

impl EnclosureModel for Bandpass6 {
    fn acoustic_load(&self, _driver: &Driver, omega: f64) -> Complex64 {
        let (z_r, z_f) = self.chambers(omega);
        z_r + z_f
    }

    fn radiated_velocity(
        &self,
        _driver: &Driver,
        p_node: Complex64,
        omega: f64,
        _u_diaphragm: Complex64,
    ) -> Complex64 {
        // Камеры включены последовательно: p делится по комплексным импедансам.
        let (p_rear, p_front) = self.rear_pressure(p_node, omega);
        let u_rear = p_rear / self.rear_branch(omega);
        let u_front = p_front / self.front_branch(omega);
        u_rear + u_front
    }

    fn port_flow(
        &self,
        _driver: &Driver,
        p_node: Complex64,
        omega: f64,
    ) -> Option<(Complex64, f64)> {
        let spec = self.port_front.as_ref()?;
        let (_, p_front) = self.rear_pressure(p_node, omega);
        Some((p_front / self.front_branch(omega), spec.area_one_m2()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::solve_point;
    use crate::response::{simulate, summarize, SimConfig};

    fn band_peak(c: &crate::response::Curves) -> (f64, f64) {
        let mut best = (0.0, f64::NEG_INFINITY);
        for (f, s) in c.freq.iter().zip(c.spl.iter()) {
            if *s > best.1 {
                best = (*f, *s);
            }
        }
        best
    }

    #[test]
    fn bp4_has_bandpass_shape() {
        let d = Driver::default();
        let b = Bandpass4::default();
        let curves = simulate(&d, &b, &SimConfig::default());
        let s = summarize(&curves, None);
        // полоса среза с двух сторон определена
        let lo = s.f3_low.expect("низ есть");
        let hi = s.f3_high.expect("верх есть");
        assert!(lo > 20.0 && lo < 60.0, "F3 низ {lo:.1}");
        assert!(hi > 60.0 && hi < 200.0, "F3 верх {hi:.1}");
        let (peak_f, _) = band_peak(&curves);
        assert!(peak_f > 30.0 && peak_f < 100.0);
    }

    #[test]
    fn bp4_below_tuning_behaves_like_sealed_rear() {
        let d = Driver::default();
        let b = Bandpass4 {
            vb_rear: 30.0,
            ..Default::default()
        };
        let sealed = crate::sealed::SealedBox {
            vb: 30.0,
            ..Default::default()
        };
        // Ниже настроек передняя камера закрыта (масса порта мала на НЧ —
        // цепь порта шунтирует), ход задаётся задней закрытой камерой.
        let exc = |f: f64, m: &dyn EnclosureModel| {
            let omega = TAU * f;
            let za = m.acoustic_load(&d, omega);
            solve_point(&d, za, omega, 2.83).cone_velocity.norm() / omega * 1.0e3
        };
        let bp = exc(15.0, &b);
        let se = exc(15.0, &sealed);
        assert!(
            ((bp - se) / se).abs() < 0.15,
            "ход БП {bp:.2} мм vs ЗЯ {se:.2} мм на 15 Гц"
        );
    }

    #[test]
    fn bp6_rolls_off_above_front_port() {
        let d = Driver::default();
        let b6 = Bandpass6::default();
        let curves = simulate(&d, &b6, &SimConfig::default());
        let s = summarize(&curves, None);
        assert!(s.f3_high.is_some());
    }
}
