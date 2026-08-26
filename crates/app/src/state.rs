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

use rust_i18n::t;

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
    /// Всплывающие уведомления (текст, время создания)
    pub toasts: Vec<(String, std::time::Instant)>,
    /// Фильтр поиска в библиотеке динамиков
    pub lib_filter: String,
    /// Ширина передней панели для baffle step, м (0 — не учитывать)
    pub baffle_m: f64,
    /// Прямоугольник последнего графика (экранные коорд.) для PNG-экспорта
    pub plot_rect: Option<egui::Rect>,
    /// Путь для сохранения PNG (взят диалогом, ждём скриншот)
    pub png_path: Option<std::path::PathBuf>,
    /// Показать окно таблицы кривых
    pub show_table: bool,
}

/// Сохраняемое между запусками состояние (кроме размеров окна — их
/// eframe persistence хранит сам).
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct AppPersist {
    pub lang: Option<String>,
    pub voltage: Option<f64>,
    /// Индекс вкладки графика (0..5)
    pub plot_tab: Option<u8>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let persisted: Option<AppPersist> = cc
            .storage
            .and_then(|st| eframe::get_value(st, eframe::APP_KEY));
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
            toasts: Vec::new(),
            lib_filter: String::new(),
            baffle_m: 0.0,
            plot_rect: None,
            png_path: None,
            show_table: false,
        };
        if let Some(p) = persisted {
            if let Some(lang) = p.lang {
                if lang == "ru" || lang == "en" {
                    app.lang = lang;
                }
            }
            if let Some(v) = p.voltage {
                if v.is_finite() && (0.1..=200.0).contains(&v) {
                    app.sim.voltage = v;
                }
            }
            if let Some(tab) = p.plot_tab {
                app.plot_tab = match tab {
                    1 => crate::ui::plots::PlotTab::Impedance,
                    2 => crate::ui::plots::PlotTab::Phase,
                    3 => crate::ui::plots::PlotTab::Excursion,
                    4 => crate::ui::plots::PlotTab::PortVel,
                    5 => crate::ui::plots::PlotTab::GroupDelay,
                    _ => crate::ui::plots::PlotTab::Spl,
                };
            }
        }
        rust_i18n::set_locale(app.lang.as_str());
        app.ensure_computed();
        app
    }

    /// Сохранение состояния (вызывается eframe при выходе).
    pub fn persist(&self, storage: &mut dyn eframe::Storage) {
        let tab = match self.plot_tab {
            crate::ui::plots::PlotTab::Spl => 0,
            crate::ui::plots::PlotTab::Impedance => 1,
            crate::ui::plots::PlotTab::Phase => 2,
            crate::ui::plots::PlotTab::Excursion => 3,
            crate::ui::plots::PlotTab::PortVel => 4,
            crate::ui::plots::PlotTab::GroupDelay => 5,
        };
        eframe::set_value(
            storage,
            eframe::APP_KEY,
            &AppPersist {
                lang: Some(self.lang.clone()),
                voltage: Some(self.sim.voltage),
                plot_tab: Some(tab),
            },
        );
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

    /// Обработка ответа скриншота окна: кроп по графику и сохранение PNG.
    pub fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let events: Vec<egui::Event> = ctx.input(|i| i.events.clone());
        for ev in events {
            if let egui::Event::Screenshot { image, .. } = ev {
                let (Some(path), Some(rect)) = (self.png_path.take(), self.plot_rect) else {
                    continue;
                };
                let scale = ctx.pixels_per_point();
                let (w, h) = (image.width() as i32, image.height() as i32);
                let clamp = |v: i32, hi: i32| v.clamp(0, hi);
                let x0 = clamp((rect.left() * scale) as i32, w);
                let y0 = clamp((rect.top() * scale) as i32, h);
                let x1 = clamp((rect.right() * scale) as i32, w);
                let y1 = clamp((rect.bottom() * scale) as i32, h);
                if x1 <= x0 || y1 <= y0 {
                    self.push_toast(t!("png.empty").to_string());
                    continue;
                }
                let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                match image::RgbaImage::from_raw(w as u32, h as u32, rgba) {
                    Some(full) => {
                        let cropped = image::imageops::crop_imm(
                            &full,
                            x0 as u32,
                            y0 as u32,
                            (x1 - x0) as u32,
                            (y1 - y0) as u32,
                        )
                        .to_image();
                        match cropped.save_with_format(&path, image::ImageFormat::Png) {
                            Ok(()) => {
                                self.push_toast(format!("{}: {}", t!("png.saved"), path.display()))
                            }
                            Err(e) => {
                                self.push_toast(format!("{}: {e}", t!("png.error")));
                            }
                        }
                    }
                    None => self.push_toast(t!("png.bad_buffer").to_string()),
                }
            }
        }
    }

    /// Показать всплывающее уведомление (5 секунд).
    pub fn push_toast(&mut self, msg: String) {
        self.toasts.push((msg, std::time::Instant::now()));
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
        let mut sim = self.sim.clone();
        sim.baffle_width_m = if self.baffle_m > 0.01 {
            Some(self.baffle_m)
        } else {
            None
        };
        let curves = simulate(&self.driver, self.model(), &sim);
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
