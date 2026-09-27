//! Custom ball texture — injection-free: an image over the standard ball
//! (AlphaConsole `BallTextures` packs), loaded through `mods/`. Only this
//! player sees it.

pub mod commands;
pub mod engine;
pub mod library;

use tauri::AppHandle;

use crate::base::AppResult;
use crate::game::ReapplyOutcome;

pub fn init(_app: &AppHandle) -> AppResult<()> {
    Ok(())
}

/// Called by `integrity` after a game update or a "verify files" pass.
pub fn reapply_after_update(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    engine::reapply_after_update(app)
}
