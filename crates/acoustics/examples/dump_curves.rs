//! Дамп эталонных кривых для внешней верификации (Python/таблицы).
//! Запуск: cargo run -p speakerlab-acoustics --example dump_curves > curves.json

use serde_json::json;
use speakerlab_acoustics::circuit::{solve_point, EnclosureModel};
use speakerlab_acoustics::driver::Driver;
use speakerlab_acoustics::port::{EndCorrection, PortGeometry, PortSpec};
use speakerlab_acoustics::response::{compute_limits, simulate, summarize, SimConfig};
use speakerlab_acoustics::sealed::SealedBox;
use speakerlab_acoustics::vented::VentedBox;
use speakerlab_acoustics::TAU;

fn main() {
    let d = Driver::default();
    let mut d0 = d.clone();
    d0.le = 0.0; // версия без индуктивности — для сравнения с аналитикой

    // 1) ЗЯ без потерь: аналитика доступна в замкнутой форме
    let sealed = SealedBox {
        vb: 30.0,
        qa: 1.0e9,
        ..Default::default()
    };
    let cfg = SimConfig::default();
    let curves_sealed = simulate(&d0, &sealed, &cfg);
    let sum_sealed = summarize(&curves_sealed, None);

    // 2) ФИ с потерями Ql=10 (как принято в литературе)
    let vented = VentedBox {
        vb: 50.0,
        fb: 35.0,
        ql: 10.0,
        ..Default::default()
    };
    let curves_vented = simulate(&d0, &vented, &cfg); // Le=0 для сравнения отклика
    let sum_vented = summarize(&curves_vented, Some(vented.fb));

    // импеданс ФИ считаем с реальной Le (для |Z| это важно)
    let z_vented: Vec<(f64, f64, f64)> = curves_vented
        .freq
        .iter()
        .map(|f| {
            let w = TAU * f;
            let za = vented.acoustic_load(&d, w);
            let op = solve_point(&d, za, w, cfg.voltage);
            (*f, op.z_in.norm(), (op.cone_velocity.norm() / w) * 1.0e3)
        })
        .collect();

    // 3) Длина порта D=100 мм
    let spec = PortSpec::new(PortGeometry::Round { diameter_mm: 100.0 });
    let len_cm =
        speakerlab_acoustics::port::port_length_m(50.0, 35.0, &spec, EndCorrection::OneFlanged)
            * 1.0e2;

    // Предельные напряжения ФИ (Le=0 кривые)
    let limits = compute_limits(&curves_vented, cfg.voltage, d.xmax, d.pe, sum_vented.z_min);

    // 4) Эталонная чувствительность по формуле Тиля–Смолла
    let eta0_percent = 9.64e-10 * d.fs.powi(3) * d.vas / d.qes;
    let spl_ref = 112.02 + 10.0 * eta0_percent.log10();

    // средний SPL полосы 200–1000 Гц по кривой без Le (ФИ)
    let band: Vec<f64> = curves_vented
        .freq
        .iter()
        .zip(curves_vented.spl.iter())
        .filter(|(f, _)| **f >= 200.0 && **f <= 1000.0)
        .map(|(_, s)| *s)
        .collect();
    let sim_passband = band.iter().sum::<f64>() / band.len() as f64;

    let out = json!({
        "driver": {
            "re": d.re, "le_mH": d.le, "fs": d.fs, "qms": d.qms, "qes": d.qes,
            "vas_l": d.vas, "sd_cm2": d.sd, "xmax_mm": d.xmax,
            "qts": d.qts(), "bl": d.bl_tm(), "mms_g": d.mms_kg() * 1e3,
            "cms": d.cms(), "rms": d.rms()
        },
        "sealed": {
            "vb_l": sealed.vb, "fc": sealed.fc(&d), "qtc": sealed.qtc(&d),
            "f3": sum_sealed.f3_low,
            "freq": curves_sealed.freq,
            "spl": curves_sealed.spl,
            "alignment_spl": curves_sealed.alignment_spl,
        },
        "vented": {
            "vb_l": vented.vb, "fb": vented.fb, "ql": vented.ql,
            "f3": sum_vented.f3_low,
            "z_max": sum_vented.z_max, "z_max_freq": sum_vented.z_max_freq,
            "exc_max": sum_vented.excursion_max_mm,
            "exc_max_freq": sum_vented.excursion_max_freq,
            "exc_at_fb": sum_vented.excursion_at_tuning,
            "freq": curves_vented.freq,
            "spl": curves_vented.spl,
            "alignment_spl": curves_vented.alignment_spl,
            "excursion_mm": curves_vented.excursion_mm,
            "group_delay_ms": curves_vented.group_delay_ms,
        },
        "limits": {
            "v_xmax": limits.v_xmax,
            "v_thermal": limits.v_thermal,
            "v_limit": limits.v_limit,
            "limiting": format!("{:?}", limits.limiting),
        },
        "xmax_mm": d.xmax,
        "pe_w": d.pe,
        "z_semi": (1..=8)
            .map(|k| {
                let f = 500.0 * k as f64;
                let dm = Driver {
                    le_model: speakerlab_acoustics::driver::LeModel::Semi,
                    ..d.clone()
                };
                (f, dm.voice_coil_impedance(TAU * f).norm())
            })
            .collect::<Vec<(f64, f64)>>(),
        "le_mH2": d.le,
        "z_vented": z_vented,
        "port_len_cm": len_cm,
        "eta0_spl_ref": spl_ref,
        "sim_passband_spl": sim_passband,
        "voltage": cfg.voltage,
    });
    println!("{out}");
}
