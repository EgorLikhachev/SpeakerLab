# -*- coding: utf-8 -*-
"""P2.3: offset-драйвер и камера горла для линии."""
import io


def patch(path, pairs):
    s = io.open(path, encoding='utf-8').read()
    for old, new in pairs:
        if old not in s:
            print(f'!! НЕ НАЙДЕНО в {path}: {old[:60]!r}')
            return False
        s = s.replace(old, new, 1)
    io.open(path, 'w', encoding='utf-8', newline='\n').write(s)
    return True


ok = patch('crates/acoustics/src/line.rs', [
    # поля LineBox
    ('''    /// Безразмерные потери на стенках/утечки (0 — идеальная линия; 0.35 — типично)
    pub wall_loss: f64,
}''',
     '''    /// Безразмерные потери на стенках/утечки (0 — идеальная линия; 0.35 — типично)
    pub wall_loss: f64,
    /// Смещение динамика от закрытого конца, м (0 — у конца).
    /// Участок [0..offset] работает закрытой заглушкой параллельно линии.
    #[serde(default)]
    pub driver_offset_m: f64,
    /// Обём камеры горла (горло перед линией), л. 0 — нет камеры.
    #[serde(default)]
    pub throat_chamber_l: f64,
}'''),
    # Default
    ('''            closed_end_area_cm2: 220.0,
            wall_loss: 0.35,
        }''',
     '''            closed_end_area_cm2: 220.0,
            wall_loss: 0.35,
            driver_offset_m: 0.0,
            throat_chamber_l: 0.0,
        }'''),
    # разбиение сегментов на смещении + stub/main импедансы
    ('''    /// Каскад всех сегментов.
    fn chain(&self, omega: f64) -> [[Complex64; 2]; 2] {''',
     '''    /// Разбить сегменты на закрытую часть [0..offset] и основную [offset..L].
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
    fn main_impedance(
        segs: &[Segment],
        omega: f64,
        wall_loss: f64,
        z_rad: Complex64,
    ) -> Complex64 {
        let m = Self::chain_of(segs, omega, wall_loss);
        let denom = m[1][0] * z_rad + m[1][1];
        if denom.norm_sqr() < 1e-30 {
            return Complex64::new(1e12, 0.0);
        }
        (m[0][0] * z_rad + m[0][1]) / denom
    }

    /// Каскад всех сегментов.
    fn chain(&self, omega: f64) -> [[Complex64; 2]; 2] {'''),
])
print('fields+helpers:', ok)

ok = patch('crates/acoustics/src/line.rs', [
    # input_impedance с offset/throat
    ('''    /// Входной импеданс линии с нагрузкой излучением.
    pub fn input_impedance(&self, omega: f64) -> Complex64 {
        if self.segments.is_empty() {
            return Complex64::new(1e12, 0.0); // заглушка: жёсткая стенка
        }
        let m = self.chain(omega);
        let z_rad = self.radiation_impedance(omega);
        // [p_in; U_in] = M·[p_m; U_m], p_m = Z_rad·U_m
        // p_in = (M00·Z_rad + M01)·U_m; U_in = (M10·Z_rad + M11)·U_m
        let denom = m[1][0] * z_rad + m[1][1];
        if denom.norm_sqr() < 1e-30 {
            return Complex64::new(1e12, 0.0);
        }
        (m[0][0] * z_rad + m[0][1]) / denom
    }''',
     '''    /// Входной импеданс линии (с учётом смещения динамика и камеры горла).
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
    }'''),
    # mouth flow: от точки входа основной линии
    ('''    /// Объёмная скорость устья при данной объёмной скорости на входе.
    fn mouth_velocity_flow(&self, u_in: Complex64, omega: f64) -> Complex64 {
        let m = self.chain(omega);
        let z_rad = self.radiation_impedance(omega);
        let denom = m[1][0] * z_rad + m[1][1];
        if denom.norm_sqr() < 1e-30 {
            return Complex64::new(0.0, 0.0);
        }
        u_in / denom
    }''',
     '''    /// Объёмная скорость устья при потоке u_in, входящем в основную линию
    /// (при смещённом драйвере — доля общего потока, идущая в линию).
    fn mouth_flow_of(&self, u_line: Complex64, omega: f64) -> Complex64 {
        let (stub, main) = if self.driver_offset_m > 1e-6 {
            self.split_segments()
        } else {
            (Vec::new(), self.segments.clone())
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
    }'''),
])
print('impedance+flow:', ok)

# тесты
ok = patch('crates/acoustics/src/line.rs', [
    ('''    #[test]
    fn volume_matches_geometry() {''',
     '''    fn straight(len: f64) -> LineBox {
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
    fn offset_suppresses_even_mode() {
        // Смещённый драйвер подавляет чётные моды (λ/2, 2λ/4) — классика ТЛ.
        let base = straight(1.715);
        let mut off = straight(1.715);
        off.driver_offset_m = 0.35;
        let f2 = 2.0 * base.quarter_wave_hz(); // вторая мода
        let w = crate::TAU * f2 * 0.93; // зона горба |Z| второй моды
        let z_base = base.input_impedance(w).norm();
        let z_off = off.input_impedance(w).norm();
        assert!(
            z_off < z_base * 0.9,
            "смещение должно гасить чётную моду: {z_off:.3e} vs {z_base:.3e}"
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
    fn throat_chamber_lowers_input_impedance_lf() {
        // Камера горла — последовательная ёмкость: на НЧ шунтирует нагрузку
        let mut with_ch = straight(1.2);
        with_ch.throat_chamber_l = 5.0;
        let base = straight(1.2);
        let w = crate::TAU * 40.0;
        let z_ch = with_ch.input_impedance(w).norm();
        let z_b = base.input_impedance(w).norm();
        assert!(
            z_ch < z_b,
            "камера горла должна снижать |Z| на НЧ: {z_ch:.3e} vs {z_b:.3e}"
        );
    }

    #[test]
    fn volume_matches_geometry() {'''),
])
print('tests:', ok)
