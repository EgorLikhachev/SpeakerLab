//! Трансмиссионная линия / четвертьволновый резонатор / рупор.
//!
//! Сегментная модель длинной линии: каждый сегмент — 2-порт
//! (ABCD-матрица) с потерями на вязкость/теплообмен, каскад матриц,
//! нагрузка — импеданс излучения открытого конца. Динамик — у закрытого
//! конца (тыльная сторона), фронт диффузора излучает напрямую.

use num_complex::Complex64;
use serde::{Deserialize, Serialize};

use crate::circuit::EnclosureModel;
use crate::driver::Driver;
use crate::{AIR_DENSITY, SPEED_OF_SOUND};

/// Кинематическая вязкость воздуха, м²/с
const NU: f64 = 1.5e-5;
/// Отношение теплоёмкостей
const GAMMA: f64 = 1.4;
/// Число Прандтля
const PR: f64 = 0.71;

/// Сегмент линии.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Segment {
    /// Длина, м
    pub length_m: f64,
    /// Площадь в начале сегмента, см²
    pub area_start_cm2: f64,
    /// Площадь в конце сегмента, см²
    pub area_end_cm2: f64,
    /// Плотность набивки, кг/м³ (0 — пусто)
    pub stuffing_kgm3: f64,
}

impl Default for Segment {
    fn default() -> Self {
        Self {
            length_m: 0.4,
            area_start_cm2: 220.0,
            area_end_cm2: 220.0,
            stuffing_kgm3: 0.0,
        }
    }
}

impl Segment {
    fn a_start(&self) -> f64 {
        self.area_start_cm2 / 1.0e4
    }
    fn a_end(&self) -> f64 {
        self.area_end_cm2 / 1.0e4
    }
    /// Средняя площадь, м².
    fn a_avg(&self) -> f64 {
        0.5 * (self.a_start() + self.a_end())
    }
    /// Гидравлический радиус, м.
    fn r_h(&self) -> f64 {
        (self.a_avg() / std::f64::consts::PI).sqrt()
    }
}

/// Линия из сегментов.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LineBox {
    pub segments: Vec<Segment>,
    /// Площадь устья для излучения (берётся из последнего сегмента)
    pub closed_end_area_cm2: f64,
    /// Безразмерные потери на стенках/утечки (0 — идеальная линия; 0.35 — типично)
    pub wall_loss: f64,
    /// Смещение динамика от закрытого конца, м (0 — у конца).
    /// Участок [0..offset] работает закрытой заглушкой параллельно линии.
    #[serde(default)]
    pub driver_offset_m: f64,
    /// Обём камеры горла (горло перед линией), л. 0 — нет камеры.
    #[serde(default)]
    pub throat_chamber_l: f64,
}

impl Default for LineBox {
    fn default() -> Self {
        // Классическая ТЛ под 8": линия ~1.7 м, сужение к устью
        Self {
            segments: vec![
                Segment {
                    length_m: 0.6,
                    area_start_cm2: 220.0,
                    area_end_cm2: 200.0,
                    stuffing_kgm3: 10.0,
                },
                Segment {
                    length_m: 0.6,
                    area_start_cm2: 200.0,
                    area_end_cm2: 170.0,
                    stuffing_kgm3: 6.0,
                },
                Segment {
                    length_m: 0.5,
                    area_start_cm2: 170.0,
                    area_end_cm2: 150.0,
                    stuffing_kgm3: 0.0,
                },
            ],
            closed_end_area_cm2: 220.0,
            wall_loss: 0.35,
            driver_offset_m: 0.0,
            throat_chamber_l: 0.0,
        }
    }
}

impl LineBox {
    /// Общая длина, м.
    pub fn total_length(&self) -> f64 {
        self.segments.iter().map(|s| s.length_m.max(0.0)).sum()
    }

    /// Объём линии, л.
    pub fn volume_l(&self) -> f64 {
        self.segments
            .iter()
            .map(|s| s.a_avg() * s.length_m.max(0.0))
            .sum::<f64>()
            * 1.0e3
    }

