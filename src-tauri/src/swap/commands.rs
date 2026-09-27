//! Swap IPC commands. Heavy work (reading / renaming packages) runs off the
//! async runtime.

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::swap::engine;
use crate::swap::state::{self, ActiveSwap, SwapEvent, SwapRequest};

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub fn swap_list() -> AppResult<Vec<ActiveSwap>> {
    Ok(state::load()?.active)
}

#[tauri::command]
pub async fn swap_apply(app: AppHandle, req: SwapRequest) -> AppResult<ActiveSwap> {
    blocking(move || engine::apply(&app, &req)).await
}

#[tauri::command]
pub async fn swap_restore(app: AppHandle, id: String) -> AppResult<()> {
    blocking(move || engine::restore(&app, &id)).await
}

#[tauri::command]
pub async fn swap_restore_all(app: AppHandle) -> AppResult<u32> {
    blocking(move || engine::restore_all(&app)).await
}

/// Newest first.
#[tauri::command]
pub fn swap_history() -> AppResult<Vec<SwapEvent>> {
    let mut history = state::load()?.history;
    history.reverse();
    Ok(history)
}
