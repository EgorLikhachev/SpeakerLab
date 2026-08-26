//! T/S-параметры динамика, производные величины и валидация.

use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

use crate::air_compliance;

/// Модель индуктивности звуковой катушки.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LeModel {
    /// Простая: Z = Re + jωLe
    #[default]
    Simple,
    /// Полуиндуктивность (LR-2, модель Райта/Вандеркуя):
    /// Z = Re + Kes·√(jω) — реальные катушки имеют наклон |Z| +3 дБ/окт,
    /// а не +6 дБ/окт идеальной индуктивности.
    Semi,
}

/// Поле T/S-параметра — для локализуемых сообщений валидации.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DriverField {
    Re,
    Le,
    Fs,
    Qms,
    Qes,
    Vas,
    Sd,
    Xmax,
}

/// Структурированная проблема во вводе (сообщение строит UI через i18n).
#[derive(Debug, Clone, PartialEq)]
pub enum DriverIssue {
    /// Значение должно быть больше нуля.
    NonPositive(DriverField),
    /// Qts > 0.6 — пригоден скорее для ЗЯ; ФИ потребует большого ящика.
    QtsHigh,
    /// Qts < 0.25 — пригоден скорее для ФИ/рупора.
    QtsLow,
    /// EBP = Fs/Qes.
    Ebp { value: f64 },
}

/// Уровень заполнения ЗЯ: влияние на эффективный объём и потери.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Fill {
    #[default]
    None,
    Light,
    Medium,
    Heavy,
}

impl Fill {
    /// Множитель эффективного объёма ящика.
    pub fn volume_factor(self) -> f64 {
        match self {
            Fill::None => 1.00,
            Fill::Light => 1.08,
            Fill::Medium => 1.22,
            Fill::Heavy => 1.40,
        }
    }
}

/// T/S-параметры динамика.
/// Единицы хранения — как в даташитах:
/// Re — Ом, Le — мГн, Vas — л, Sd — см², Xmax — мм, Mms — г, Bl — Т·м.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Driver {
    pub name: String,
    pub manufacturer: String,
    pub re: f64,
    pub le: f64,
    pub fs: f64,
    pub qms: f64,
    pub qes: f64,
    pub vas: f64,
    pub sd: f64,
    pub xmax: f64,
    /// Номинальная мощность, Вт
    pub pe: f64,
    /// Чувствительность, дБ (2.83 В / 1 м)
    pub spl: f64,
    /// Сила фактора, Т·м. 0 — вычислить из остальных параметров.
    pub bl: f64,
    /// Масса подвижной системы, г. 0 — вычислить из Vas/Fs/Sd.
    pub mms: f64,
    /// Модель индуктивности катушки
    #[serde(default)]
    pub le_model: LeModel,
    /// Kes полуиндуктивности, Ом·√с. 0 — оценить из Le на 1 кГц.
    #[serde(default)]
    pub kes: f64,
}

impl Default for Driver {
    fn default() -> Self {
        // Реалистичный 8" НЧ-динамик — стартовый пример для первого запуска.
        Self {
            name: "8\" woofer".into(),
            manufacturer: String::new(),
            re: 3.2,
            le: 1.2,
            fs: 30.0,
            qms: 4.5,
            qes: 0.42,
            vas: 62.0,
            sd: 220.0,
            xmax: 8.0,
            pe: 300.0,
            spl: 87.0,
            bl: 0.0,
            mms: 0.0,
            le_model: LeModel::Simple,
            kes: 0.0,
        }
    }
}

impl Driver {
    /// Полная добротность Qts.
    pub fn qts(&self) -> f64 {
        self.qes * self.qms / (self.qes + self.qms)
    }

    /// EBP — указатель пригодности: >90 — ФИ, <50 — ЗЯ.
    pub fn ebp(&self) -> f64 {
        self.fs / self.qes
    }

    /// Площадь диффузора, м².
    pub fn sd_m2(&self) -> f64 {
        self.sd / 1.0e4
    }

    /// Эквивалентный объём, м³.
    pub fn vas_m3(&self) -> f64 {
        self.vas / 1.0e3
    }

    /// Податливость подвески, м/Н (из Vas).
    pub fn cms(&self) -> f64 {
        let cas = air_compliance(self.vas_m3()); // акустическая ёмкость, м⁵/Н
        cas / (self.sd_m2() * self.sd_m2())
    }

    /// Масса подвижной системы, кг (из сохранённого значения или из Cms/Fs).
    pub fn mms_kg(&self) -> f64 {
        if self.mms > 0.0 {
            self.mms / 1.0e3
        } else {
            let w = TAU * self.fs;
            1.0 / (w * w * self.cms())
        }
    }

    /// Механические потери, Н·с/м.
    pub fn rms(&self) -> f64 {
        TAU * self.fs * self.mms_kg() / self.qms
    }

