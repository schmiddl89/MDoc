use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG_DIR_NAME: &str = "MDoc";
/// Ältere Schreibweisen unter `%APPDATA%` (Windows: Groß/Klein oft gleich).
const LEGACY_CONFIG_DIR_NAMES: &[&str] = &["mdoc", "mdword"];

/// `%APPDATA%\MDoc` – gemeinsame Config für Editor und Word-Plugin.
///
/// Migriert einmalig von `mdoc` / `mdword` und korrigiert die Ordner-Schreibweise.
#[must_use]
pub fn config_dir() -> PathBuf {
    let Some(base) = dirs_appdata() else {
        return PathBuf::from(CONFIG_DIR_NAME);
    };
    let preferred = base.join(CONFIG_DIR_NAME);
    migrate_legacy_config_dir(&base, &preferred);
    normalize_dir_casing(&base, CONFIG_DIR_NAME);
    preferred
}

fn migrate_legacy_config_dir(base: &Path, preferred: &Path) {
    if preferred.is_dir() {
        return;
    }
    for &legacy_name in LEGACY_CONFIG_DIR_NAMES {
        if legacy_name.eq_ignore_ascii_case(CONFIG_DIR_NAME) {
            continue;
        }
        let legacy = base.join(legacy_name);
        if !legacy.is_dir() {
            continue;
        }
        match fs::rename(&legacy, preferred) {
            Ok(()) => return,
            Err(_) => {
                // Dateisperre o. ä.: Legacy weiter nutzen, bis Migration klappt.
                return;
            }
        }
    }
}

/// Stellt die gewünschte Schreibweise her (z. B. `mdoc` → `MDoc` unter Windows).
fn normalize_dir_casing(parent: &Path, desired_name: &str) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if !name_str.eq_ignore_ascii_case(desired_name) || name_str == desired_name {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let from = entry.path();
        let temp = parent.join(format!("{desired_name}.__casing__"));
        let to = parent.join(desired_name);
        if fs::rename(&from, &temp).is_ok() {
            let _ = fs::rename(&temp, &to);
        }
        return;
    }
}

/// Geteiltes Style-Profil (JSON), von Desktop und Word-Plugin genutzt.
#[must_use]
pub fn shared_style_profile_path() -> PathBuf {
    config_dir().join("style-profile.json")
}

/// UI-Sprache (`de` / `en`), gemeinsam für Editor und Word-Add-in.
#[must_use]
pub fn ui_language_path() -> PathBuf {
    config_dir().join("ui-language")
}

/// Liest die UI-Sprache; ungültig/fehlend → `None`.
#[must_use]
pub fn load_ui_language() -> Option<String> {
    let text = fs::read_to_string(ui_language_path()).ok()?;
    let lang = text.lines().next().unwrap_or("").trim().to_ascii_lowercase();
    matches!(lang.as_str(), "de" | "en").then_some(lang)
}

/// Speichert die UI-Sprache (`de` oder `en`).
pub fn save_ui_language(lang: &str) -> std::io::Result<()> {
    let normalized = lang.trim().to_ascii_lowercase();
    if normalized != "de" && normalized != "en" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "ui-language must be de or en",
        ));
    }
    let path = ui_language_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{normalized}\n"))
}

/// Ribbon-Platzierung (`home` / `ownTab`), gemeinsam für Editor und Word-Add-in.
#[must_use]
pub fn ribbon_placement_path() -> PathBuf {
    config_dir().join("ribbon-placement")
}

/// Liest die Ribbon-Platzierung; ungültig/fehlend → `None` (Add-in default: `home`).
#[must_use]
pub fn load_ribbon_placement() -> Option<String> {
    let text = fs::read_to_string(ribbon_placement_path()).ok()?;
    let raw = text.lines().next().unwrap_or("").trim().to_ascii_lowercase();
    match raw.as_str() {
        "home" => Some("home".to_string()),
        "owntab" | "tab" | "mdoc" | "mdword" => Some("ownTab".to_string()),
        _ => None,
    }
}

/// Speichert die Ribbon-Platzierung (`home` oder `ownTab`).
pub fn save_ribbon_placement(placement: &str) -> std::io::Result<()> {
    let normalized = match placement.trim().to_ascii_lowercase().as_str() {
        "home" => "home",
        "owntab" | "tab" | "mdoc" | "mdword" => "ownTab",
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "ribbon-placement must be home or ownTab",
            ));
        }
    };
    let path = ribbon_placement_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{normalized}\n"))
}

