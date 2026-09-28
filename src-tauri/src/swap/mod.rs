//! Item swaps (owned → wanted): the wanted item's package, renamed to the
//! owned item's package name, replaces the owned package in the install.
//! In game the player equips the owned item and sees the wanted one.
//!
//! All writes go through `game::writer` (backups, hash-checked restore).

pub mod commands;
pub mod engine;
pub mod paint;
pub mod rules;
pub mod state;

use tauri::AppHandle;

use crate::base::error::AppResult;
use crate::game::ReapplyOutcome;

pub use state::{ActiveSwap, SwapEvent, SwapRequest};

pub fn init(_app: &AppHandle) -> AppResult<()> {
    // State is loaded lazily; integrity handles post-update rebuilds.
    Ok(())
}

/// Called by `integrity` after a game update / verify pass.
pub fn reapply_after_update(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    engine::reapply_after_update(app)
}

/// Last owned item used per slot (for random presets).
pub fn owned_defaults() -> AppResult<std::collections::BTreeMap<crate::catalog::Slot, u32>> {
    Ok(state::load()?.owned_defaults)
}

/// Currently active swap requests (for preset capture).
pub fn active_requests() -> AppResult<Vec<SwapRequest>> {
    Ok(state::load()?
        .active
        .into_iter()
        .map(|a| a.request)
        .collect())
}