    /// Четвертьволновая частота (без учёта набивки — геометрическая), Гц.
    pub fn quarter_wave_hz(&self) -> f64 {
        let l = self.total_length();
        if l <= 0.0 {
            return 0.0;
        }
        SPEED_OF_SOUND / (4.0 * l)
    }

    /// Площадь устья, м².
    fn mouth_area(&self) -> f64 {
        self.segments
            .last()
            .map(|s| s.a_end())
            .unwrap_or(0.0)
            .max(1e-4)
    }

    /// ABCD-матрица сегмента (2×2 комплексных) с граничными потерями
    /// и потерями в набивке (вязкая + тепловая релаксация).
    fn segment_matrix(seg: &Segment, omega: f64, wall_loss: f64) -> [[Complex64; 2]; 2] {
        let a = seg.a_avg();
        let rh = seg.r_h().max(1e-4);

        // Эквивалентные плотность и сжимаемость с граничными потерями:
        // вязкий слой: ρ_eff = ρ0·(1 + (1−j)·δ_v/r_h), δ_v = √(2ν/ω)
        // тепловой слой: K_eff = P0·γ/(γ − (γ−1)·(1 − (1−j)·δ_t/r_h)^-1), δ_t=δ_v/√Pr
        let delta = (2.0 * NU / omega.max(1e-3)).sqrt();
        let dv = Complex64::new(1.0, -1.0) * delta / rh;
        let dt = Complex64::new(1.0, -1.0) * delta / (rh * PR.sqrt());

        let p_atm = AIR_DENSITY * SPEED_OF_SOUND * SPEED_OF_SOUND / GAMMA;
        let mut rho_eff = AIR_DENSITY * (1.0 + dv);
        // κ_eff: изотермическая P_атм (НЧ) → адиабатическая γ·P_атм (ВЧ)
        let kappa_eff = p_atm * GAMMA
            / (GAMMA
                - (GAMMA - 1.0) * (Complex64::new(1.0, 0.0) / (Complex64::new(1.0, 0.0) - dt)));

        // Набивка: пористая среда (Дарси): ρ_eff = ρ0 + ρ_волокна − j·σ/ω.
        // Мнимая часть даёт затухание, действительная замедляет волну.
        // σ ≈ 120·d калибровано: 10 кг/м³ ≈ 2+ непера на линию — резонансы
        // подавляются, как у практических ТЛ с длинноволокнистой ватой.
        if seg.stuffing_kgm3 > 0.0 {
            let d = seg.stuffing_kgm3;
            let sigma = 120.0 * d; // Па·s/м²
            let rho_fiber = AIR_DENSITY * (d / 60.0); // присоединённая масса волокна
            rho_eff = rho_eff
                + Complex64::new(rho_fiber, 0.0)
                + Complex64::new(0.0, -sigma / omega.max(1e-3));
        }

        // Погонные параметры акустической линии (p, U):
        // z_unit [Па·с/м³ на м], y_unit [м³/(Па·с) на м]
        let mut z_unit = Complex64::new(0.0, 1.0) * omega * rho_eff / a;
        // Потери на стенках/утечки: активное сопротивление на метр, дающее
        // затухание ≈ wall_loss непер на метр линии.
        let z_line = AIR_DENSITY * SPEED_OF_SOUND / a;
        z_unit += Complex64::new(2.0 * wall_loss * z_line, 0.0);
        let y_unit = Complex64::new(0.0, 1.0) * omega * a / kappa_eff;

        let g = (z_unit * y_unit).sqrt(); // постоянная распространения, 1/м
        let z0 = (z_unit / y_unit).sqrt(); // характеристическое сопротивление
        let l = seg.length_m.max(0.0);

        let ch = (g * l).cosh();
        let sh = (g * l).sinh();
        [[ch, z0 * sh], [sh / z0, ch]]
    }

