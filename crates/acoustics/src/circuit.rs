//! Универсальное решение электро-механо-акустической схемы динамика.
//!
//! Все модели оформлений (ЗЯ, ФИ, ПИ, бандпасс, ТЛ…) сводятся к одному
//! интерфейсу: оформить → акустический импеданс нагрузки на диффузор
//! `Z_a(ω)` (акустический домен, Па·с/м⁵) + правило суммирования
//! излучаемых объёмных скоростей.

use num_complex::Complex64;
use std::f64::consts::TAU;

use crate::driver::Driver;

/// Рабочая точка схемы на одной частоте.
#[derive(Debug, Clone, Copy)]
pub struct OperatingPoint {
    /// Круговая частота, рад/с
    pub omega: f64,
    /// Входной электрический импеданс, Ом
    pub z_in: Complex64,
    /// Скорость диффузора, м/с (комплексная амплитуда)
    pub cone_velocity: Complex64,
    /// Объёмная скорость диффузора, м³/с
    pub u_diaphragm: Complex64,
}

/// Оформление как «чёрный ящик» для решателя.
pub trait EnclosureModel {
    /// Акустический импеданс, которым оформление нагружает диффузор.
    fn acoustic_load(&self, driver: &Driver, omega: f64) -> Complex64;

    /// Суммарная излучаемая объёмная скорость при узловом давлении
    /// `p_node = U_d · Z_a` на задней стороне диффузора.
    fn radiated_velocity(
        &self,
        driver: &Driver,
        p_node: Complex64,
        omega: f64,
        u_diaphragm: Complex64,
    ) -> Complex64;

    /// Объёмная скорость и площадь (м², одного отверстия) порта/ПИ — для графиков.
    fn port_flow(
        &self,
        _driver: &Driver,
        _p_node: Complex64,
        _omega: f64,
    ) -> Option<(Complex64, f64)> {
        None
    }
}

/// Решение схемы при синусоидальном напряжении `voltage` (В, действ.).
///
/// Электрическая сторона: `e = i·(Re + jωLe) + Bl·v`;
/// механическая: `v = Bl·i / Z_m`, где
/// `Z_m = Rms + jωMms + 1/(jωCms) + Sd²·Z_a`.
pub fn solve_point(driver: &Driver, za: Complex64, omega: f64, voltage: f64) -> OperatingPoint {
    let sd = driver.sd_m2();
    let bl = driver.bl_tm();

    let ze = Complex64::new(driver.re, omega * driver.le * 1.0e-3); // Le: мГн → Гн
    let zm = Complex64::new(driver.rms(), omega * driver.mms_kg())
        + Complex64::new(0.0, -1.0 / (omega * driver.cms()))
        + za * sd * sd;

    let denom = ze * zm + bl * bl;
    let v = if denom.norm_sqr() < 1e-30 {
        Complex64::new(0.0, 0.0)
    } else {
        bl * voltage / denom
    };
    let i = v * zm / bl;
    let z_in = if i.norm_sqr() > 1e-30 {
        Complex64::new(voltage, 0.0) / i
    } else {
        ze
    };

    OperatingPoint {
        omega,
        z_in,
        cone_velocity: v,
        u_diaphragm: v * sd,
    }
}

/// Частота (Гц) из круговой.
#[inline]
pub fn hz(omega: f64) -> f64 {
    omega / TAU
}
