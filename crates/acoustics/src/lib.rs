//! Ядро акустических расчётов SpeakerLab.
//!
//! Крейт не зависит от UI: чистые модели и математика.
//! Все расчёты ведутся в СИ; структуры данных хранят
//! «инженерные» единицы из даташитов (литры, см², мГн, мм).

pub mod bandpass;
pub mod boxdim;
pub mod circuit;
pub mod driver;
pub mod line;
pub mod passive;
pub mod port;
pub mod response;
pub mod sealed;
pub mod suggest;
pub mod vented;

/// Плотность воздуха при 20 °C, кг/м³
pub const AIR_DENSITY: f64 = 1.204;
/// Скорость звука при 20 °C, м/с
pub const SPEED_OF_SOUND: f64 = 343.0;
/// Порог слышимости, Па (2·10⁻⁵)
pub const P_REF: f64 = 2.0e-5;
/// 2π
pub const TAU: f64 = std::f64::consts::TAU;

/// Параллельное соединение двух комплексных сопротивлений.
#[inline]
pub fn parallel(a: num_complex::Complex64, b: num_complex::Complex64) -> num_complex::Complex64 {
    let s = a + b;
    if s.norm_sqr() < 1e-30 {
        num_complex::Complex64::new(0.0, 0.0)
    } else {
        a * b / s
    }
}

/// Акустическая ёмкость объёма воздуха V [м³]: C = V/(ρ·c²), м⁵/Н.
#[inline]
pub fn air_compliance(volume_m3: f64) -> f64 {
    volume_m3 / (AIR_DENSITY * SPEED_OF_SOUND * SPEED_OF_SOUND)
}