    /// Разбить сегменты на закрытую часть [0..offset] и основную [offset..L].
    /// Сегмент, пересекаемый смещением, делится на два (линейная интерполяция
    /// площади по длине).
    fn split_segments(&self) -> (Vec<Segment>, Vec<Segment>) {
        let off = self.driver_offset_m.max(0.0);
        if off <= 0.0 {
            return (Vec::new(), self.segments.clone());
        }
        let mut stub = Vec::new();
        let mut main = Vec::new();
        let mut acc = 0.0;
        for seg in &self.segments {
            let len = seg.length_m.max(0.0);
            if len <= 0.0 {
                continue;
            }
            if acc + len <= off + 1e-9 {
                stub.push(seg.clone());
            } else if acc >= off - 1e-9 {
                main.push(seg.clone());
            } else {
                // деление сегмента
                let t = (off - acc) / len;
                let a_mid = seg.area_start_cm2 + (seg.area_end_cm2 - seg.area_start_cm2) * t;
                stub.push(Segment {
                    length_m: off - acc,
                    area_start_cm2: seg.area_start_cm2,
                    area_end_cm2: a_mid,
                    stuffing_kgm3: seg.stuffing_kgm3,
                });
                main.push(Segment {
                    length_m: acc + len - off,
                    area_start_cm2: a_mid,
                    area_end_cm2: seg.area_end_cm2,
                    stuffing_kgm3: seg.stuffing_kgm3,
                });
            }
            acc += len;
        }
        (stub, main)
    }

    /// Каскад матриц произвольного списка сегментов.
    fn chain_of(segs: &[Segment], omega: f64, wall_loss: f64) -> [[Complex64; 2]; 2] {
        let mut m = [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        ];
        for seg in segs {
            if seg.length_m <= 0.0 {
                continue;
            }
            m = mat_mul(m, Self::segment_matrix(seg, omega, wall_loss));
        }
        m
    }

    /// Импеданс закрытой заглушки (нагрузка « холостого хода»): Z = A/C.
    fn stub_impedance(segs: &[Segment], omega: f64, wall_loss: f64) -> Option<Complex64> {
        if segs.iter().all(|s| s.length_m <= 0.0) {
            return None; // заглушки нет
        }
        let m = Self::chain_of(segs, omega, wall_loss);
        if m[1][0].norm_sqr() < 1e-30 {
            return Some(Complex64::new(1e12, 0.0));
        }
        Some(m[0][0] / m[1][0])
    }

    /// Импеданс основной линии от точки смещения до устья.
    fn main_impedance(segs: &[Segment], omega: f64, wall_loss: f64, z_rad: Complex64) -> Complex64 {
        let m = Self::chain_of(segs, omega, wall_loss);
        let denom = m[1][0] * z_rad + m[1][1];
        if denom.norm_sqr() < 1e-30 {
            return Complex64::new(1e12, 0.0);
        }
        (m[0][0] * z_rad + m[0][1]) / denom
    }

    /// Каскад всех сегментов.
    fn chain(&self, omega: f64) -> [[Complex64; 2]; 2] {
        let mut m = [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
        ];
        for seg in &self.segments {
            if seg.length_m <= 0.0 {
                continue;
            }
            m = mat_mul(m, Self::segment_matrix(seg, omega, self.wall_loss));
        }
        m
    }

    /// Импеданс излучения открытого конца (фланцевый, низкочастотное приближение).
    fn radiation_impedance(&self, omega: f64) -> Complex64 {
        let a = self.mouth_area();
        let r_mouth = (a / std::f64::consts::PI).sqrt(); // эквивалентный радиус
        let k = omega / SPEED_OF_SOUND;
        // Z ≈ ρ0c/A · ( (k·r)²/2 + j·0.6133·k·r )
        let kr = k * r_mouth;
        let char_z = AIR_DENSITY * SPEED_OF_SOUND / a;
        char_z * Complex64::new(kr * kr / 2.0, 0.6133 * kr)
    }

