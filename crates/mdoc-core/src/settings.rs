//! Gemeinsame App-Einstellungen unter `%APPDATA%\MDoc\settings.json`.
//!
//! Migriert einmalig aus Legacy-Dateien `ui-language` und `ribbon-placement`.

use std::fs;
use std::io;

use serde::{Deserialize, Serialize};

use crate::paths::{
    config_dir, ribbon_placement_path, settings_path, ui_language_path,
};

/// Persistierte Einstellungen (Editor + Word-Add-in).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ribbon_placement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_percent: Option<u8>,
}

impl AppSettings {
    #[must_use]
    pub fn resolved_ui_language(&self) -> Option<&str> {
        self.ui_language.as_deref().and_then(normalize_ui_language)
    }

    #[must_use]
    pub fn resolved_ribbon_placement(&self) -> &str {
        self.ribbon_placement
            .as_deref()
            .and_then(normalize_ribbon_placement)
            .unwrap_or("home")
    }

    #[must_use]
    pub fn resolved_split_percent(&self) -> u8 {
        self.split_percent
            .map(clamp_split_percent)
            .unwrap_or(50)
    }
}

/// Liest Settings inkl. Legacy-Migration.
#[must_use]
pub fn load_app_settings() -> AppSettings {
    match read_settings_file() {
        Ok(Some(settings)) => settings,
        Ok(None) | Err(_) => migrate_legacy_settings(),
    }
}

/// Speichert Settings und schreibt `settings.json`.
pub fn save_app_settings(settings: &AppSettings) -> io::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    fs::write(path, format!("{json}\n"))
}

/// Lädt, setzt die UI-Sprache und speichert.
pub fn save_ui_language(lang: &str) -> io::Result<()> {
    let Some(normalized) = normalize_ui_language(lang) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ui-language must be de or en",
        ));
    };
    let mut settings = load_app_settings();
    settings.ui_language = Some(normalized.to_string());
    save_app_settings(&settings)
}

/// Lädt, setzt die Ribbon-Platzierung und speichert.
pub fn save_ribbon_placement(placement: &str) -> io::Result<()> {
    let Some(normalized) = normalize_ribbon_placement(placement) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ribbon-placement must be home or ownTab",
        ));
    };
    let mut settings = load_app_settings();
    settings.ribbon_placement = Some(normalized.to_string());
    save_app_settings(&settings)
}

/// Lädt, setzt den Split-Anteil und speichert.
pub fn save_split_percent(percent: u8) -> io::Result<()> {
    let mut settings = load_app_settings();
    settings.split_percent = Some(clamp_split_percent(percent));
    save_app_settings(&settings)
}

#[must_use]
pub fn load_ui_language() -> Option<String> {
    load_app_settings()
        .resolved_ui_language()
        .map(str::to_string)
}

#[must_use]
pub fn load_ribbon_placement() -> Option<String> {
    let settings = load_app_settings();
    settings
        .ribbon_placement
        .as_deref()
        .and_then(normalize_ribbon_placement)
        .map(str::to_string)
}

#[must_use]
pub fn load_split_percent() -> u8 {
    load_app_settings().resolved_split_percent()
}

fn read_settings_file() -> io::Result<Option<AppSettings>> {
    let path = settings_path();
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    let settings: AppSettings = serde_json::from_str(&text)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(settings))
}

fn migrate_legacy_settings() -> AppSettings {
    let mut settings = AppSettings::default();
    if let Some(lang) = read_legacy_ui_language() {
        settings.ui_language = Some(lang);
    }
    if let Some(placement) = read_legacy_ribbon_placement() {
        settings.ribbon_placement = Some(placement);
    }
    if settings.ui_language.is_some() || settings.ribbon_placement.is_some() {
        let _ = save_app_settings(&settings);
        let _ = fs::create_dir_all(config_dir());
    }
    settings
}

fn read_legacy_ui_language() -> Option<String> {
    let text = fs::read_to_string(ui_language_path()).ok()?;
    let lang = text.lines().next().unwrap_or("").trim();
    normalize_ui_language(lang).map(str::to_string)
}

fn read_legacy_ribbon_placement() -> Option<String> {
    let text = fs::read_to_string(ribbon_placement_path()).ok()?;
    let raw = text.lines().next().unwrap_or("").trim();
    normalize_ribbon_placement(raw).map(str::to_string)
}

fn normalize_ui_language(value: &str) -> Option<&'static str> {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "de" => Some("de"),
        "en" => Some("en"),
        other if other.starts_with("de-") || other.starts_with("de_") => Some("de"),
        other if other.starts_with("en-") || other.starts_with("en_") => Some("en"),
        _ => None,
    }
}

fn normalize_ribbon_placement(value: &str) -> Option<&'static str> {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "home" => Some("home"),
        "owntab" | "tab" | "mdoc" | "mdword" => Some("ownTab"),
        _ => None,
    }
}

fn clamp_split_percent(value: u8) -> u8 {
    value.clamp(20, 80)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_language_and_ribbon() {
        assert_eq!(normalize_ui_language("DE"), Some("de"));
        assert_eq!(normalize_ui_language("de-DE"), Some("de"));
        assert_eq!(normalize_ribbon_placement("ownTab"), Some("ownTab"));
        assert_eq!(normalize_ribbon_placement("tab"), Some("ownTab"));
        assert_eq!(clamp_split_percent(10), 20);
        assert_eq!(clamp_split_percent(90), 80);
    }
}
