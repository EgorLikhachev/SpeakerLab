//! Тесты app-крейта: паритет локалей, сериализация проекта.

/// Ключи верхнего уровня из YAML-файла локали (простой парсер без зависимостей).
fn locale_keys(text: &str) -> std::collections::HashSet<String> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim_end();
            // строка вида "key: value" или "key: value" без вложенности
            let (key, rest) = l.split_once(':')?;
            if key.contains(' ') || !rest.starts_with(' ') {
                return None;
            }
            Some(key.trim().to_string())
        })
        .collect()
}

#[test]
fn locales_have_identical_keys() {
    let ru = include_str!("../locales/ru.yml");
    let en = include_str!("../locales/en.yml");
    let ru_keys = locale_keys(ru);
    let en_keys = locale_keys(en);
    let mut missing_in_en: Vec<_> = ru_keys.difference(&en_keys).collect();
    let mut missing_in_ru: Vec<_> = en_keys.difference(&ru_keys).collect();
    missing_in_en.sort();
    missing_in_ru.sort();
    assert!(
        missing_in_en.is_empty() && missing_in_ru.is_empty(),
        "нет в en: {missing_in_en:?}; нет в ru: {missing_in_ru:?}"
    );
}

#[test]
fn project_serde_roundtrip() {
    use crate::project::Project;
    let p = Project::default();
    let json = serde_json::to_string_pretty(&p).unwrap();
    let back: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(back.driver.name, p.driver.name);
    assert_eq!(back.kind, p.kind);
    assert!((back.vented.vb - p.vented.vb).abs() < 1e-12);
    assert_eq!(back.line.segments.len(), p.line.segments.len());
}

#[test]
fn project_deserializes_minimal_json() {
    // Старые/минимальные проекты без новых полей должны читаться (serde default)
    let json = r#"{"driver": {"re": 3.2, "fs": 30.0, "qms": 4.5, "qes": 0.42,
                  "vas": 62.0, "sd": 220.0}, "kind": "sealed"}"#;
    let p: crate::project::Project = serde_json::from_str(json)
        .expect("минимальный проект должен читаться через serde(default)");
    assert_eq!(p.kind, crate::state::EnclosureKind::Sealed);
    assert_eq!(p.driver.re, 3.2);
}
