//! Состояние приложения и живой пересчёт.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use speakerlab_acoustics::bandpass::{Bandpass4, Bandpass6};
use speakerlab_acoustics::circuit::EnclosureModel;
use speakerlab_acoustics::driver::Driver;
use speakerlab_acoustics::line::LineBox;
use speakerlab_acoustics::passive::PassiveBox;
use speakerlab_acoustics::response::{
    compute_limits, simulate, summarize, Curves, Limits, SimConfig, Summary,
};
use speakerlab_acoustics::sealed::SealedBox;
use speakerlab_acoustics::vented::VentedBox;

use crate::library;
use crate::ui::box_calc::BoxCalcState;
use crate::ui::port_calc::PortCalc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnclosureKind {
    #[serde(rename = "sealed")]
    Sealed,
    #[serde(rename = "vented")]
    Vented,
    #[serde(rename = "passive")]
    Passive,
    #[serde(rename = "bandpass4")]
    Bandpass4,
    #[serde(rename = "bandpass6")]
    Bandpass6,
    #[serde(rename = "line")]
    Line,
}

pub struct App {
    pub lang: String,
    pub driver: Driver,
    pub kind: EnclosureKind,
    pub sealed: SealedBox,
    pub vented: VentedBox,
    pub passive: PassiveBox,
    pub bp4: Bandpass4,
    pub bp6: Bandpass6,
    pub line: LineBox,
    pub sim: SimConfig,
    /// Кэш кривых (пересчитывается при dirty)
    pub curves: Option<Curves>,
    pub summary: Option<Summary>,
    /// Предельные напряжения (Xmax / порт / мощность)
    pub limits: Option<Limits>,
    dirty: bool,
    /// Есть несохранённые изменения (для метки в заголовке)
    pub modified: bool,
    pub project_path: Option<PathBuf>,
    pub library: Vec<Driver>,
    // окна
    pub port_calc: PortCalc,
    pub box_calc: BoxCalcState,
    pub show_library: bool,
    pub library_selected: Option<usize>,
    pub plot_tab: crate::ui::plots::PlotTab,
    /// Эталонные кривые для сравнения (пунктиром на графиках)
    pub ref_curves: Option<(String, Curves)>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            lang: library::load_settings().lang,
            driver: Driver::default(),
            kind: EnclosureKind::Vented,
            sealed: SealedBox::default(),
            vented: VentedBox::default(),
            passive: PassiveBox::default(),
            bp4: Bandpass4::default(),
            bp6: Bandpass6::default(),
            line: LineBox::default(),
            sim: SimConfig::default(),
            curves: None,
            summary: None,
            limits: None,
            dirty: true,
            modified: false,
            project_path: None,
            library: library::load_library(),
            port_calc: PortCalc::default(),
            box_calc: BoxCalcState::default(),
            show_library: false,
            library_selected: None,
            plot_tab: Default::default(),
            ref_curves: None,
        };
        rust_i18n::set_locale(app.lang.as_str());
        app.ensure_computed();
        app
    }

    /// Активная модель оформления.
    pub fn model(&self) -> &dyn EnclosureModel {
        match self.kind {
            EnclosureKind::Sealed => &self.sealed,
            EnclosureKind::Vented => &self.vented,
            EnclosureKind::Passive => &self.passive,
            EnclosureKind::Bandpass4 => &self.bp4,
            EnclosureKind::Bandpass6 => &self.bp6,
            EnclosureKind::Line => &self.line,
        }
    }

    /// Пометить состояние изменёншимся — на следующем кадре всё пересчитается.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.modified = true;
    }

    /// Живой пересчёт: вызывается каждый кадр, работает только при изменениях.
    pub fn ensure_computed(&mut self) {
        if !self.dirty {
            return;
        }
        if !self.driver.is_valid() {
            self.curves = None;
            self.summary = None;
            return; // остаёмся dirty: параметры могут стать валидными снова
        }
        let curves = simulate(&self.driver, self.model(), &self.sim);
        let summary = summarize(&curves, self.tuning_hz());
        self.limits = Some(compute_limits(
            &curves,
            self.sim.voltage,
            self.driver.xmax,
            self.driver.pe,
            summary.z_min,
        ));
        self.summary = Some(summary);
        self.curves = Some(curves);
        self.dirty = false;
    }

    /// Новый проект (значения по умолчанию).
    pub fn reset(&mut self) {
        self.driver = Driver::default();
        self.kind = EnclosureKind::Vented;
        self.sealed = SealedBox::default();
        self.vented = VentedBox::default();
        self.passive = PassiveBox::default();
        self.bp4 = Bandpass4::default();
        self.bp6 = Bandpass6::default();
        self.line = LineBox::default();
        self.sim = SimConfig::default();
        self.project_path = None;
        self.ref_curves = None;
        self.modified = false;
        self.mark_dirty();
    }

    pub fn set_lang(&mut self, lang: &str) {
        self.lang = lang.to_string();
        rust_i18n::set_locale(lang);
        library::save_settings(&self.lang);
    }

    /// Заголовок окна.
    pub fn title(&self) -> String {
        let dot = if self.modified { " •" } else { "" };
        match &self.project_path {
            Some(p) => format!(
                "SpeakerLab — {}{dot}",
                p.file_stem()
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_default()
            ),
            None => format!("SpeakerLab{dot}"),
        }
    }

    /// Частота настройки текущего оформления (для метрики «ход @ Fb»).
    pub fn tuning_hz(&self) -> Option<f64> {
        match self.kind {
            EnclosureKind::Sealed | EnclosureKind::Line => None,
            EnclosureKind::Vented => Some(self.vented.fb),
            EnclosureKind::Passive => Some(self.passive.tuning_hz()),
            EnclosureKind::Bandpass4 => Some(self.bp4.fb),
            EnclosureKind::Bandpass6 => Some(self.bp6.fb_front),
        }
    }

    /// Чистый объём текущего оформления (для калькулятора размеров), л.
    pub fn net_volume(&self) -> f64 {
        match self.kind {
            EnclosureKind::Sealed => self.sealed.vb,
            EnclosureKind::Vented => self.vented.vb,
            EnclosureKind::Passive => self.passive.vb,
            EnclosureKind::Bandpass4 => self.bp4.vb_rear + self.bp4.vb_front,
            EnclosureKind::Bandpass6 => self.bp6.vb_rear + self.bp6.vb_front,
            EnclosureKind::Line => self.line.volume_l(),
        }
    }
}