    /// Сила фактора, Т·м (из сохранённого или из Qes).
    pub fn bl_tm(&self) -> f64 {
        if self.bl > 0.0 {
            self.bl
        } else {
            (TAU * self.fs * self.mms_kg() * self.re / self.qes).sqrt()
        }
    }

    /// Импеданс катушки (электрическая ветвь) на круговой частоте ω.
    pub fn voice_coil_impedance(&self, omega: f64) -> Complex64 {
        let le_h = self.le * 1.0e-3;
        match self.le_model {
            LeModel::Simple => Complex64::new(self.re, omega * le_h),
            LeModel::Semi => {
                let kes = if self.kes > 0.0 {
                    self.kes
                } else {
                    le_h * TAU.sqrt() * (1000.0f64).sqrt()
                };
                // √(jω) = √(ω/2)·(1+j)
                let k = kes * (omega / 2.0).sqrt();
                Complex64::new(self.re + k, k)
            }
        }
    }

    /// Проверка ввода. Возвращает список проблем (пустой = всё в порядке).
    pub fn issues(&self) -> Vec<DriverIssue> {
        let mut out = Vec::new();
        let positive = [
            (DriverField::Re, self.re),
            (DriverField::Le, self.le),
            (DriverField::Fs, self.fs),
            (DriverField::Qms, self.qms),
            (DriverField::Qes, self.qes),
            (DriverField::Vas, self.vas),
            (DriverField::Sd, self.sd),
            (DriverField::Xmax, self.xmax),
        ];
        for (field, v) in positive {
            if !(v.is_finite() && v > 0.0) {
                out.push(DriverIssue::NonPositive(field));
            }
        }
        let qts = self.qts();
        if qts > 0.6 {
            out.push(DriverIssue::QtsHigh);
        } else if qts < 0.25 {
            out.push(DriverIssue::QtsLow);
        }
        out.push(DriverIssue::Ebp { value: self.ebp() });
        out
    }

    /// Все ли параметры физически валидны (для запуска расчёта).
    pub fn is_valid(&self) -> bool {
        self.re > 0.0
            && self.fs > 0.0
            && self.qms > 0.0
            && self.qes > 0.0
            && self.vas > 0.0
            && self.sd > 0.0
            && self.re.is_finite()
            && self.fs.is_finite()
            && self.qms.is_finite()
            && self.qes.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_params_consistent() {
        let d = Driver::default();
        // Проверка Bl через явное значение Mms
        let bl1 = d.bl_tm();
        let mut d2 = d.clone();
        d2.mms = d.mms_kg() * 1.0e3;
        let bl2 = d2.bl_tm();
        assert!((bl1 - bl2).abs() < 1e-9);

        // Qts
        let qts = d.qts();
        assert!((0.0 < qts) && (qts < d.qms.min(d.qes)));
    }

    #[test]
    fn cms_from_vas_scales() {
        let d1 = Driver {
            vas: 100.0,
            ..Driver::default()
        };
        let d2 = Driver {
            vas: 50.0,
            ..Driver::default()
        };
        assert!((d1.cms() / d2.cms() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn serde_roundtrip() {
        let d = Driver::default();
        let json = serde_json::to_string(&d).unwrap();
        let back: Driver = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, d.name);
        assert_eq!(back.re, d.re);
    }

    #[test]
    fn hf_slope_simple_6db_semi_3db_per_octave() {
        let simple = Driver::default();
        let semi = Driver {
            le_model: LeModel::Semi,
            ..Driver::default()
        };
        let z = |d: &Driver, f: f64| d.voice_coil_impedance(TAU * f).norm();
        let slope = |d: &Driver| {
            let z2k = z(d, 2000.0);
            let z8k = z(d, 8000.0);
            20.0 * (z8k / z2k).log10() / 2.0 // дБ на октаву (2 октавы)
        };
        let s_simple = slope(&simple);
        let s_semi = slope(&semi);
        assert!(
            (5.4..=6.4).contains(&s_simple),
            "Simple наклон {s_simple:.2} дБ/окт (ожидание ~6)"
        );
        assert!(
            (2.4..=3.4).contains(&s_semi),
            "Semi наклон {s_semi:.2} дБ/окт (ожидание ~3)"
        );
    }

    #[test]
    fn kes_estimate_matches_explicit() {
        let auto = Driver {
            le_model: LeModel::Semi,
            ..Driver::default()
        };
        let le_h = auto.le * 1.0e-3;
        let kes = le_h * TAU.sqrt() * (1000.0f64).sqrt();
        let explicit = Driver {
            le_model: LeModel::Semi,
            kes,
            ..Driver::default()
        };
        let w = TAU * 500.0;
        let diff = (auto.voice_coil_impedance(w) - explicit.voice_coil_impedance(w)).norm();
        assert!(diff < 1e-12);
    }

    #[test]
    fn issues_flags_bad_input() {
        let d = Driver {
            re: -1.0,
            ..Driver::default()
        };
        assert!(d
            .issues()
            .contains(&DriverIssue::NonPositive(DriverField::Re)));
        assert!(!d.is_valid());
    }
}
