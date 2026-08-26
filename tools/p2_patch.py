# -*- coding: utf-8 -*-
"""P2.4 (q_port) + P2.5 (ход ПИ) + P2.11 (компрессия порта)."""
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


# ============ P2.4: q_port + P2.11: компрессия ============
ok = patch('crates/acoustics/src/vented.rs', [
    ('''/// Потери порта приняты почти нулевыми (Qp ≈ 10), как в классической модели.
const Q_PORT: f64 = 10.0;''',
     '''/// Коэффициент нелинейного роста сопротивления порта с ростом скорости
/// (компрессия выхода порта на большой скорости).
const PORT_COMPRESSION_K: f64 = 0.02;'''),
    ('''    /// Геометрия порта (нужна для скорости воздуха; настройка — по Fb)
    pub port: Option<PortSpec>,
}''',
     '''    /// Геометрия порта (нужна для скорости воздуха; настройка — по Fb)
    pub port: Option<PortSpec>,
    /// Добротность потерь порта (10 — почти без потерь)
    #[serde(default = "default_q_port")]
    pub q_port: f64,
}

fn default_q_port() -> f64 {
    10.0
}'''),
    ('''        Self {
            vb: 55.0,
            fb: 34.0,
            ql: 10.0,
            port: None,
        }''',
     '''        Self {
            vb: 55.0,
            fb: 34.0,
            ql: 10.0,
            port: None,
            q_port: 10.0,
        }'''),
    ('''    fn port_branch_impedance(&self, omega: f64) -> Complex64 {
        let m = self.port_mass();
        let r_ap = TAU * self.fb * m / Q_PORT;
        Complex64::new(r_ap, omega * m)
    }''',
     '''    fn port_branch_impedance(&self, omega: f64) -> Complex64 {
        let m = self.port_mass();
        let r_ap = TAU * self.fb * m / self.q_port.max(0.5);
        Complex64::new(r_ap, omega * m)
    }

    /// Сопротивление порта с учётом компрессии по скорости воздуха:
    /// R(v) = R₀·(1 + k·(v/20 м/с)²), 3 итерации от узлового давления.
    fn port_branch_nonlinear(&self, omega: f64, p_node: Complex64) -> Complex64 {
        let m = self.port_mass();
        let r0 = TAU * self.fb * m / self.q_port.max(0.5);
        let area = self.port.as_ref().map(|s| s.area_one_m2()).unwrap_or(1e-3);
        let mut z = Complex64::new(r0, omega * m);
        for _ in 0..3 {
            let v = (p_node / z).norm() / area;
            z = Complex64::new(r0 * (1.0 + PORT_COMPRESSION_K * (v / 20.0).powi(2)), omega * m);
        }
        z
    }'''),
    ('''        // v > 0 — диффузор движется В ящик: фронтальная сторона излучает −U_d,
        // а порт под положительным давлением выкачивает +U_p.
        let z_p = self.port_branch_impedance(omega);
        let u_p = p_node / z_p;
        u_p - u_diaphragm''',
     '''        // v > 0 — диффузор движется В ящик: фронтальная сторона излучает −U_d,
        // а порт под положительным давлением выкачивает +U_p.
        // R порта учитывает компрессию по скорости воздуха (P2.11).
        let z_p = self.port_branch_nonlinear(omega, p_node);
        let u_p = p_node / z_p;
        u_p - u_diaphragm'''),
    ('''    #[test]
    fn below_fb_excursion_grows() {''',
     '''    #[test]
    fn port_compression_reduces_output_at_high_velocity() {
        use num_complex::Complex64 as C;
        let d = Driver::default();
        let b = VentedBox {
            vb: 50.0,
            fb: 35.0,
            port: Some(PortSpec::new(crate::port::PortGeometry::Round {
                diameter_mm: 40.0,
            })),
            ..Default::default()
        };
        let omega = TAU * b.fb;
        let u_small = b.radiated_velocity(&d, C::new(1.0, 0.0), omega, C::new(0.0, 0.0));
        // ×1000 давления → скорость >20 м/с → компрессия
        let u_big = b.radiated_velocity(&d, C::new(1000.0, 0.0), omega, C::new(0.0, 0.0));
        let ratio = u_big.norm() / (u_small.norm() * 1000.0);
        assert!(
            ratio < 0.98,
            "компрессия должна снижать выход порта: отношение {ratio:.3}"
        );
    }

    #[test]
    fn below_fb_excursion_grows() {'''),
])
print('vented:', ok)

