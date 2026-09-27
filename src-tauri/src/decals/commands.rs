//! IPC commands of the custom decals feature.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::base::config::{self, AppConfigPatch};
use crate::base::{AppError, AppResult};
use crate::decals::engine::{self, DecalApplyRequest, DecalStatus};
use crate::decals::library::DecalPack;

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Every pack found in the configured libraries (previews generated/cached).
#[tauri::command]
pub async fn decals_library(app: AppHandle) -> AppResult<Vec<DecalPack>> {
    blocking(move || Ok(engine::library(&app))).await
}

/// Replaces the library folders (empty = default AlphaConsole folder).
#[tauri::command]
pub async fn decals_library_folders_set(
    app: AppHandle,
    folders: Vec<String>,
) -> AppResult<Vec<DecalPack>> {
    let mut clean: Vec<PathBuf> = Vec::new();
    for f in folders.iter().map(|f| f.trim()).filter(|f| !f.is_empty()) {
        let p = PathBuf::from(f);
        if !clean.contains(&p) {
            clean.push(p);
        }
    }
    config::update(
        &app,
        AppConfigPatch {
            decal_library_folders: Some(clean),
            ..Default::default()
        },
    )?;
    blocking(move || Ok(engine::library(&app))).await
}

#[tauri::command]
pub async fn decals_status(app: AppHandle) -> AppResult<DecalStatus> {
    blocking(move || engine::status(&app)).await
}

#[tauri::command]
pub async fn decals_apply(app: AppHandle, req: DecalApplyRequest) -> AppResult<DecalStatus> {
    blocking(move || engine::apply(&app, &req)).await
}

/// Copies the AlphaConsole packs into the app's library (converted to hybrid
/// packs when `convert` is set). The AlphaConsole folder is not modified.
#[tauri::command]
pub async fn decals_import_alphaconsole(
    app: AppHandle,
    convert: bool,
) -> AppResult<crate::decals::import::ImportReport> {
    blocking(move || engine::import_alphaconsole(&app, convert)).await
}

#[tauri::command]
pub async fn decals_remove(app: AppHandle) -> AppResult<DecalStatus> {
    blocking(move || engine::remove(&app)).await
}
