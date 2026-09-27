//! IPC commands of the maps module (names fixed by docs/ARCHITECTURE.md).

use std::path::PathBuf;

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::extras::launch;
use crate::game::install;
use crate::maps::model::{BrowseResult, MapEntry, MapPatch, MapSession, RemoteSource};
use crate::maps::{library, session, sources};

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn maps_list() -> AppResult<Vec<MapEntry>> {
    blocking(library::list).await
}

/// Files (`.upk`, `.udk`, `.zip`) or folders (walked, one map per folder).
#[tauri::command]
pub async fn maps_import(paths: Vec<String>) -> AppResult<Vec<MapEntry>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    blocking(move || library::import_paths(&paths)).await
}

#[tauri::command]
pub async fn maps_delete(id: String) -> AppResult<()> {
    if session::current().is_some_and(|s| s.map_id == id) {
        return Err(AppError::Conflict(
            "this map is currently installed — restore the stock arena first".into(),
        ));
    }
    blocking(move || library::delete(&id)).await
}

#[tauri::command]
pub async fn maps_update(id: String, patch: MapPatch) -> AppResult<MapEntry> {
    blocking(move || library::update(&id, patch)).await
}

#[tauri::command]
pub async fn maps_browse(
    source: RemoteSource,
    query: Option<String>,
    page: Option<u32>,
) -> AppResult<BrowseResult> {
    sources::browse(source, query.as_deref().unwrap_or(""), page.unwrap_or(1)).await
}

#[tauri::command]
pub async fn maps_download(
    app: AppHandle,
    source: RemoteSource,
    remote_id: String,
) -> AppResult<MapEntry> {
    sources::download(&app, source, &remote_id).await
}

#[tauri::command]
pub async fn maps_activate(
    app: AppHandle,
    id: String,
    target: Option<String>,
) -> AppResult<MapSession> {
    session::activate(&app, &id, target, false).await
}

#[tauri::command]
pub async fn maps_deactivate(app: AppHandle) -> AppResult<()> {
    session::deactivate(&app).await
}

#[tauri::command]
pub fn maps_session() -> AppResult<Option<MapSession>> {
    Ok(session::current())
}

/// Installs the map, starts the game without EAC and restores the stock
/// arena when the game exits (if enabled in settings).
#[tauri::command]
pub async fn maps_play_offline(app: AppHandle, id: String) -> AppResult<MapSession> {
    let rl = install::current_install(&app)?;
    let session = session::activate(&app, &id, None, true).await?;
    if let Err(err) = launch::spawn_no_eac(&rl) {
        // The map stays installed: the user can start the game manually.
        tracing::warn!(%err, "offline launch failed after activation");
        return Err(err);
    }
    session::watch_offline_session(app, session.map_id.clone());
    Ok(session)
}
