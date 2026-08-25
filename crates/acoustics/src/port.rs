//! Расчёт порта фазоинвертора (в духе Bassport).

use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_4, TAU};

use crate::{air_compliance, AIR_DENSITY};

/// Коррекция на «виртуальное удлинение» торцов порта.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EndCorrection {
    /// Один конец у стенки (бaffles), другой свободный — k ≈ 0.732 (типично)
    #[default]
    OneFlanged,
    /// Оба конца у стенок — k ≈ 0.85
    BothFlanged,
    /// Оба конца свободны — k ≈ 0.614
    BothFree,
}

impl EndCorrection {
    pub fn k(self) -> f64 {
        match self {
            EndCorrection::OneFlanged => 0.732,
            EndCorrection::BothFlanged => 0.850,
            EndCorrection::BothFree => 0.614,
        }
    }
}

/// Форма сечения порта.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "camelCase")]
pub enum PortGeometry {
    Round { diameter_mm: f64 },
    Slot { width_mm: f64, height_mm: f64 },
}

impl Default for PortGeometry {
    fn default() -> Self {
        PortGeometry::Round { diameter_mm: 70.0 }
    }
}

impl PortGeometry {
    /// Площадь одного порта, м².
    pub fn area_m2(self) -> f64 {
        match self {
            PortGeometry::Round { diameter_mm } => FRAC_PI_4 * (diameter_mm / 1.0e3).powi(2),
            PortGeometry::Slot {
                width_mm,
                height_mm,
            } => (width_mm / 1.0e3) * (height_mm / 1.0e3),
        }
    }

    /// Гидравлический диаметр (диаметр круга той же площади), м.
    pub fn hydraulic_diameter_m(self) -> f64 {
        (4.0 * self.area_m2() / std::f64::consts::PI).sqrt()
    }
}

/// Порт: геометрия + количество одинаковых отверстий.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PortSpec {
    pub geometry: PortGeometry,
    pub count: u32,
}

impl PortSpec {
    pub fn new(geometry: PortGeometry) -> Self {
        Self { geometry, count: 1 }
    }

    /// Площадь одного порта, м².
    pub fn area_one_m2(&self) -> f64 {
        self.geometry.area_m2()
    }

    /// Суммарная площадь, м².
    pub fn area_total_m2(&self) -> f64 {
        self.area_one_m2() * self.count.max(1) as f64
    }

    /// Гидравлический диаметр, м.
    pub fn hydraulic_diameter_m(&self) -> f64 {
        self.geometry.hydraulic_diameter_m()
    }
}

/// Длина порта (м), настраивающая объём `vb_l` на частоту `fb_hz`.
///
/// Из `M_ap = 1/(ω_b²·C_ab)` и `M_ap = ρ₀·(L + k·D)/A_total`:
/// `L = A_total·c²/(ω_b²·V_b) − k·D`.
pub fn port_length_m(vb_liters: f64, fb_hz: f64, spec: &PortSpec, ec: EndCorrection) -> f64 {
    let a_total = spec.area_total_m2();
    if a_total <= 0.0 || vb_liters <= 0.0 || fb_hz <= 0.0 {
        return 0.0;
    }
    let wb = TAU * fb_hz;
    let vb = vb_liters / 1.0e3;
    let l_eff = a_total * crate::SPEED_OF_SOUND * crate::SPEED_OF_SOUND / (wb * wb * vb);
    (l_eff - ec.k() * spec.hydraulic_diameter_m()).max(0.0)
}

/// Частота настройки (Гц), которую даст порт длиной `length_m` в объёме `vb_l`.
pub fn fb_for_length(vb_liters: f64, length_m: f64, spec: &PortSpec, ec: EndCorrection) -> f64 {
    let a_total = spec.area_total_m2();
    if a_total <= 0.0 || vb_liters <= 0.0 {
        return 0.0;
    }
    let l_eff = length_m + ec.k() * spec.hydraulic_diameter_m();
    if l_eff <= 0.0 {
        return 0.0;
    }
    let m_ap = AIR_DENSITY * l_eff / a_total;
    1.0 / (TAU * (m_ap * air_compliance(vb_liters / 1.0e3)).sqrt())
}

/// Оценка скорости воздуха, м/с: (уровень — см. `velocity_level`).
pub fn velocity_level(v_m_s: f64) -> VelocityLevel {
    if v_m_s < 17.0 {
        VelocityLevel::Ok
    } else if v_m_s < 22.0 {
        VelocityLevel::Caution
    } else {
        VelocityLevel::Excessive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VelocityLevel {
    Ok,
    Caution,
    Excessive,
}

/// Рекомендуемый диапазон суммарной площади порта (м²): 10–30 % от Sd.
pub fn recommended_area_range(sd_m2: f64) -> (f64, f64) {
    (0.10 * sd_m2, 0.30 * sd_m2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_roundtrip_fb() {
        let spec = PortSpec::new(PortGeometry::Round { diameter_mm: 100.0 });
        let (vb, fb) = (50.0, 35.0);
        let len = port_length_m(vb, fb, &spec, EndCorrection::OneFlanged);
        // классическая формула (см): 23562.5·D²/(Fb²·Vb) − 0.732·D ≈ 31.1 см
        assert!((len * 1.0e2 - 30.9).abs() < 1.5, "длина {len:.3} м");
        let back = fb_for_length(vb, len, &spec, EndCorrection::OneFlanged);
        assert!((back - fb).abs() / fb < 1e-6);
    }

    #[test]
    fn length_scales_with_area() {
        let small = PortSpec::new(PortGeometry::Round { diameter_mm: 50.0 });
        let big = PortSpec::new(PortGeometry::Round { diameter_mm: 100.0 });
        let l1 = port_length_m(50.0, 35.0, &small, EndCorrection::OneFlanged);
        let l2 = port_length_m(50.0, 35.0, &big, EndCorrection::OneFlanged);
        assert!(l2 > l1, "больше площадь → длиннее порт ({l2} vs {l1})");
    }

    #[test]
    fn slot_area() {
        let s = PortGeometry::Slot {
            width_mm: 200.0,
            height_mm: 50.0,
        };
        assert!((s.area_m2() - 0.01).abs() < 1e-12);
    }
}
