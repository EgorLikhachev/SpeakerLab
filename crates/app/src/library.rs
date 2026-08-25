//! Личная библиотека динамиков (JSON-файлы в каталоге данных приложения)
//! и настройки.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

use speakerlab_acoustics::driver::Driver;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub lang: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { lang: "ru".into() }
    }
}

/// Каталог данных приложения (ОС-зависимый).
pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "SpeakerLab")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn drivers_dir() -> PathBuf {
    data_dir().join("drivers")
}

pub fn load_settings() -> Settings {
    let path = data_dir().join("settings.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_settings(lang: &str) {
    let s = Settings {
        lang: lang.to_string(),
    };
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(&s) {
        let _ = std::fs::write(dir.join("settings.json"), json);
    }
}

/// Имя файла для динамика: «Производитель Модель.json» (санитизированное).
fn driver_file_name(d: &Driver) -> String {
    let raw = if d.manufacturer.is_empty() {
        d.name.clone()
    } else {
        format!("{} {}", d.manufacturer, d.name)
    };
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '.' || c == '"' || c == '\'' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim().replace(' ', "_");
    if trimmed.is_empty() {
        "driver".to_string()
    } else {
        trimmed
    }
}

pub fn load_library() -> Vec<Driver> {
    let dir = drivers_dir();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "json").unwrap_or(false) {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    if let Ok(d) = serde_json::from_str::<Driver>(&text) {
                        out.push(d);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Сохранить/обновить динамик в библиотеке.
pub fn save_driver(d: &Driver) -> io::Result<PathBuf> {
    let dir = drivers_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", driver_file_name(d)));
    let json = serde_json::to_string_pretty(d)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)?;
    Ok(path)
}

pub fn delete_driver(d: &Driver) -> io::Result<()> {
    let path = drivers_dir().join(format!("{}.json", driver_file_name(d)));
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn import_driver(path: &std::path::Path) -> Option<Driver> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn export_driver(d: &Driver, path: &std::path::Path) -> io::Result<()> {
    let json = serde_json::to_string_pretty(d)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}