    /// Входной импеданс линии (с учётом смещения динамика и камеры горла).
    pub fn input_impedance(&self, omega: f64) -> Complex64 {
        if self.segments.is_empty() {
            return Complex64::new(1e12, 0.0); // заглушка: жёсткая стенка
        }
        let z_rad = self.radiation_impedance(omega);
        if self.driver_offset_m > 1e-6 {
            let (stub, main) = self.split_segments();
            let z_main = Self::main_impedance(&main, omega, self.wall_loss, z_rad);
            let z = match Self::stub_impedance(&stub, omega, self.wall_loss) {
                Some(zs) => crate::parallel(z_main, zs),
                None => z_main,
            };
            return self.with_throat(z, omega);
        }
        let m = self.chain(omega);
        // [p_in; U_in] = M·[p_m; U_m], p_m = Z_rad·U_m
        let denom = m[1][0] * z_rad + m[1][1];
        if denom.norm_sqr() < 1e-30 {
            return Complex64::new(1e12, 0.0);
        }
        let z = (m[0][0] * z_rad + m[0][1]) / denom;
        self.with_throat(z, omega)
    }

    /// Последовательная податливость камеры горла (если задана).
    fn with_throat(&self, z: Complex64, omega: f64) -> Complex64 {
        if self.throat_chamber_l > 1e-6 {
            let c_at = crate::air_compliance(self.throat_chamber_l / 1.0e3);
            z + Complex64::new(0.0, -1.0 / (omega * c_at))
        } else {
            z
        }
    }

    /// Объёмная скорость устья при потоке u_in, входящем в основную линию
    /// (при смещённом драйвере — доля общего потока, идущая в линию).
    fn mouth_flow_of(&self, u_line: Complex64, omega: f64) -> Complex64 {
        let main = if self.driver_offset_m > 1e-6 {
            self.split_segments().1
        } else {
            self.segments.clone()
        };
        let m = Self::chain_of(&main, omega, self.wall_loss);
        let z_rad = self.radiation_impedance(omega);
        let denom = m[1][0] * z_rad + m[1][1];
        if denom.norm_sqr() < 1e-30 {
            return Complex64::new(0.0, 0.0);
        }
        u_line / denom
    }

    /// Поток, отходящий в основную линию, при полном потоке драйвера u_total.
    fn line_share(&self, u_total: Complex64, omega: f64) -> Complex64 {
        if self.driver_offset_m <= 1e-6 {
            return u_total;
        }
        let (stub, main) = self.split_segments();
        let z_rad = self.radiation_impedance(omega);
        let z_main = Self::main_impedance(&main, omega, self.wall_loss, z_rad);
        let Some(zs) = Self::stub_impedance(&stub, omega, self.wall_loss) else {
            return u_total;
        };
        // параллельное деление: U_main = U_total · Zs/(Zs+Zmain)
        u_total * zs / (zs + z_main)
    }

    fn mouth_velocity_flow(&self, u_in: Complex64, omega: f64) -> Complex64 {
        let u_line = self.line_share(u_in, omega);
        self.mouth_flow_of(u_line, omega)
    }
}

fn mat_mul(a: [[Complex64; 2]; 2], b: [[Complex64; 2]; 2]) -> [[Complex64; 2]; 2] {
    let mut out = [[Complex64::new(0.0, 0.0); 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j];
        }
    }
    out
}

impl EnclosureModel for LineBox {
    fn acoustic_load(&self, _driver: &Driver, omega: f64) -> Complex64 {
        // Давление у закрытого конца при потоке U: входной импеданс линии.
        self.input_impedance(omega)
    }

    fn radiated_velocity(
        &self,
        _driver: &Driver,
        _p_node: Complex64,
        omega: f64,
        u_diaphragm: Complex64,
    ) -> Complex64 {
        // Фронт диффузора излучает напрямую (−U_d), устье — через линию.
        let u_mouth = self.mouth_velocity_flow(u_diaphragm, omega);
        u_mouth - u_diaphragm
    }

