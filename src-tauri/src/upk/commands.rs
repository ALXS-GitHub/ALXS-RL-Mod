//! IPC commands of the package engine.

use tauri::AppHandle;

use crate::base::{AppError, AppResult};
use crate::game::{fingerprint, install};
use crate::upk::{keys, thumbs};

#[tauri::command]
pub fn keys_status(app: AppHandle) -> AppResult<keys::KeysStatus> {
    // Re-read so a freshly dropped keys.txt is picked up without a restart.
    keys::reload(&app);
    Ok(keys::status(&app))
}

/// Imports a `keys.txt` chosen by the user (see [`keys::import_file`]).
#[tauri::command]
pub fn keys_import(app: AppHandle, path: String) -> AppResult<keys::KeysStatus> {
    keys::import_file(&app, std::path::Path::new(&path))?;
    Ok(keys::status(&app))
}

/// Absolute path of the cached thumbnail PNG for a catalog package, or
/// `None` when the item has no readable icon. The front-end turns the path
/// into a URL with `convertFileSrc`.
#[tauri::command]
pub async fn thumbnail_get(app: AppHandle, package: String) -> AppResult<Option<String>> {
    if package.trim().is_empty() || package.contains(['/', '\\']) || package.contains("..") {
        return Err(AppError::InvalidInput(format!(
            "bad package name {package:?}"
        )));
    }
    let install = install::current_install(&app)?;
    let build = fingerprint::current(&install)
        .map(|f| f.id())
        .unwrap_or_default();
    let ring = keys::ring(&app);
    tauri::async_runtime::spawn_blocking(move || {
        thumbs::thumbnail(&install.cooked_dir, &ring, &package, &build)
            .map(|p| p.map(|p| p.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}
