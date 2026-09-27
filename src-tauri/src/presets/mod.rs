//! Loadout presets: save, apply (per-part report), share codes, random.

pub mod active;
pub mod bridge;
pub mod commands;
pub mod model;
pub mod random;
pub mod share;
pub mod store;

use tauri::AppHandle;

use crate::base::error::AppResult;

pub use model::{Preset, PresetApplyReport};

pub fn init(_app: &AppHandle) -> AppResult<()> {
    // Make sure the folder exists so the first `list` is cheap.
    crate::base::paths::data_subdir("presets").map(|_| ())
}