# ============ P2.5: ход ПИ ============
ok = patch('crates/acoustics/src/passive.rs', [
    ('''    /// Добротность потерь ящика
    pub ql: f64,
}''',
     '''    /// Добротность потерь ящика
    pub ql: f64,
    /// Предельный ход ПИ, мм
    #[serde(default = "default_pr_xmax")]
    pub xmax_mm: f64,
}

fn default_pr_xmax() -> f64 {
    10.0
}'''),
    ('''        Self {
            vb: 55.0,
            mass_g: 120.0,
            sd_cm2: 220.0,
            fs_pr: 22.0,
            ql: 10.0,
        }''',
     '''        Self {
            vb: 55.0,
            mass_g: 120.0,
            sd_cm2: 220.0,
            fs_pr: 22.0,
            ql: 10.0,
            xmax_mm: 10.0,
        }'''),
])
print('passive:', ok)

ok = True and patch('crates/acoustics/src/response.rs', [
    ('''    /// Скорость воздуха в порте, м/с (если порт задан)
    pub port_vel_m_s: Option<Vec<f64>>,''',
     '''    /// Скорость воздуха в порте, м/с (если порт задан)
    pub port_vel_m_s: Option<Vec<f64>>,
    /// Перемещение «поршня» порта/ПИ (амплитуда, мм) — ход ПИ для ПИ
    #[serde(default)]
    pub port_disp_mm: Option<Vec<f64>>,'''),
    ('''    let mut port_vel: Vec<f64> = Vec::with_capacity(n);''',
     '''    let mut port_vel: Vec<f64> = Vec::with_capacity(n);
    let mut port_disp: Vec<f64> = Vec::with_capacity(n);'''),
    ('''        if has_port {
            if let Some((u_p, area)) = enclosure.port_flow(driver, p_node, omega) {
                let v = if area > 0.0 { u_p.norm() / area } else { 0.0 };
                port_vel.push(v);
            } else {
                port_vel.push(0.0);
            }
        }''',
     '''        if has_port {
            if let Some((u_p, area)) = enclosure.port_flow(driver, p_node, omega) {
                let v = if area > 0.0 { u_p.norm() / area } else { 0.0 };
                port_vel.push(v);
                let disp = if area > 0.0 { u_p.norm() / (omega * area) * 1.0e3 } else { 0.0 };
                port_disp.push(disp);
            } else {
                port_vel.push(0.0);
                port_disp.push(0.0);
            }
        }'''),
    ('''        port_vel_m_s: has_port.then_some(port_vel),
        group_delay_ms,''',
     '''        port_vel_m_s: has_port.then_some(port_vel),
        port_disp_mm: has_port.then_some(port_disp),
        group_delay_ms,'''),
    ('''    #[test]
    fn baffle_step_shape() {''',
     '''    #[test]
    fn pr_excursion_peaks_at_tuning() {
        // На настройке ход ПИ доминирует, ход диффузора минимален.
        let d = Driver::default();
        let b = crate::passive::PassiveBox::default();
        let curves = simulate(&d, &b, &SimConfig::default());
        let disp = curves.port_disp_mm.expect("ход ПИ");
        let fb = b.tuning_hz();
        let at = |arr: &[f64], f: f64| -> f64 {
            let i = curves
                .freq
                .iter()
                .enumerate()
                .min_by(|a, c| (a.1 - f).abs().total_cmp(&(c.1 - f).abs()))
                .unwrap()
                .0;
            arr[i]
        };
        assert!(
            at(&disp, fb) > 3.0 * at(&disp, fb * 2.0),
            "ход ПИ на настройке должен доминировать"
        );
        assert!(at(&curves.excursion_mm, fb) < at(&curves.excursion_mm, fb * 0.7));
    }

    #[test]
    fn baffle_step_shape() {'''),
])
print('response:', ok)
