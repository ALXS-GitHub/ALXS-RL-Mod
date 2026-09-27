//! App-level commands: version info and a local diagnostics bundle.
//!
//! Diagnostics are **never uploaded**: the zip is written locally and the
//! user decides what to do with it.

use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;
use tauri::AppHandle;

use crate::base::error::AppResult;
use crate::base::{config, paths};
use crate::game;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub debug: bool,
}

#[tauri::command]
pub fn app_info() -> AppResult<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: paths::data_dir()?,
        logs_dir: paths::logs_dir()?,
        debug: cfg!(debug_assertions),
    })
}

/// Zips logs + state files (no game files, no keys) into
/// `%LOCALAPPDATA%\ALXS-RL-Mod\diagnostics\` and returns the zip path.
#[tauri::command]
pub async fn diagnostics_export(app: AppHandle) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || build_diagnostics(&app))
        .await
        .map_err(|e| crate::base::AppError::Internal(e.to_string()))?
}

fn build_diagnostics(app: &AppHandle) -> AppResult<String> {
    let out_dir = paths::data_subdir("diagnostics")?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let zip_path = out_dir.join(format!("alxs-rl-mod-diagnostics-{stamp}.zip"));
    let file = std::fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let data = paths::data_dir()?;
    // State files only — small JSON, no user media, no keys.
    for name in ["config.json", "manifest.json"] {
        let p = data.join(name);
        if let Ok(bytes) = std::fs::read(&p) {
            zip.start_file(name, opts)?;
            zip.write_all(&bytes)?;
        }
    }
    if let Ok(entries) = std::fs::read_dir(paths::logs_dir()?) {
        let mut logs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        logs.sort();
        for p in logs.iter().rev().take(5) {
            if let (Some(name), Ok(bytes)) = (p.file_name(), std::fs::read(p)) {
                zip.start_file(format!("logs/{}", name.to_string_lossy()), opts)?;
                zip.write_all(&bytes)?;
            }
        }
    }

    let cfg = config::current(app);
    let status = game::status_snapshot(app);
    let summary = serde_json::json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "locale": cfg.locale,
        "game": status,
    });
    zip.start_file("summary.json", opts)?;
    zip.write_all(serde_json::to_string_pretty(&summary)?.as_bytes())?;
    zip.finish()?;
    tracing::info!(path = %zip_path.display(), "diagnostics exported");
    Ok(zip_path.to_string_lossy().into_owned())
}
