//! Подсказки оформлений по T/S-параметрам (стартовые точки, не догма).

use crate::driver::Driver;
use crate::sealed::SealedBox;

#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Ключ локализации названия
    pub label_key: &'static str,
    /// Объём, л
    pub vb: f64,
    /// Частота настройки (ФИ/ПИ), Гц
    pub fb: Option<f64>,
}

/// Варианты ЗЯ под типовые Qtc.
pub fn sealed_suggestions(driver: &Driver) -> Vec<Suggestion> {
    let mut out = Vec::new();
    for (key, qtc) in [
        ("suggest.sealed.qtc0577", 0.577),
        ("suggest.sealed.qtc0707", 0.707),
        ("suggest.sealed.qtc09", 0.90),
        ("suggest.sealed.qtc12", 1.20),
    ] {
        if let Some(vb) = SealedBox::vb_for_qtc(driver, qtc) {
            out.push(Suggestion {
                label_key: key,
                vb,
                fb: None,
            });
        }
    }
    out
}

/// Варианты ФИ. Аппроксимации «максимально плоской» настройки:
/// Vb ≈ 15·Vas·Qts^2.87, Fb ≈ 0.42·Fs/Qts^0.9 (корректна при 0.2 < Qts < 0.55).
pub fn vented_suggestions(driver: &Driver) -> Vec<Suggestion> {
    let qts = driver.qts();
    if !(0.15..0.65).contains(&qts) {
        return Vec::new();
    }
    let vb_flat = 15.0 * driver.vas * qts.powf(2.87);
    let fb_flat = 0.42 * driver.fs / qts.powf(0.9);
    vec![
        Suggestion {
            label_key: "suggest.vented.flat",
            vb: vb_flat,
            fb: Some(fb_flat),
        },
        Suggestion {
            label_key: "suggest.vented.compact",
            vb: 0.7 * vb_flat,
            fb: Some(1.1 * fb_flat),
        },
        Suggestion {
            label_key: "suggest.vented.ebs",
            vb: 1.8 * vb_flat,
            fb: Some(0.85 * fb_flat),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_suggestions_hit_targets() {
        let d = Driver::default();
        let sugg = sealed_suggestions(&d);
        assert!(sugg.len() >= 3);
        let b = SealedBox {
            vb: sugg[1].vb,
            ..Default::default()
        };
        assert!((b.qtc(&d) - 0.707).abs() < 1e-6);
    }

    #[test]
    fn vented_suggestions_reasonable() {
        let d = Driver::default(); // Qts ≈ 0.386
        let sugg = vented_suggestions(&d);
        assert_eq!(sugg.len(), 3);
        let s = &sugg[0];
        assert!(s.vb > 10.0 && s.vb < 300.0, "Vb_flat = {:.1} л", s.vb);
        let fb = s.fb.unwrap();
        assert!(fb > 15.0 && fb < 60.0, "Fb_flat = {fb:.1} Гц");
    }
}
