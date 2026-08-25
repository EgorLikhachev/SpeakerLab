//! Сохранение/загрузка проектов (.spkproj — JSON).

use serde::{Deserialize, Serialize};
use std::path::Path;

use speakerlab_acoustics::bandpass::{Bandpass4, Bandpass6};
use speakerlab_acoustics::driver::Driver;
use speakerlab_acoustics::line::LineBox;
use speakerlab_acoustics::passive::PassiveBox;
use speakerlab_acoustics::response::SimConfig;
use speakerlab_acoustics::sealed::SealedBox;
use speakerlab_acoustics::vented::VentedBox;

use crate::state::{App, EnclosureKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Project {
    pub format_version: u32,
    pub driver: Driver,
    pub kind: EnclosureKind,
    pub sealed: SealedBox,
    pub vented: VentedBox,
    pub passive: PassiveBox,
    pub bp4: Bandpass4,
    pub bp6: Bandpass6,
    pub line: LineBox,
    pub sim: SimConfig,
    pub lang: String,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            format_version: 1,
            driver: Driver::default(),
            kind: EnclosureKind::Vented,
            sealed: SealedBox::default(),
            vented: VentedBox::default(),
            passive: PassiveBox::default(),
            bp4: Bandpass4::default(),
            bp6: Bandpass6::default(),
            line: LineBox::default(),
            sim: SimConfig::default(),
            lang: "ru".into(),
        }
    }
}

impl Project {
    pub fn from_app(app: &App) -> Self {
        Self {
            format_version: 1,
            driver: app.driver.clone(),
            kind: app.kind,
            sealed: app.sealed.clone(),
            vented: app.vented.clone(),
            passive: app.passive.clone(),
            bp4: app.bp4.clone(),
            bp6: app.bp6.clone(),
            line: app.line.clone(),
            sim: app.sim.clone(),
            lang: app.lang.clone(),
        }
    }

    pub fn apply_to(self, app: &mut App) {
        let lang = self.lang.clone();
        app.driver = self.driver;
        app.kind = self.kind;
        app.sealed = self.sealed;
        app.vented = self.vented;
        app.passive = self.passive;
        app.bp4 = self.bp4;
        app.bp6 = self.bp6;
        app.line = self.line;
        app.sim = self.sim;
        if lang != app.lang {
            app.set_lang(&lang);
        }
        app.mark_dirty();
        app.modified = false;
    }
}

pub fn save(app: &App, path: &Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(&Project::from_app(app))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

pub fn load(path: &Path) -> std::io::Result<Project> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Экспорт текущих кривых в CSV.
pub fn export_csv(app: &App, path: &Path) -> std::io::Result<()> {
    use std::fmt::Write as _;
    let Some(c) = &app.curves else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no curves",
        ));
    };
    let mut out =
        String::from("freq_hz;spl_db;impedance_ohm;phase_deg;excursion_mm;group_delay_ms");
    let vel = c.port_vel_m_s.as_ref();
    if vel.is_some() {
        out.push_str(";port_velocity_m_s");
    }
    out.push('\n');
    for i in 0..c.freq.len() {
        let _ = writeln!(
            out,
            "{:.2};{:.2};{:.3};{:.1};{:.4};{:.3}{}",
            c.freq[i],
            c.spl[i],
            c.z_mag[i],
            c.z_phase[i],
            c.excursion_mm[i],
            c.group_delay_ms[i],
            vel.map(|v| format!(";{:.3}", v[i])).unwrap_or_default(),
        );
    }
    std::fs::write(path, out)
}
