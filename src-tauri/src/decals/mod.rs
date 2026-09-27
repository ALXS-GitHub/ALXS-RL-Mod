//! Custom decals from AlphaConsole-style packs — injection-free.
//!
//! Two modes, both loaded through `mods/` from a renamed donor decal:
//! - **paintable** (AlphaConsole packs): the colour-zone mask replaces the
//!   donor's `_RGB`; the game paints the zones with the player's colours.
//! - **hybrid** (packs with `1_Diffuse_Skin` + `2_Diffuse_Skin_Mask`): art
//!   and zone mask replace those of a donor built on the stock
//!   `Body_Paintable_Diffuse_Mat`; masked zones take the player's colours,
//!   the rest shows the art in its own colours. Validated in game
//!   (2026-09-27, Octane on the Stars slot).

pub mod commands;
pub mod convert;
pub mod engine;
pub mod import;
pub mod library;
pub mod pipeline;
pub mod targets;

pub use engine::{DecalApplyRequest, DecalStatus};

use tauri::AppHandle;

use crate::base::AppResult;
use crate::game::ReapplyOutcome;

pub fn init(app: &AppHandle) -> AppResult<()> {
    if let Err(err) = engine::migrate_active(app) {
        tracing::warn!(%err, "active decal id not migrated");
    }
    Ok(())
}

/// Request that would re-create the active decal, if any (used by presets).
pub fn active_request(_app: &AppHandle) -> AppResult<Option<DecalApplyRequest>> {
    engine::active_request()
}

/// Called by `integrity` after a game update or a "verify files" pass.
pub fn reapply_after_update(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    engine::reapply_after_update(app)
}
