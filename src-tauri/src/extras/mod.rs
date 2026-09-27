//! Small read-mostly features: launching the game, local replays and
//! BakkesMod detection.

pub mod bakkes;
pub mod commands;
pub mod launch;
pub mod replays;

use tauri::AppHandle;

use crate::base::error::AppResult;

pub fn init(_app: &AppHandle) -> AppResult<()> {
    Ok(())
}
