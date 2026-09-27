//! IPC commands of the custom ball feature.

use tauri::AppHandle;

use crate::ball::engine::{self, BallStatus};
use crate::ball::library::BallPack;
use crate::base::{AppError, AppResult};
use crate::decals::import::ImportReport;

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Every ball pack of the app's library (previews generated/cached).
#[tauri::command]
pub async fn ball_library() -> AppResult<Vec<BallPack>> {
    blocking(|| Ok(engine::library())).await
}

#[tauri::command]
pub async fn ball_status(app: AppHandle) -> AppResult<BallStatus> {
    blocking(move || engine::status(&app)).await
}

#[tauri::command]
pub async fn ball_apply(app: AppHandle, pack_id: String) -> AppResult<BallStatus> {
    blocking(move || engine::apply(&app, &pack_id)).await
}

#[tauri::command]
pub async fn ball_remove(app: AppHandle) -> AppResult<BallStatus> {
    blocking(move || engine::remove(&app)).await
}

/// Copies AlphaConsole's `BallTextures` packs into the app's library.
#[tauri::command]
pub async fn ball_import_alphaconsole() -> AppResult<ImportReport> {
    blocking(engine::import_alphaconsole).await
}
