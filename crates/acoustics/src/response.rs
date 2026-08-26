//! Расчёт частотных характеристик системы «динамик + оформление».

use num_complex::Complex64;
use serde::{Deserialize, Serialize};

use crate::circuit::{solve_point, EnclosureModel};
use crate::driver::Driver;
use crate::{AIR_DENSITY, P_REF, TAU};

/// Настройки симуляции.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SimConfig {
    /// Напряжение синусоидального генератора, В (действ.)
    pub voltage: f64,
    pub fmin: f64,
    pub fmax: f64,
    pub points: usize,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            voltage: 2.83,
            fmin: 10.0,
            fmax: 20000.0,
            points: 512,
        }
    }
}

/// Все рассчитанные кривые (индексы согласованы с `freq`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Curves {
    pub freq: Vec<f64>,
    /// SPL, дБ (2π-полупространство, 1 м)
    pub spl: Vec<f64>,
    /// |Z|, Ом
    pub z_mag: Vec<f64>,
    /// Фаза Z, град
    pub z_phase: Vec<f64>,
    /// Экскурсия диффузора, мм (амплитуда)
    pub excursion_mm: Vec<f64>,
    /// Скорость воздуха в порте, м/с (если порт задан)
    pub port_vel_m_s: Option<Vec<f64>>,
    /// Групповая задержка, мс
    pub group_delay_ms: Vec<f64>,
    /// SPL «выравнивания» без индуктивности катушки (Le = 0):
    /// для честных F3/Fb-метрик, не искажённых бугром от Le.
    pub alignment_spl: Vec<f64>,
}

/// Сводка по результатам (для статусной строки и предупреждений).
/// Максимумы ищутся в осмысленных полосах: |Z| — 15–500 Гц (иначе «максимум»
/// всегда даёт рост индуктивности на ВЧ), ход — 15–300 Гц (ниже — вне
/// рабочего диапазона, выше — ход всегда мал).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Summary {
    pub peak_spl: f64,
    /// Нижняя/верхняя частоты по уровню −3 дБ от пика
    pub f3_low: Option<f64>,
    pub f3_high: Option<f64>,
    pub z_min: f64,
    pub z_max: f64,
    pub z_max_freq: f64,
    pub excursion_max_mm: f64,
    pub excursion_max_freq: f64,
    /// Ход диффузора на частоте настройки (ФИ/ПИ/БП), мм
    pub excursion_at_tuning: Option<f64>,
    pub port_vel_max_m_s: Option<f64>,
    pub port_vel_max_freq: f64,
}

/// Окно поиска максимума |Z|, Гц.
const Z_WINDOW: (f64, f64) = (15.0, 500.0);
/// Окно поиска максимума хода, Гц.
const EXC_WINDOW: (f64, f64) = (15.0, 300.0);
/// Окно расчёта предельных напряжений, Гц.
const LIMIT_WINDOW: (f64, f64) = (15.0, 500.0);
/// Предельная скорость воздуха в порте для расчёта Vмакс, м/с.
pub const PORT_VEL_LIMIT_M_S: f64 = 17.0;

/// Предельные напряжения системы (до нарушения ограничений).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Limits {
    /// (Напряжение, частота) до достижения Xmax
    pub v_xmax: Option<(f64, f64)>,
    /// (Напряжение, частота) до предельной скорости порта
    pub v_port: Option<(f64, f64)>,
    /// Напряжение тепловой (мощностной) границы, В
    pub v_thermal: Option<f64>,
    /// Самое строгое из доступных ограничений, В
    pub v_limit: Option<f64>,
    /// Что именно ограничивает
    pub limiting: LimitKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LimitKind {
    #[default]
    None,
    Xmax,
    Port,
    Thermal,
}

const DB_FLOOR: f64 = -120.0;

