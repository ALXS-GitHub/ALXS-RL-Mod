//! IPC commands of the colour palette feature. Heavy work (reading and
//! patching the ~80 MB `TAGame.upk`) runs on the blocking pool.

use tauri::AppHandle;

use crate::base::{AppError, AppResult};
use crate::palette::engine::{self, PaletteStatus};
use crate::palette::store::{self, Palette};

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn palette_status(app: AppHandle) -> AppResult<PaletteStatus> {
    blocking(move || engine::status(&app)).await
}

/// The game's own picker colours (the base every palette starts from).
#[tauri::command]
pub async fn palette_stock(app: AppHandle) -> AppResult<Palette> {
    blocking(move || engine::stock(&app)).await
}

#[tauri::command]
pub fn palette_list() -> AppResult<Vec<Palette>> {
    store::list()
}

#[tauri::command]
pub fn palette_save(palette: Palette) -> AppResult<Palette> {
    store::save(palette)
}

/// Deleting the active palette first restores the stock picker.
#[tauri::command]
pub async fn palette_delete(app: AppHandle, id: String) -> AppResult<()> {
    blocking(move || {
        if engine::is_active(&id) {
            engine::restore(&app)?;
        }
        store::delete(&id)
    })
    .await
}

#[tauri::command]
pub async fn palette_apply(app: AppHandle, id: String) -> AppResult<PaletteStatus> {
    blocking(move || engine::apply(&app, &id)).await
}

#[tauri::command]
pub async fn palette_restore(app: AppHandle) -> AppResult<PaletteStatus> {
    blocking(move || engine::restore(&app)).await
}