/// Datei mit dem absoluten Pfad der im Editor gewählten Word-Vorlage (eine Zeile).
#[must_use]
pub fn shared_template_path_file() -> PathBuf {
    config_dir().join("template-path.txt")
}

/// Gecachte `styles.xml` der Editor-Vorlage (Fallback, wenn Word die `.dotm` sperrt).
#[must_use]
pub fn shared_template_styles_path() -> PathBuf {
    config_dir().join("template-styles.xml")
}

/// Gecachte `numbering.xml` der Editor-Vorlage (Outline-Nummerierung der Überschriften).
#[must_use]
pub fn shared_template_numbering_path() -> PathBuf {
    config_dir().join("template-numbering.xml")
}

/// Metadaten zum Vorlagen-Cache: Pfad, mtime (Unix-ms) und Größe der Vorlage.
#[must_use]
pub fn shared_template_styles_meta_path() -> PathBuf {
    config_dir().join("template-styles.meta")
}

/// Fingerprint der Vorlagendatei (ohne Inhalt zu lesen).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFileFingerprint {
    pub path: PathBuf,
    pub mtime_unix_ms: i64,
    pub size: u64,
    /// Ob beim letzten Cache-Lauf `numbering.xml` vorhanden war.
    pub numbering_present: bool,
}

impl TemplateFileFingerprint {
    /// Liest mtime + Größe per `metadata` (auch wenn die Datei zum Lesen gesperrt ist).
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let meta = fs::metadata(path)?;
        let mtime_unix_ms = system_time_to_unix_ms(meta.modified()?);
        Ok(Self {
            path: path.to_owned(),
            mtime_unix_ms,
            size: meta.len(),
            numbering_present: false,
        })
    }

    fn matches_file(&self, path: &Path) -> bool {
        match Self::from_path(path) {
            Ok(current) => {
                paths_equal(&self.path, path)
                    && self.mtime_unix_ms == current.mtime_unix_ms
                    && self.size == current.size
            }
            Err(_) => false,
        }
    }
}

/// Liest die gemeinsam hinterlegte Vorlagen-Datei, falls vorhanden und gültig.
#[must_use]
pub fn load_shared_template_path() -> Option<PathBuf> {
    let file = shared_template_path_file();
    let text = fs::read_to_string(&file).ok()?;
    let path = PathBuf::from(text.lines().next().unwrap_or("").trim());
    path.is_file().then_some(path)
}

/// Speichert den Vorlagen-Pfad und aktualisiert Styles-Cache + Meta (oder löscht alles bei `None`).
pub fn save_shared_template_path(path: Option<&Path>) -> std::io::Result<()> {
    let file = shared_template_path_file();
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    match path {
        Some(path) if path.is_file() => {
            fs::write(&file, format!("{}\n", path.display()))?;
            write_styles_cache(path)?;
        }
        Some(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Vorlagendatei existiert nicht",
            ));
        }
        None => {
            remove_if_exists(&file)?;
            clear_styles_cache()?;
        }
    }
    Ok(())
}

/// Prüft mtime+Größe; liest die Vorlage nur bei Änderung / fehlendem Cache erneut.
///
/// Bei Dateisperre bleibt ein vorhandener Cache erhalten (`Ok(())`).
/// Fehlt jeder Cache und Lesen scheitert, wird der Fehler durchgereicht.
pub fn ensure_shared_template_styles_cache() -> std::io::Result<()> {
    let Some(template) = load_shared_template_path() else {
        return Ok(());
    };

    if styles_cache_is_fresh(&template) {
        return Ok(());
    }

    match write_styles_cache(&template) {
        Ok(()) => Ok(()),
        Err(error) if shared_template_styles_path().is_file() => {
            // Gesperrt o. ä.: alter Cache bleibt nutzbar.
            let _ = error;
            Ok(())
        }
        Err(error) => Err(error),
    }
}

/// Word-Benutzer-`Normal.dotm`, falls vorhanden.
#[must_use]
pub fn normal_dotm_path() -> Option<PathBuf> {
    let path = dirs_appdata()?.join("Microsoft").join("Templates").join("Normal.dotm");
    path.is_file().then_some(path)
}

/// Effektive Vorlage: expliziter Pfad, sonst gemeinsame Editor-Vorlage, sonst `Normal.dotm`.
#[must_use]
pub fn resolve_template(explicit: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = explicit.filter(|path| path.is_file()) {
        return Some(path);
    }
    if let Some(path) = load_shared_template_path() {
        return Some(path);
    }
    normal_dotm_path()
}

