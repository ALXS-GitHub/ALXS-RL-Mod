//! Which preset was applied last, and whether the game still matches it.
//!
//! Stored next to the presets folder (not inside: every `.json` there is a
//! preset). Forgotten automatically once nothing is modified anymore.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::base::error::AppResult;
use crate::base::{fsx, paths};
use crate::presets::bridge;
use crate::presets::model::Preset;
use crate::presets::store;
use crate::swap::{self, SwapRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    preset_id: String,
    name: String,
    applied_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePreset {
    pub preset_id: String,
    pub name: String,
    pub applied_at: DateTime<Utc>,
    /// Something changed since the preset was applied (or it was deleted).
    pub modified: bool,
}

/// What a loadout touches, reduced to comparable keys.
#[derive(Debug, Default, PartialEq)]
struct Loadout {
    swaps: Vec<(crate::catalog::Slot, u32, u32, u8)>,
    palette: Option<String>,
    decal: Option<String>,
    map: Option<String>,
}

impl Loadout {
    fn new(
        swaps: &[SwapRequest],
        palette: Option<String>,
        decal: Option<String>,
        map: Option<String>,
    ) -> Self {
        let mut swaps: Vec<_> = swaps
            .iter()
            .map(|r| (r.slot, r.owned_id, r.wanted_id, r.paint.unwrap_or(0)))
            .collect();
        swaps.sort_unstable();
        Self {
            swaps,
            palette,
            decal,
            map,
        }
    }

    fn of_preset(p: &Preset) -> Self {
        Self::new(
            &p.swaps,
            p.palette.as_ref().map(|x| x.id.clone()),
            p.decal.as_ref().map(|x| x.pack_id.clone()),
            p.map_id.clone(),
        )
    }

    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

fn file() -> AppResult<PathBuf> {
    Ok(paths::data_dir()?.join("active-preset.json"))
}

pub fn record(preset: &Preset) -> AppResult<()> {
    fsx::write_json(
        &file()?,
        &Record {
            preset_id: preset.id.clone(),
            name: preset.name.clone(),
            applied_at: Utc::now(),
        },
    )
}

fn forget() -> AppResult<()> {
    let path = file()?;
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn current_loadout(app: &AppHandle) -> AppResult<Loadout> {
    Ok(Loadout::new(
        &swap::active_requests()?,
        bridge::current_palette(app).ok().flatten().map(|p| p.id),
        bridge::current_decal(app).ok().flatten().map(|d| d.pack_id),
        bridge::current_map(app).ok().flatten(),
    ))
}

pub fn current(app: &AppHandle) -> AppResult<Option<ActivePreset>> {
    let path = file()?;
    if !path.is_file() {
        return Ok(None);
    }
    let Ok(rec) = serde_json::from_slice::<Record>(&std::fs::read(&path)?) else {
        forget()?;
        return Ok(None);
    };
    let now = current_loadout(app)?;
    if now.is_empty() {
        forget()?;
        return Ok(None);
    }
    let modified = store::get(&rec.preset_id).map_or(true, |p| Loadout::of_preset(&p) != now);
    Ok(Some(ActivePreset {
        preset_id: rec.preset_id,
        name: rec.name,
        applied_at: rec.applied_at,
        modified,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Slot;

    fn req(slot: Slot, owned: u32, wanted: u32, paint: Option<u8>) -> SwapRequest {
        SwapRequest {
            slot,
            owned_id: owned,
            wanted_id: wanted,
            paint,
            tint: None,
        }
    }

    #[test]
    fn same_loadout_ignores_order_and_unpainted_spelling() {
        let a = Loadout::new(
            &[
                req(Slot::Wheels, 1, 2, None),
                req(Slot::Boost, 3, 4, Some(5)),
            ],
            Some("p".into()),
            None,
            None,
        );
        let b = Loadout::new(
            &[
                req(Slot::Boost, 3, 4, Some(5)),
                req(Slot::Wheels, 1, 2, Some(0)),
            ],
            Some("p".into()),
            None,
            None,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn any_part_change_is_a_modification() {
        let base = Loadout::new(
            &[req(Slot::Wheels, 1, 2, None)],
            None,
            None,
            Some("m".into()),
        );
        assert_ne!(
            base,
            Loadout::new(
                &[req(Slot::Wheels, 1, 9, None)],
                None,
                None,
                Some("m".into())
            )
        );
        assert_ne!(
            base,
            Loadout::new(&[req(Slot::Wheels, 1, 2, None)], None, None, None)
        );
        assert!(Loadout::new(&[], None, None, None).is_empty());
        assert!(!base.is_empty());
    }
}