/// Основной расчёт: лог-сетка частот, для каждой — решение схемы.
pub fn simulate(driver: &Driver, enclosure: &dyn EnclosureModel, cfg: &SimConfig) -> Curves {
    let n = cfg.points.max(16);
    let log_ratio = (cfg.fmax / cfg.fmin).ln();

    // Копия динамика без индуктивности — для метрик выравнивания (F3 и т.п.).
    let mut d_align = driver.clone();
    d_align.le = 0.0;

    let mut freq = Vec::with_capacity(n);
    let mut spl = Vec::with_capacity(n);
    let mut alignment_spl = Vec::with_capacity(n);
    let mut z_mag = Vec::with_capacity(n);
    let mut z_phase = Vec::with_capacity(n);
    let mut excursion_mm = Vec::with_capacity(n);
    let mut port_vel: Vec<f64> = Vec::with_capacity(n);
    let mut u_rad_phase: Vec<f64> = Vec::with_capacity(n);

    let has_port = enclosure
        .port_flow(driver, Complex64::new(0.0, 0.0), TAU * cfg.fmin)
        .is_some();

    for k in 0..n {
        let f = cfg.fmin * (log_ratio * (k as f64 / (n - 1) as f64)).exp();
        let omega = TAU * f;

        let za = enclosure.acoustic_load(driver, omega);
        let op = solve_point(driver, za, omega, cfg.voltage);
        let p_node = op.u_diaphragm * za;
        let u_rad = enclosure.radiated_velocity(driver, p_node, omega, op.u_diaphragm);

        // Дальнее поле, полупространство, 1 м: p = ω·ρ₀·|U|/(2π·r)
        let p_far = omega * AIR_DENSITY * u_rad.norm() / TAU;
        let spl_db = 20.0 * (p_far.max(1e-30) / P_REF).log10();
        spl.push(spl_db.max(DB_FLOOR));

        // То же без Le — только излучение, для метрик выравнивания.
        let za_a = enclosure.acoustic_load(&d_align, omega);
        let op_a = solve_point(&d_align, za_a, omega, cfg.voltage);
        let p_a = op_a.u_diaphragm * za_a;
        let u_a = enclosure.radiated_velocity(&d_align, p_a, omega, op_a.u_diaphragm);
        let p_far_a = omega * AIR_DENSITY * u_a.norm() / TAU;
        alignment_spl.push((20.0 * (p_far_a.max(1e-30) / P_REF).log10()).max(DB_FLOOR));

        z_mag.push(op.z_in.norm());
        z_phase.push(op.z_in.arg().to_degrees());
        excursion_mm.push(op.cone_velocity.norm() / omega * 1.0e3);
        u_rad_phase.push(u_rad.arg());

        if has_port {
            if let Some((u_p, area)) = enclosure.port_flow(driver, p_node, omega) {
                let v = if area > 0.0 { u_p.norm() / area } else { 0.0 };
                port_vel.push(v);
            } else {
                port_vel.push(0.0);
            }
        }

        freq.push(f);
    }

    let group_delay_ms = group_delay(&freq, &u_rad_phase);

    Curves {
        freq,
        spl,
        alignment_spl,
        z_mag,
        z_phase,
        excursion_mm,
        port_vel_m_s: has_port.then_some(port_vel),
        group_delay_ms,
    }
}

/// Групповая задержка τ = −dφ/dω (мс) по развёрнутой фазе излучения.
fn group_delay(freq: &[f64], phase: &[f64]) -> Vec<f64> {
    let n = freq.len();
    if n < 2 {
        return vec![0.0; n];
    }
    // развёртка фазы (±2π)
    let mut unwrapped = Vec::with_capacity(n);
    unwrapped.push(phase[0]);
    for k in 1..n {
        let mut ph = phase[k];
        while ph - unwrapped[k - 1] > std::f64::consts::PI {
            ph -= TAU;
        }
        while ph - unwrapped[k - 1] < -std::f64::consts::PI {
            ph += TAU;
        }
        unwrapped.push(ph);
    }
    let mut out = Vec::with_capacity(n);
    out.push(-(unwrapped[1] - unwrapped[0]) / (TAU * (freq[1] - freq[0])) * 1.0e3);
    for k in 1..n - 1 {
        let dphi = unwrapped[k + 1] - unwrapped[k - 1];
        let dw = TAU * (freq[k + 1] - freq[k - 1]);
        out.push(-dphi / dw * 1.0e3);
    }
    out.push(*out.last().unwrap());
    out
}

