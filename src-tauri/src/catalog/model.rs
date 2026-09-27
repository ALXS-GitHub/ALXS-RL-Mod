//! Catalog data model shared with the swap / presets modules and the UI.

use serde::{Deserialize, Serialize};

/// Cosmetic slots we can swap. Audio-only slots (engine audio, anthems)
/// are not exposed: their `.bnk` banks are not handled yet.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Slot {
    Body,
    Decal,
    Wheels,
    Boost,
    Topper,
    Antenna,
    GoalExplosion,
    Trail,
    PaintFinish,
}

impl Slot {
    pub const ALL: [Slot; 9] = [
        Slot::Body,
        Slot::Decal,
        Slot::Wheels,
        Slot::Boost,
        Slot::Topper,
        Slot::Antenna,
        Slot::GoalExplosion,
        Slot::Trail,
        Slot::PaintFinish,
    ];

    /// Stable small index used by compact share codes.
    pub fn index(self) -> u8 {
        Slot::ALL.iter().position(|s| *s == self).unwrap_or(0) as u8
    }

    pub fn from_index(i: u8) -> Option<Slot> {
        Slot::ALL.get(i as usize).copied()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    /// Product id from the game's product database.
    pub id: u32,
    pub slot: Slot,
    /// Internal asset name (`WHEEL_AlphaRim`, `Skin_Octane_Flames`…).
    pub asset: String,
    /// Package file in `CookedPCConsole` holding the asset (real casing).
    pub package: String,
    /// Thumbnail package (`<base>_T_SF.upk`) when present, for `thumbnail_get`.
    pub thumbnail_package: Option<String>,
    pub label_fr: String,
    /// English label from the game (humanised asset name if unknown).
    pub label_en: String,
    /// Decals only: the body this decal belongs to (`None` = universal).
    pub body_id: Option<u32>,
    /// Several products live in the same package (tier / colour variants):
    /// swapping the package affects all of them.
    pub shared_package: bool,
    /// The package is encrypted with a key missing from `keys.txt`: it can be
    /// swapped *into* (owned) but not used as a source (wanted).
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotCount {
    pub slot: Slot,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSnapshot {
    pub items: Vec<CatalogItem>,
    pub counts: Vec<SlotCount>,
    /// Products of exposed slots whose package is not on disk.
    pub unresolved: u32,
    /// Build fingerprint the snapshot was built against.
    pub build: String,
    /// Product list and labels were read from the installed game (always
    /// current); `false` = bundled list only (localisation not readable).
    pub from_game: bool,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl CatalogSnapshot {
    pub fn get(&self, id: u32) -> Option<&CatalogItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn by_slot(&self, slot: Slot) -> impl Iterator<Item = &CatalogItem> {
        self.items.iter().filter(move |i| i.slot == slot)
    }
}
