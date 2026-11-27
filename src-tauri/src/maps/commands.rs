//! IPC commands of the maps module (names fixed by docs/ARCHITECTURE.md).

use std::path::PathBuf;

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::extras::launch;
use crate::game::install;
use crate::maps::model::{BrowseResult, MapEntry, MapPatch, MapSession, MapSort, RemoteSource};
use crate::maps::{library, repair, session, sources, textures};

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
    sort: Option<MapSort>,
) -> AppResult<BrowseResult> {
    sources::browse(
        source,
        query.as_deref().unwrap_or(""),
        page.unwrap_or(1),
        sort.unwrap_or_default(),
    )
    .await
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

/// Bundled repair mappings (oldest first; the last one is the default).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingInfo {
    pub version: u32,
    pub label: String,
    pub released: String,
    pub game_build: String,
    pub rules: usize,
    pub latest: bool,
}

#[tauri::command]
pub fn maps_repair_mappings() -> AppResult<Vec<MappingInfo>> {
    let latest = repair::latest().version;
    Ok(repair::mappings()
        .iter()
        .map(|m| MappingInfo {
            version: m.version,
            label: m.label.clone(),
            released: m.released.clone(),
            game_build: m.game_build.clone(),
            rules: m.rules.len(),
            latest: m.version == latest,
        })
        .collect())
}

/// What `version` (default: latest) would fix in every library map.
#[tauri::command]
pub async fn maps_repair_status(version: Option<u32>) -> AppResult<Vec<library::RepairStatus>> {
    let version = version.unwrap_or(repair::latest().version);
    blocking(move || {
        library::list()?
            .iter()
            .map(|e| library::repair_status(e, version))
            .collect()
    })
    .await
}

/// After a repair / undo: the installed arena must hold the new file.
fn reinstall_if_active(app: &AppHandle, id: &str) -> AppResult<()> {
    if let Some(s) = session::current().filter(|s| s.map_id == id) {
        session::activate_blocking(app, id, Some(s.target), s.offline).map_err(|err| {
            AppError::Conflict(format!(
                "the map was changed but could not be reinstalled now ({err}) — go back to the main menu and install it again"
            ))
        })?;
    }
    Ok(())
}

/// Repairs a map with mapping `version` (default: latest). The original
/// package stays; thumbnail, name and settings are the map's own.
#[tauri::command]
pub async fn maps_repair(app: AppHandle, id: String, version: Option<u32>) -> AppResult<MapEntry> {
    let version = version.unwrap_or(repair::latest().version);
    blocking(move || {
        let entry = library::repair_map(&id, version)?;
        reinstall_if_active(&app, &id)?;
        Ok(entry)
    })
    .await
}

/// Goes back to the map's original package.
#[tauri::command]
pub async fn maps_repair_undo(app: AppHandle, id: String) -> AppResult<MapEntry> {
    blocking(move || {
        let entry = library::undo_repair(&id)?;
        reinstall_if_active(&app, &id)?;
        Ok(entry)
    })
    .await
}

#[tauri::command]
pub async fn maps_textures_status(app: AppHandle) -> AppResult<textures::TexturesStatus> {
    blocking(move || textures::status(&app)).await
}

/// Downloads the workshop textures pack (explicit user action only).
#[tauri::command]
pub async fn maps_textures_install(app: AppHandle) -> AppResult<textures::TexturesStatus> {
    textures::install(&app).await
}

#[tauri::command]
pub async fn maps_textures_remove(app: AppHandle) -> AppResult<textures::TexturesStatus> {
    blocking(move || textures::remove(&app)).await
}

/// Installs the map, starts the game without EAC (Epic: brings the Epic
/// launcher forward, the user picks "Launch without Anti-Cheat") and
/// restores the stock arena when the game exits (if enabled in settings).
#[tauri::command]
pub async fn maps_play_offline(app: AppHandle, id: String) -> AppResult<MapSession> {
    let rl = install::current_install(&app)?;
    let session = session::activate(&app, &id, None, true).await?;
    if let Err(err) = launch::start_no_eac(&rl) {
        // The map stays installed: the user can start the game manually.
        tracing::warn!(%err, "offline launch failed after activation");
        return Err(err);
    }
    session::watch_offline_session(app, session.map_id.clone());
    Ok(session)
}
