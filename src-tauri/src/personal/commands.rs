//! IPC commands of the personal edition (registered only with
//! `--features personal`, see `lib.rs`).

use tauri::AppHandle;

use crate::base::error::AppResult;
use crate::personal::mmr::{self, MmrProfile};
use crate::personal::ratings::RatingsState;
use crate::personal::service;

/// tracker.gg profile of any player (the "MMR" tab).
#[tauri::command]
pub async fn mmr_lookup(app: AppHandle, platform: String, player: String) -> AppResult<MmrProfile> {
    mmr::lookup(&app, &platform, &player).await
}

/// Your MMR lines and the lobby ratings (kept live by `personal://ratings`).
#[tauri::command]
pub fn ratings_state(app: AppHandle) -> AppResult<RatingsState> {
    Ok(service::snapshot(&app))
}