/// Сводка по кривым: −3 дБ, пики, максимумы в рабочих полосах.
/// `tuning_hz` — частота настройки ФИ/ПИ/БП (для метрики «ход @ Fb»).
/// Пик и частоты среза берутся из кривой выравнивания (без Le).
pub fn summarize(curves: &Curves, tuning_hz: Option<f64>) -> Summary {
    let mut s = Summary {
        peak_spl: curves
            .alignment_spl
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max),
        ..Default::default()
    };
    if s.peak_spl.is_finite() && curves.freq.len() > 8 {
        s.f3_low = crossing_below(&curves.freq, &curves.alignment_spl, s.peak_spl - 3.0);
        s.f3_high = crossing_above(&curves.freq, &curves.alignment_spl, s.peak_spl - 3.0);
    }

    let in_band = |f: f64, w: (f64, f64)| f >= w.0 && f <= w.1;

    let mut z_min = f64::INFINITY;
    for (i, &z) in curves.z_mag.iter().enumerate() {
        if in_band(curves.freq[i], Z_WINDOW) && z > s.z_max {
            s.z_max = z;
            s.z_max_freq = curves.freq[i];
        }
        if z < z_min {
            z_min = z;
        }
    }
    s.z_min = if z_min.is_finite() { z_min } else { 0.0 };

    for (i, &e) in curves.excursion_mm.iter().enumerate() {
        if in_band(curves.freq[i], EXC_WINDOW) && e > s.excursion_max_mm {
            s.excursion_max_mm = e;
            s.excursion_max_freq = curves.freq[i];
        }
    }

    // Ход на частоте настройки: минимум между пиками |Z| — проектная точка.
    if let Some(fb) = tuning_hz {
        if let Some(i) = curves
            .freq
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - fb).abs().total_cmp(&(b.1 - fb).abs()))
            .map(|(i, _)| i)
        {
            s.excursion_at_tuning = Some(curves.excursion_mm[i]);
        }
    }

    if let Some(vel) = &curves.port_vel_m_s {
        let mut vmax = 0.0;
        for (i, &v) in vel.iter().enumerate() {
            if v > vmax {
                vmax = v;
                s.port_vel_max_freq = curves.freq[i];
            }
        }
        if vmax > 0.0 {
            s.port_vel_max_m_s = Some(vmax);
        }
    }
    s
}

/// Предельные напряжения: до Xmax, до скорости порта, до тепловой границы.
///
/// Система линейна по напряжению: достаточно пересчитать текущие кривые
/// (получены при `voltage`) пропорционально.
pub fn compute_limits(
    curves: &Curves,
    voltage: f64,
    xmax_mm: f64,
    pe_w: f64,
    z_min_ohm: f64,
) -> Limits {
    let mut lim = Limits::default();
    if !(voltage > 0.0) {
        return lim;
    }

    // Минимальное допустимое напряжение по каждой частоте окна.
    if xmax_mm > 0.0 {
        let mut best: Option<(f64, f64)> = None;
        for (i, &e) in curves.excursion_mm.iter().enumerate() {
            let f = curves.freq[i];
            if !(LIMIT_WINDOW.0..=LIMIT_WINDOW.1).contains(&f) || e <= 1e-9 {
                continue;
            }
            let v = voltage * xmax_mm / e;
            if best.is_none_or(|(bv, _)| v < bv) {
                best = Some((v, f));
            }
        }
        lim.v_xmax = best;
    }
    if let Some(vel) = &curves.port_vel_m_s {
        let mut best: Option<(f64, f64)> = None;
        for (i, &v_) in vel.iter().enumerate() {
            let f = curves.freq[i];
            if !(LIMIT_WINDOW.0..=LIMIT_WINDOW.1).contains(&f) || v_ <= 1e-9 {
                continue;
            }
            let v = voltage * PORT_VEL_LIMIT_M_S / v_;
            if best.is_none_or(|(bv, _)| v < bv) {
                best = Some((v, f));
            }
        }
        lim.v_port = best;
    }
    if pe_w > 0.0 && z_min_ohm > 0.0 {
        lim.v_thermal = Some((pe_w * z_min_ohm).sqrt());
    }

    // Самое строгое ограничение
    let mut min_v = f64::INFINITY;
    let mut kind = LimitKind::None;
    if let Some((v, _)) = lim.v_xmax {
        if v < min_v {
            min_v = v;
            kind = LimitKind::Xmax;
        }
    }
    if let Some((v, _)) = lim.v_port {
        if v < min_v {
            min_v = v;
            kind = LimitKind::Port;
        }
    }
    if let Some(v) = lim.v_thermal {
        if v < min_v {
            min_v = v;
            kind = LimitKind::Thermal;
        }
    }
    if min_v.is_finite() {
        lim.v_limit = Some(min_v);
        lim.limiting = kind;
    }
    lim
}

/// Первое пересечение уровня `level` снизу вверх (нижняя частота среза).
fn crossing_below(freq: &[f64], spl: &[f64], level: f64) -> Option<f64> {
    for k in 1..spl.len() {
        if spl[k] >= level && spl[k - 1] < level {
            let t = (level - spl[k - 1]) / (spl[k] - spl[k - 1]);
            return Some(freq[k - 1] * (freq[k] / freq[k - 1]).powf(t));
        }
    }
    None
}

