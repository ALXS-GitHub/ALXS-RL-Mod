//! Catalog IPC commands.

use std::sync::Arc;

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::catalog::{self, CatalogSnapshot};

fn unwrap_snapshot(snap: Arc<CatalogSnapshot>) -> CatalogSnapshot {
    Arc::try_unwrap(snap).unwrap_or_else(|shared| (*shared).clone())
}

#[tauri::command]
pub async fn catalog_get(app: AppHandle) -> AppResult<CatalogSnapshot> {
    tauri::async_runtime::spawn_blocking(move || catalog::snapshot(&app).map(unwrap_snapshot))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn catalog_refresh(app: AppHandle) -> AppResult<CatalogSnapshot> {
    tauri::async_runtime::spawn_blocking(move || catalog::rebuild(&app).map(unwrap_snapshot))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}
