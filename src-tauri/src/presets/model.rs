//! Preset = a full loadout: item swaps + palette + custom decal + map.

use serde::{Deserialize, Serialize};

use crate::swap::SwapRequest;

/// Palette carried inline so a share code is self-contained.
pub type PresetPalette = crate::palette::store::Palette;
/// Custom-decal selection (owned by the engine slice).
pub type PresetDecal = crate::decals::DecalApplyRequest;

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    /// Empty for a preset that was never saved (random / import preview).
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default = "now")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "now")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub swaps: Vec<SwapRequest>,
    #[serde(default)]
    pub palette: Option<PresetPalette>,
    #[serde(default)]
    pub decal: Option<PresetDecal>,
    #[serde(default)]
    pub map_id: Option<String>,
}

impl Preset {
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            id: String::new(),
            name: name.into(),
            created_at: now(),
            updated_at: now(),
            swaps: Vec::new(),
            palette: None,
            decal: None,
            map_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PresetPart {
    Swaps,
    Palette,
    Decal,
    Map,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartResult {
    pub part: PresetPart,
    pub ok: bool,
    /// Items applied for `swaps`, 1/0 for the other parts.
    pub applied: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetApplyReport {
    pub preset_id: String,
    pub ok: bool,
    pub parts: Vec<PartResult>,
}