/// Пересечение уровня `level` сверху вниз после максимума.
fn crossing_above(freq: &[f64], spl: &[f64], level: f64) -> Option<f64> {
    let peak_i = spl
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| i)?;
    for k in peak_i..spl.len().saturating_sub(1) {
        if spl[k] >= level && spl[k + 1] < level {
            let t = (spl[k] - level) / (spl[k] - spl[k + 1]);
            return Some(freq[k] * (freq[k + 1] / freq[k]).powf(t));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sealed::SealedBox;
    use crate::vented::VentedBox;

    #[test]
    fn sealed_f3_matches_classic_formula() {
        let d = Driver::default();
        // Ящик без потерь, чтобы сравниваться с классической формулой.
        let b = SealedBox {
            vb: 30.0,
            qa: 1.0e9,
            ..Default::default()
        };
        let cfg = SimConfig::default();
        let curves = simulate(&d, &b, &cfg);
        let s = summarize(&curves, None);

        // f3 = fc·sqrt( sqrt(a²+1) − a ), a = 1 − 1/(2Qtc²)
        // (для Qtc < 0.707 срез лежит выше fc)
        let fc = b.fc(&d);
        let qtc = b.qtc(&d);
        let a = 1.0 - 1.0 / (2.0 * qtc * qtc);
        let f3 = fc * ((a * a + 1.0).sqrt() - a).sqrt();
        let got = s.f3_low.unwrap();
        assert!(
            (got - f3).abs() / f3 < 0.02,
            "f3 из кривой {got:.1} Гц, формула {f3:.1} Гц"
        );
    }

    #[test]
    fn sensitivity_matches_reference_efficiency() {
        // Эталонная эффективность Тиля–Смолла: η0 = 9.64e-10·Fs³·Vas(л)/Qes,
        // SPL(1 Вт, 1 м, полупространство) = 112.02 + 10·lg(η0).
        // Симуляция при 2.83 В должна дать +10·lg(V²/Re) к этому уровню.
        // Сравниваем без Le: η0 определена для идеального драйвера.
        let d = Driver {
            le: 0.0,
            ..Driver::default()
        };
        let b = VentedBox {
            vb: 50.0,
            fb: 35.0,
            ..Default::default()
        };
        let curves = simulate(&d, &b, &SimConfig::default());
        let band: Vec<f64> = curves
            .freq
            .iter()
            .zip(curves.spl.iter())
            .filter(|(f, _)| **f >= 200.0 && **f <= 1000.0)
            .map(|(_, s)| *s)
            .collect();
        let sim = band.iter().sum::<f64>() / band.len() as f64;

        let eta0 = 9.64e-10 * d.fs.powi(3) * d.vas / d.qes;
        let theory = 112.02 + 10.0 * eta0.log10() + 10.0 * (2.83f64.powi(2) / d.re).log10();
        assert!(
            (sim - theory).abs() < 1.5,
            "SPL полосы {sim:.2} дБ против эталонного {theory:.2} дБ"
        );
    }

    #[test]
    fn sensitivity_sane() {
        let d = Driver::default();
        let b = SealedBox {
            vb: 60.0,
            ..Default::default()
        };
        let cfg = SimConfig::default();
        let curves = simulate(&d, &b, &cfg);
        // средний SPL в полосе 200–2000 Гц близок к паспортной чувствительности
        let band: Vec<f64> = curves
            .freq
            .iter()
            .zip(&curves.spl)
            .filter(|(f, _)| **f >= 200.0 && **f <= 2000.0)
            .map(|(_, s)| *s)
            .collect();
        let avg = band.iter().sum::<f64>() / band.len() as f64;
        assert!(
            (avg - d.spl).abs() < 3.0,
            "средний SPL {avg:.1} дБ против паспортных {:.1} дБ",
            d.spl
        );
    }

    #[test]
    fn vented_extends_deeper_but_falls_steeper() {
        let d = Driver::default();
        let cfg = SimConfig::default();
        let sealed = simulate(
            &d,
            &SealedBox {
                vb: 50.0,
                ..Default::default()
            },
            &cfg,
        );
        let vented = simulate(
            &d,
            &VentedBox {
                vb: 50.0,
                fb: 35.0,
                ..Default::default()
            },
            &cfg,
        );

        // ФИ расширяет бас вниз при том же объёме
        let f3_sealed = summarize(&sealed, None).f3_low.unwrap();
        let f3_vented = summarize(&vented, None).f3_low.unwrap();
        assert!(
            f3_vented < f3_sealed - 8.0,
            "f3 ФИ {f3_vented:.1} Гц должна быть заметно ниже ЗЯ {f3_sealed:.1} Гц"
        );

        // …но ниже настройки валится быстрее (24 дБ/окт против 12)
        let at = |c: &Curves, f: f64| -> f64 {
            let i = c
                .freq
                .iter()
                .enumerate()
                .min_by(|a, b| (a.1 - f).abs().total_cmp(&(b.1 - f).abs()))
                .unwrap()
                .0;
            c.spl[i]
        };
        let octaves = (25.0f64 / 15.0).log2();
        let slope = |c: &Curves| (at(c, 25.0) - at(c, 15.0)) / octaves;
        assert!(
            slope(&vented) > slope(&sealed) + 4.0,
            "спад ФИ {:.1} дБ/окт против ЗЯ {:.1} дБ/окт",
            slope(&vented),
            slope(&sealed)
        );
    }

    #[test]
    fn group_delay_positive_and_peaked_near_fb() {
        let d = Driver::default();
        let b = VentedBox {
            vb: 50.0,
            fb: 35.0,
            ..Default::default()
        };
        let curves = simulate(&d, &b, &SimConfig::default());
        assert!(curves.group_delay_ms.iter().all(|t| t.is_finite()));
        let peak = curves.group_delay_ms.iter().cloned().fold(0.0f64, f64::max);
        assert!(peak > 5.0, "пик ГЗ {peak:.1} мс — подозрительно мал");
    }

    #[test]
    fn summary_windows_are_meaningful() {
        // Максимумы ищутся в рабочих полосах, а не на краях сетки
        let d = Driver::default();
        let b = VentedBox {
            vb: 50.0,
            fb: 35.0,
            ..Default::default()
        };
        let curves = simulate(&d, &b, &SimConfig::default());
        let s = summarize(&curves, Some(b.fb));

        assert!(
            (15.0..=500.0).contains(&s.z_max_freq) && s.z_max_freq < 200.0,
            "|Z|max на {} Гц — должен быть резонанс системы, не ВЧ-рост Le",
            s.z_max_freq
        );
        assert!(
            (15.0..=300.0).contains(&s.excursion_max_freq),
            "ход макс на {} Гц — вне окна",
            s.excursion_max_freq
        );
        // на настройке ход минимален (порт разгружает диффузор)
        let efb = s.excursion_at_tuning.expect("ход @ Fb");
        assert!(
            efb < s.excursion_max_mm * 0.8,
            "ход @ Fb {efb:.2} мм vs макс {} мм",
            s.excursion_max_mm
        );
    }

    #[test]
    fn limits_linear_and_identify_bottleneck() {
        let d = Driver::default();
        let b = VentedBox {
            vb: 50.0,
            fb: 35.0,
            ..Default::default()
        };
        let cfg = SimConfig::default();
        let curves = simulate(&d, &b, &cfg);
        let s = summarize(&curves, Some(b.fb));
        let lim = compute_limits(&curves, cfg.voltage, d.xmax, d.pe, s.z_min);

        assert!(lim.v_limit.is_some());
        let (v1, f1) = lim.v_xmax.expect("v_xmax");
        assert!((15.0..=500.0).contains(&f1));

        // Линейность: при другом напряжении предельное то же
        let cfg2 = SimConfig {
            voltage: 5.66,
            ..Default::default()
        };
        let curves2 = simulate(&d, &b, &cfg2);
        let s2 = summarize(&curves2, Some(b.fb));
        let lim2 = compute_limits(&curves2, cfg2.voltage, d.xmax, d.pe, s2.z_min);
        let (v2, _) = lim2.v_xmax.unwrap();
        assert!((v1 - v2).abs() / v1 < 0.01, "{v1:.2} vs {v2:.2}");

        // При v_xmax максимум хода в окне равен Xmax (±2%)
        let cfgx = SimConfig {
            voltage: v1,
            ..Default::default()
        };
        let cx = simulate(&d, &b, &cfgx);
        let sx = summarize(&cx, Some(b.fb));
        assert!(
            (sx.excursion_max_mm / d.xmax - 1.0).abs() < 0.02,
            "ход при v_xmax: {} мм vs Xmax {} мм",
            sx.excursion_max_mm,
            d.xmax
        );

        // Узкий порт: ограничивает скорость воздуха, а не Xmax
        let narrow = VentedBox {
            vb: 50.0,
            fb: 35.0,
            port: Some(crate::port::PortSpec::new(
                crate::port::PortGeometry::Round { diameter_mm: 30.0 },
            )),
            ..Default::default()
        };
        let cn = simulate(&d, &narrow, &SimConfig::default());
        let sn = summarize(&cn, Some(narrow.fb));
        let ln = compute_limits(&cn, 2.83, d.xmax, d.pe, sn.z_min);
        assert_eq!(ln.limiting, LimitKind::Port);
    }
}
