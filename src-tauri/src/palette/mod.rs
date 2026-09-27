//! Custom colour palette: replaces the game's primary (blue / orange) and
//! accent picker colours through `CookedPCConsole/mods/TAGame.upk`.
//!
//! Update-robust by design: the picker arrays are located by content
//! signature in the current `TAGame.upk` (see `scan`), never by hard-coded
//! offsets or file hashes, and the override is rebuilt from the current
//! stock file after every game update.

pub mod commands;
pub mod engine;
pub mod scan;
pub mod store;

use tauri::AppHandle;

use crate::base::AppResult;
use crate::game::ReapplyOutcome;

pub fn init(_app: &AppHandle) -> AppResult<()> {
    store::import_legacy()?;
    Ok(())
}

/// The palette currently applied, if any (used by presets).
pub fn active_palette(_app: &AppHandle) -> AppResult<Option<store::Palette>> {
    engine::active_palette()
}

/// Called by `integrity` after a game update or a "verify files" pass.
pub fn reapply_after_update(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    engine::reapply_after_update(app)
}