    fn port_flow(
        &self,
        _driver: &Driver,
        _p_node: Complex64,
        _omega: f64,
    ) -> Option<(Complex64, f64)> {
        // Скорость в устье требует U_d (не p_node) — см. mouth_velocity_flow.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::{simulate, SimConfig};
    use crate::TAU;

    /// Резонансы входного импеданса линии (максимумы) — поиск пиков.
    fn impedance_peaks(line: &LineBox, fmin: f64, fmax: f64) -> Vec<f64> {
        let n = 4000;
        let mut peaks = Vec::new();
        let mut prev2 = 0.0;
        let mut prev = 0.0;
        let mut prev_f = 0.0;
        for k in 0..=n {
            let f = fmin * (fmax / fmin).powf(k as f64 / n as f64);
            let w = TAU * f;
            let z = line.input_impedance(w).norm();
            if prev > prev2 && prev >= z && prev_f > fmin * 1.05 {
                peaks.push(prev_f);
            }
            prev2 = prev;
            prev = z;
            prev_f = f;
        }
        peaks
    }

    #[test]
    fn quarter_wave_resonance() {
        // Пустая линия постоянного сечения: первый пик Z около c/(4L) = 50 Гц.
        // Вязкостные/тепловые потери и излучение устья смещают моду вниз
        // на ~10–15% и удлиняют её концом — принимаем 40…56 Гц.
        let line = LineBox {
            segments: vec![Segment {
                length_m: 1.715, // c/4L = 343/6.86 = 50 Гц
                area_start_cm2: 200.0,
                area_end_cm2: 200.0,
                stuffing_kgm3: 0.0,
            }],
            closed_end_area_cm2: 200.0,
            wall_loss: 0.35,
            driver_offset_m: 0.0,
            throat_chamber_l: 0.0,
        };
        let peaks = impedance_peaks(&line, 15.0, 400.0);
        assert!(!peaks.is_empty(), "пиков нет");
        assert!(
            (40.0..=56.0).contains(&peaks[0]),
            "первый резонанс {:.1} Гц, ожидался 40–56 Гц",
            peaks[0]
        );
        // нечётные моды: следующий пик примерно втрое выше первой
        let third = peaks.iter().find(|p| **p > 2.5 * peaks[0]);
        assert!(
            third.is_some_and(|p| (p / peaks[0] - 3.0).abs() < 0.35),
            "третья мода не найдена: {peaks:?}"
        );
    }

    #[test]
    fn stuffing_damps_resonance() {
        let empty = LineBox {
            segments: vec![Segment {
                length_m: 1.715,
                area_start_cm2: 200.0,
                area_end_cm2: 200.0,
                stuffing_kgm3: 0.0,
            }],
            closed_end_area_cm2: 200.0,
            wall_loss: 0.35,
            driver_offset_m: 0.0,
            throat_chamber_l: 0.0,
        };
        let stuffed = LineBox {
            segments: vec![Segment {
                length_m: 1.715,
                area_start_cm2: 200.0,
                area_end_cm2: 200.0,
                stuffing_kgm3: 6.0,
            }],
            closed_end_area_cm2: 200.0,
            wall_loss: 0.35,
            driver_offset_m: 0.0,
            throat_chamber_l: 0.0,
        };
        let p_empty = impedance_peaks(&empty, 15.0, 300.0);
        assert!(!p_empty.is_empty(), "у пустой линии должны быть моды");

        // Набивка снижает пики АЧХ от стоячих волн: максимум SPL
        // в полосе 40–600 Гц на мелкой сетке должен падать заметно.
        let d = Driver::default();
        let spl_max = |l: &LineBox| -> f64 {
            let mut best = f64::NEG_INFINITY;
            for k in 0..=1200 {
                let f = 40.0 * (600.0f64 / 40.0).powf(k as f64 / 1200.0);
                let w = crate::TAU * f;
                let za = l.acoustic_load(&d, w);
                let op = crate::circuit::solve_point(&d, za, w, 2.83);
                let p = op.u_diaphragm * za;
                let u = l.radiated_velocity(&d, p, w, op.u_diaphragm);
                let spl = 20.0
                    * ((w * crate::AIR_DENSITY * u.norm() / crate::TAU).max(1e-30) / crate::P_REF)
                        .log10();
                if spl > best {
                    best = spl;
                }
            }
            best
        };
        let m_empty = spl_max(&empty);
        let m_stuffed = spl_max(&stuffed);
        assert!(
            m_stuffed < m_empty - 2.0,
            "набивка должна давить пик стоячей волны: {m_stuffed:.1} vs {m_empty:.1} дБ"
        );
    }

    #[test]
    fn simulate_produces_finite_curves() {
        let d = Driver::default();
        let line = LineBox::default();
        let curves = simulate(&d, &line, &SimConfig::default());
        assert!(curves.spl.iter().all(|v| v.is_finite()));
        // отклик выше 20 Гц в целом в разумных пределах
        let band: Vec<f64> = curves
            .freq
            .iter()
            .zip(curves.spl.iter())
            .filter(|(f, _)| **f >= 40.0 && **f <= 500.0)
            .map(|(_, s)| *s)
            .collect();
        let max = band.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max > 80.0 && max < 110.0, "SPL в полосе {max:.1} дБ");
    }

