use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, WindowEvent};

/// Nach Bestätigung im Frontend: nächstes CloseRequested durchlassen.
static ALLOW_CLOSE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentStats {
    blocks: usize,
    words: usize,
    characters: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportResult {
    blocks: usize,
    path: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    markdown: String,
    blocks: usize,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ExportSettings {
    template_path: Option<PathBuf>,
    style_profile_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppPaths {
    config_dir: PathBuf,
    shared_profile_path: PathBuf,
    shared_template_path: Option<PathBuf>,
    shared_template_path_file: PathBuf,
    default_template_path: Option<PathBuf>,
    default_template_available: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchOptions {
    map_styles: bool,
    guide: bool,
    document_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSettingsDto {
    ui_language: Option<String>,
    ribbon_placement: String,
    /// `None` until the user has saved a split once (keeps localStorage as interim source).
    split_percent: Option<u8>,
}

#[tauri::command]
async fn analyze_markdown(markdown: String) -> Result<DocumentStats, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let document =
            mdoc_core::markdown::parse(&markdown).map_err(|error| error.to_string())?;
        Ok(DocumentStats {
            blocks: document.blocks.len(),
            words: markdown.split_whitespace().count(),
            characters: markdown.chars().count(),
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn read_markdown(path: PathBuf) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        fs::read_to_string(&path)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn write_markdown(path: PathBuf, markdown: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        fs::write(&path, markdown).map_err(|error| {
            format!("Could not write {}: {error}", path.display())
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn inspect_template(
    path: PathBuf,
) -> Result<Vec<mdoc_core::template::TemplateStyle>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        mdoc_core::template::inspect(path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn inspect_style_profile(path: PathBuf) -> Result<mdoc_core::StyleProfile, String> {
    tauri::async_runtime::spawn_blocking(move || load_style_profile(&path))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn get_app_paths() -> AppPaths {
    let default_template_path = mdoc_core::normal_dotm_path();
    AppPaths {
        config_dir: mdoc_core::config_dir(),
        shared_profile_path: mdoc_core::shared_style_profile_path(),
        shared_template_path: mdoc_core::load_shared_template_path(),
        shared_template_path_file: mdoc_core::shared_template_path_file(),
        default_template_available: default_template_path.is_some(),
        default_template_path,
    }
}

#[tauri::command]
async fn save_shared_template_path(path: Option<PathBuf>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        mdoc_core::save_shared_template_path(path.as_deref()).map_err(|error| {
            format!("Shared template path could not be saved: {error}")
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Aktualisiert den Styles-Cache nur bei geändertem mtime/Größe (oder fehlendem Cache).
#[tauri::command]
async fn ensure_shared_template_styles_cache() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        mdoc_core::ensure_shared_template_styles_cache().map_err(|error| {
            format!("Template styles cache could not be updated: {error}")
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn get_launch_options() -> LaunchOptions {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let map_styles = args
        .iter()
        .any(|arg| arg == "--map-styles" || arg == "/map-styles");
    let guide = args.iter().any(|arg| arg == "--guide" || arg == "/guide");
    let document_path = args
        .iter()
        .map(PathBuf::from)
        .find(|path| is_launch_document(path));
    LaunchOptions {
        map_styles,
        guide,
        document_path,
    }
}

/// Zweite App-Instanz mit der eingebetteten Kurzanleitung.
#[tauri::command]
fn open_guide_window() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("Unknown executable path: {error}"))?;
    Command::new(exe)
        .arg("--guide")
        .spawn()
        .map_err(|error| format!("Could not open help window: {error}"))?;
    Ok(())
}

/// Zentrale Settings aus `%APPDATA%\MDoc\settings.json` (inkl. Legacy-Migration).
#[tauri::command]
fn get_app_settings() -> AppSettingsDto {
    let settings = mdoc_core::load_app_settings();
    AppSettingsDto {
        ui_language: settings.resolved_ui_language().map(str::to_string),
        ribbon_placement: settings.resolved_ribbon_placement().to_string(),
        split_percent: settings
            .split_percent
            .map(|value| value.clamp(20, 80)),
    }
}

/// Speichert die UI-Sprache gemeinsam für Editor und Word-Add-in.
#[tauri::command]
fn set_ui_language(lang: String) -> Result<(), String> {
    mdoc_core::save_ui_language(&lang).map_err(|error| {
        format!("UI language could not be saved: {error}")
    })
}

/// Speichert die Ribbon-Platzierung; Word muss neu gestartet werden.
#[tauri::command]
fn set_ribbon_placement(placement: String) -> Result<(), String> {
    mdoc_core::save_ribbon_placement(&placement).map_err(|error| {
        format!("Ribbon placement could not be saved: {error}")
    })
}

/// Speichert den Split-Anteil Editor/Vorschau.
#[tauri::command]
fn set_split_percent(percent: u8) -> Result<(), String> {
    mdoc_core::save_split_percent(percent).map_err(|error| {
        format!("Split percent could not be saved: {error}")
    })
}

/// Erlaubt genau einen nachfolgenden Schließen-Versuch (nach Speichern/Verwerfen).
#[tauri::command]
fn accept_window_close() {
    ALLOW_CLOSE.store(true, Ordering::SeqCst);
}

#[tauri::command]
async fn load_shared_style_profile(
    template_path: Option<PathBuf>,
) -> Result<mdoc_core::StyleProfile, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let shared = mdoc_core::shared_style_profile_path();
        if shared.is_file() {
            return load_style_profile(&shared);
        }
        let template = mdoc_core::resolve_template(template_path);
        if let Some(template) = template {
            return mdoc_core::profile_from_template(&template)
                .map_err(|error| error.to_string());
        }
        Ok(mdoc_core::StyleProfile::default())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn save_shared_style_profile(
    profile: mdoc_core::StyleProfile,
) -> Result<PathBuf, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = mdoc_core::shared_style_profile_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("Could not create config directory: {error}")
            })?;
        }
        let json = serde_json::to_string_pretty(&profile)
            .map_err(|error| format!("Could not serialize style profile: {error}"))?;
        fs::write(&path, json).map_err(|error| {
            format!("Could not write {}: {error}", path.display())
        })?;
        Ok(path)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn resolve_template_path(path: Option<PathBuf>) -> Option<PathBuf> {
    tauri::async_runtime::spawn_blocking(move || mdoc_core::resolve_template(path))
        .await
        .ok()
        .flatten()
}

#[tauri::command]
async fn profile_from_template(path: PathBuf) -> Result<mdoc_core::StyleProfile, String> {
    tauri::async_runtime::spawn_blocking(move || {
        mdoc_core::profile_from_template(path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn import_docx(
    path: PathBuf,
    style_profile_path: Option<PathBuf>,
) -> Result<ImportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = resolve_style_profile(style_profile_path, None)?;
        let document =
            mdoc_core::docx_to_document(path, &profile).map_err(|error| error.to_string())?;
        let blocks = document.blocks.len();
        Ok(ImportResult {
            markdown: mdoc_core::document_to_markdown(&document),
            blocks,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn export_docx(
    markdown: String,
    path: PathBuf,
    settings: ExportSettings,
) -> Result<ExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let template = mdoc_core::resolve_template(settings.template_path.clone());
        let style_profile =
            resolve_style_profile(settings.style_profile_path, template.clone())?;
        let options = mdoc_core::ExportOptions {
            style_profile,
            template,
        };
        let document = mdoc_core::markdown_to_docx_with_options(&markdown, &path, &options)
            .map_err(|error| error.to_string())?;
        Ok(ExportResult {
            blocks: document.blocks.len(),
            path,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

fn resolve_style_profile(
    explicit: Option<PathBuf>,
    template: Option<PathBuf>,
) -> Result<mdoc_core::StyleProfile, String> {
    if let Some(profile_path) = explicit.filter(|path| path.is_file()) {
        return load_style_profile(&profile_path);
    }
    let shared = mdoc_core::shared_style_profile_path();
    if shared.is_file() {
        return load_style_profile(&shared);
    }
    if let Some(template) = template {
        return mdoc_core::profile_from_template(&template).map_err(|error| error.to_string());
    }
    Ok(mdoc_core::StyleProfile::default())
}

fn load_style_profile(path: &PathBuf) -> Result<mdoc_core::StyleProfile, String> {
    let json = fs::read_to_string(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let profile: mdoc_core::StyleProfile = serde_json::from_str(&json)
        .map_err(|error| format!("{} is not a valid style profile: {error}", path.display()))?;
    Ok(mdoc_core::migrate_legacy_english_ids(profile))
}

fn is_launch_document(path: &Path) -> bool {
    let raw = path.as_os_str();
    if raw.is_empty() || path.to_string_lossy().starts_with('-') {
        return false;
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    matches!(
        extension.as_deref(),
        Some("md" | "markdown" | "mdown" | "docx")
    ) && path.is_file()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Frontend hat freigegeben → dieses eine Close durchlassen.
                if ALLOW_CLOSE.swap(false, Ordering::SeqCst) {
                    return;
                }
                // Sonst blockieren und das Frontend fragen (Dirty-Check / Dialog).
                api.prevent_close();
                let _ = window.emit("mdoc-close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            analyze_markdown,
            read_markdown,
            write_markdown,
            inspect_template,
            inspect_style_profile,
            import_docx,
            export_docx,
            get_app_paths,
            get_launch_options,
            open_guide_window,
            get_app_settings,
            set_ui_language,
            set_ribbon_placement,
            set_split_percent,
            accept_window_close,
            load_shared_style_profile,
            save_shared_style_profile,
            save_shared_template_path,
            ensure_shared_template_styles_cache,
            resolve_template_path,
            profile_from_template
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
