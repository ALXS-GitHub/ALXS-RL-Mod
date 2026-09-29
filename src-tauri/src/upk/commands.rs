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

/// Catalog items a key can open (0 without a game or keys).
fn readable_items(app: &AppHandle) -> usize {
    crate::catalog::snapshot(app)
        .map(|s| s.items.iter().filter(|i| !i.locked).count())
        .unwrap_or(0)
}

/// Rebuilds the catalog after a key change (locks depend on the ring) and
/// reports what the new keys brought.
async fn after_keys_change(
    app: AppHandle,
    before: usize,
    (added, total): (usize, usize),
) -> AppResult<keys::KeysUpdate> {
    let unlocked = if added == 0 {
        0
    } else {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(err) = crate::catalog::rebuild(&app) {
                tracing::warn!(%err, "catalog rebuild after key change failed");
            }
            readable_items(&app).saturating_sub(before)
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
    };
    Ok(keys::KeysUpdate {
        added,
        total,
        unlocked,
        status: keys::status(&app),
    })
}

async fn readable_before(app: &AppHandle) -> AppResult<usize> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || readable_items(&app))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Imports a `keys.txt` chosen by the user (see [`keys::import_file`]).
#[tauri::command]
pub async fn keys_import(app: AppHandle, path: String) -> AppResult<keys::KeysUpdate> {
    let before = readable_before(&app).await?;
    let counts = keys::import_file(&app, std::path::Path::new(&path))?;
    after_keys_change(app, before, counts).await
}

/// Downloads the community key list (see [`keys::download_community`]) and
/// merges it into the ring.
#[tauri::command]
pub async fn keys_download(app: AppHandle) -> AppResult<keys::KeysUpdate> {
    let incoming = keys::download_community().await?;
    let before = readable_before(&app).await?;
    let counts = keys::add_downloaded(&app, &incoming, &current_build(&app))?;
    after_keys_change(app, before, counts).await
}

/// Hides the "update your keys" suggestion until the next game build.
#[tauri::command]
pub fn keys_dismiss_update(app: AppHandle) -> AppResult<keys::KeysStatus> {
    keys::mark_checked(&current_build(&app))?;
    Ok(keys::status(&app))
}

fn current_build(app: &AppHandle) -> String {
    install::current_install(app)
        .ok()
        .as_ref()
        .and_then(fingerprint::current)
        .map(|f| f.id())
        .unwrap_or_default()
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