    fn straight(len: f64) -> LineBox {
        LineBox {
            segments: vec![Segment {
                length_m: len,
                area_start_cm2: 200.0,
                area_end_cm2: 200.0,
                stuffing_kgm3: 0.0,
            }],
            closed_end_area_cm2: 200.0,
            wall_loss: 0.35,
            driver_offset_m: 0.0,
            throat_chamber_l: 0.0,
        }
    }

    #[test]
    fn offset_smooths_even_mode_ripple() {
        // Смещённый драйвер сглаживает риппл чётной моды в АЧХ — классика ТЛ.
        use crate::circuit::{solve_point, EnclosureModel};
        let ripple = |line: &LineBox, flo: f64, fhi: f64| -> f64 {
            let d = Driver::default();
            let mut best = f64::NEG_INFINITY;
            let mut worst = f64::INFINITY;
            for k in 0..=400 {
                let f = flo * (fhi / flo).powf(k as f64 / 400.0);
                let w = crate::TAU * f;
                let za = line.acoustic_load(&d, w);
                let op = solve_point(&d, za, w, 2.83);
                let p = op.u_diaphragm * za;
                let u = line.radiated_velocity(&d, p, w, op.u_diaphragm);
                let spl = 20.0
                    * ((w * crate::AIR_DENSITY * u.norm() / crate::TAU).max(1e-30) / crate::P_REF)
                        .log10();
                best = best.max(spl);
                worst = worst.min(spl);
            }
            best - worst
        };
        let base = straight(1.715);
        let mut off = straight(1.715);
        off.driver_offset_m = 0.35;
        // эффект — в области верхних мод (3-я/5-я гармоники λ/4)
        let r_base = ripple(&base, 100.0, 300.0);
        let r_off = ripple(&off, 100.0, 300.0);
        assert!(
            r_off < r_base * 0.8,
            "смещение должно сглаживать верхние моды: риппл {r_off:.1} vs {r_base:.1} дБ"
        );
    }

    #[test]
    fn zero_offset_matches_plain_line() {
        let mut a = straight(1.2);
        a.driver_offset_m = 0.0;
        let b = straight(1.2);
        for k in 1..=50 {
            let f = 30.0 * k as f64;
            let d = (a.input_impedance(crate::TAU * f) - b.input_impedance(crate::TAU * f)).norm();
            assert!(d < 1e-9);
        }
    }

    #[test]
    fn throat_chamber_blocks_lf() {
        // Камера горла — последовательная ёмкость: растит |Z| на НЧ
        let mut with_ch = straight(1.715);
        with_ch.throat_chamber_l = 5.0;
        let base = straight(1.715);
        let w = crate::TAU * 25.0;
        let z_ch = with_ch.input_impedance(w).norm();
        let z_b = base.input_impedance(w).norm();
        assert!(
            z_ch > z_b * 2.0,
            "камера горла должна блокировать НЧ: {z_ch:.3e} vs {z_b:.3e}"
        );
    }

    #[test]
    fn volume_matches_geometry() {
        let line = LineBox::default();
        let v = line.volume_l();
        // ~0.6·200 + 0.6·185 + 0.5·160 = 120+111+80 см³·м = 31.1 л
        assert!((v - 31.1).abs() < 1.0, "объём {v:.1} л");
    }
}
