//! Every call from presets into other slices lives here, so the cross-slice
//! surface is visible (and adjustable) in one place.
//!
//! Assumed APIs (see docs/ARCHITECTURE.md):
//! - engine: `palette::store::{Palette, save}`, `palette::commands::{palette_apply, palette_restore}`,
//!   `palette::active_palette`, `decals::DecalApplyRequest`,
//!   `decals::commands::{decals_apply, decals_remove}`, `decals::active_request`;
//! - play: `maps::commands::maps_activate`, `maps::active_map_id`.

use tauri::AppHandle;

use crate::base::error::AppResult;
use crate::presets::model::{PresetDecal, PresetPalette};

pub async fn apply_palette(app: &AppHandle, palette: Option<&PresetPalette>) -> AppResult<()> {
    match palette {
        Some(p) => {
            // Store it first: the palette may come from a share code.
            crate::palette::store::save(p.clone()).map(|_| ())?;
            crate::palette::commands::palette_apply(app.clone(), p.id.clone())
                .await
                .map(|_| ())
        }
        None => crate::palette::commands::palette_restore(app.clone())
            .await
            .map(|_| ()),
    }
}

pub async fn apply_decal(app: &AppHandle, decal: Option<&PresetDecal>) -> AppResult<()> {
    match decal {
        Some(req) => crate::decals::commands::decals_apply(app.clone(), req.clone())
            .await
            .map(|_| ()),
        None => crate::decals::commands::decals_remove(app.clone())
            .await
            .map(|_| ()),
    }
}

pub async fn apply_map(app: &AppHandle, map_id: &str) -> AppResult<()> {
    crate::maps::commands::maps_activate(app.clone(), map_id.to_string(), None)
        .await
        .map(|_| ())
}

pub fn current_palette(app: &AppHandle) -> AppResult<Option<PresetPalette>> {
    crate::palette::active_palette(app)
}

pub fn current_decal(app: &AppHandle) -> AppResult<Option<PresetDecal>> {
    crate::decals::active_request(app)
}

pub fn current_map(app: &AppHandle) -> AppResult<Option<String>> {
    crate::maps::active_map_id(app)
}