fn styles_cache_is_fresh(template: &Path) -> bool {
    if !shared_template_styles_path().is_file() {
        return false;
    }
    // Ältere Caches ohne numbering.xml einmalig erneuern (Überschriften-Outline).
    let meta = match load_styles_meta() {
        Some(meta) => meta,
        None => return false,
    };
    if !meta.matches_file(template) {
        return false;
    }
    meta.numbering_present == shared_template_numbering_path().is_file()
}

fn write_styles_cache(template: &Path) -> std::io::Result<()> {
    let styles_cache = shared_template_styles_path();
    let numbering_cache = shared_template_numbering_path();
    crate::template::cache_template_parts(template, &styles_cache, &numbering_cache).map_err(
        |error| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Vorlagen-Teile konnten nicht aus {} gelesen werden: {error}",
                    template.display()
                ),
            )
        },
    )?;
    let mut fingerprint = TemplateFileFingerprint::from_path(template)?;
    fingerprint.numbering_present = numbering_cache.is_file();
    write_styles_meta(&fingerprint)
}

fn clear_styles_cache() -> std::io::Result<()> {
    remove_if_exists(&shared_template_styles_path())?;
    remove_if_exists(&shared_template_numbering_path())?;
    remove_if_exists(&shared_template_styles_meta_path())?;
    Ok(())
}

fn load_styles_meta() -> Option<TemplateFileFingerprint> {
    let text = fs::read_to_string(shared_template_styles_meta_path()).ok()?;
    parse_styles_meta(&text)
}

fn write_styles_meta(fingerprint: &TemplateFileFingerprint) -> std::io::Result<()> {
    let meta_path = shared_template_styles_meta_path();
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = format!(
        "path={}\nmtime_unix_ms={}\nsize={}\nnumbering={}\n",
        fingerprint.path.display(),
        fingerprint.mtime_unix_ms,
        fingerprint.size,
        if fingerprint.numbering_present { "1" } else { "0" }
    );
    fs::write(meta_path, body)
}

fn parse_styles_meta(text: &str) -> Option<TemplateFileFingerprint> {
    let mut path = None;
    let mut mtime_unix_ms = None;
    let mut size = None;
    let mut numbering_present = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("path=") {
            path = Some(PathBuf::from(value.trim()));
        } else if let Some(value) = line.strip_prefix("mtime_unix_ms=") {
            mtime_unix_ms = value.trim().parse().ok();
        } else if let Some(value) = line.strip_prefix("size=") {
            size = value.trim().parse().ok();
        } else if let Some(value) = line.strip_prefix("numbering=") {
            numbering_present = matches!(value.trim(), "1" | "true" | "yes");
        }
    }
    Some(TemplateFileFingerprint {
        path: path?,
        mtime_unix_ms: mtime_unix_ms?,
        size: size?,
        numbering_present,
    })
}

fn system_time_to_unix_ms(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as i64,
        Err(err) => -(err.duration().as_millis() as i64),
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    // Windows: Groß/Klein und Slash-Varianten angleichen.
    let a = a.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
    let b = b.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
    a == b
}

fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn dirs_appdata() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::{TemplateFileFingerprint, parse_styles_meta, resolve_template};
    use std::fs;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[test]
    fn missing_explicit_falls_through_without_panic() {
        let missing = PathBuf::from("Z:\\definitely-missing-mdoc-template.dotm");
        let resolved = resolve_template(Some(missing));
        if let Some(path) = resolved {
            assert!(path.is_file());
        }
    }

    #[test]
    fn parse_styles_meta_roundtrip_keys() {
        let text =
            "path=C:\\Templates\\Corp.dotx\nmtime_unix_ms=1710000000123\nsize=4096\nnumbering=1\n";
        let meta = parse_styles_meta(text).expect("meta");
        assert_eq!(meta.path, PathBuf::from("C:\\Templates\\Corp.dotx"));
        assert_eq!(meta.mtime_unix_ms, 1_710_000_000_123);
        assert_eq!(meta.size, 4096);
        assert!(meta.numbering_present);
    }

    #[test]
    fn fingerprint_detects_size_change() {
        let dir = std::env::temp_dir().join(format!(
            "mdoc-fingerprint-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        let file = dir.join("sample.dotx");
        fs::write(&file, b"pk-fake").expect("write sample");

        let fp = TemplateFileFingerprint::from_path(&file).expect("fingerprint");
        assert!(fp.matches_file(&file));

        fs::write(&file, b"pk-fake-longer").expect("rewrite");
        assert!(!fp.matches_file(&file));

        let _ = fs::remove_dir_all(&dir);
    }
}
