//! Saved multiplayer host setups ("presets"): map, arena replaced and the
//! Rocket Plugin match options, reloaded in one click on the host tab.
//!
//! Stored as one JSON list in `<data>/multi/host_presets.json`. A preset
//! keeps the map's name too, so it still reads well if the map was deleted
//! from the library (the UI then flags it).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};
use crate::maps::library;
use crate::maps::model::normalize_target;
use crate::multi::rp_options::MatchOptions;

const MAX_PRESETS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostPreset {
    pub id: String,
    pub name: String,
    pub map_id: String,
    pub map_name: String,
    pub target: String,
    /// 0 = original, N = copy repaired with mapping vN, `None` = the map's
    /// default at hosting time.
    #[serde(default)]
    pub variant: Option<u32>,
    #[serde(default)]
    pub options: MatchOptions,
    pub saved_at: DateTime<Utc>,
}

/// What the UI sends: `id` set = overwrite that preset.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostPresetInput {
    pub id: Option<String>,
    pub name: String,
    pub map_id: String,
    pub target: String,
    #[serde(default)]
    pub variant: Option<u32>,
    #[serde(default)]
    pub options: MatchOptions,
}

fn file() -> AppResult<std::path::PathBuf> {
    Ok(paths::data_subdir("multi")?.join("host_presets.json"))
}

pub fn list() -> AppResult<Vec<HostPreset>> {
    let mut presets: Vec<HostPreset> = fsx::read_json_or_default(&file()?)?;
    presets.sort_by_key(|p| p.name.to_lowercase());
    Ok(presets)
}

/// Clean name: trimmed, control characters dropped, 1–60 characters.
pub(crate) fn clean_name(raw: &str) -> AppResult<String> {
    let name: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(60)
        .collect();
    if name.is_empty() {
        return Err(AppError::InvalidInput("empty preset name".into()));
    }
    Ok(name)
}

/// Inserts or replaces `preset` (by id) in `presets`.
pub(crate) fn upsert(presets: &mut Vec<HostPreset>, preset: HostPreset) -> AppResult<()> {
    match presets.iter_mut().find(|p| p.id == preset.id) {
        Some(existing) => *existing = preset,
        None => {
            if presets.len() >= MAX_PRESETS {
                return Err(AppError::InvalidInput("too many presets".into()));
            }
            presets.push(preset);
        }
    }
    Ok(())
}

pub fn save(input: HostPresetInput) -> AppResult<HostPreset> {
    let name = clean_name(&input.name)?;
    let target = normalize_target(&input.target)
        .ok_or_else(|| AppError::InvalidInput(format!("unknown Labs arena {}", input.target)))?
        .to_string();
    let map = library::get(&input.map_id)?;
    let variant = input
        .variant
        .map(|v| map.resolve_variant(Some(v)).map_err(AppError::InvalidInput))
        .transpose()?;
    let mut presets: Vec<HostPreset> = fsx::read_json_or_default(&file()?)?;
    let preset = HostPreset {
        id: input
            .id
            .filter(|id| presets.iter().any(|p| &p.id == id))
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name,
        map_id: map.id,
        variant,
        map_name: map.name,
        target,
        options: input.options,
        saved_at: Utc::now(),
    };
    upsert(&mut presets, preset.clone())?;
    fsx::write_json(&file()?, &presets)?;
    Ok(preset)
}

pub fn delete(id: &str) -> AppResult<()> {
    let mut presets: Vec<HostPreset> = fsx::read_json_or_default(&file()?)?;
    let before = presets.len();
    presets.retain(|p| p.id != id);
    if presets.len() == before {
        return Err(AppError::NotFound(format!("preset {id}")));
    }
    fsx::write_json(&file()?, &presets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(id: &str, name: &str) -> HostPreset {
        HostPreset {
            id: id.into(),
            name: name.into(),
            map_id: "m".into(),
            map_name: "Map".into(),
            target: "Labs_Underpass_P.upk".into(),
            variant: None,
            options: MatchOptions::default(),
            saved_at: Utc::now(),
        }
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_name("  Ping pong 3v3 \n").unwrap(), "Ping pong 3v3");
        assert!(clean_name("   ").is_err());
        assert_eq!(clean_name(&"x".repeat(100)).unwrap().len(), 60);
    }

    #[test]
    fn upsert_replaces_by_id() {
        let mut list = vec![preset("a", "A")];
        upsert(&mut list, preset("b", "B")).unwrap();
        upsert(&mut list, preset("a", "A2")).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "A2");
    }

    #[test]
    fn old_presets_without_options_load() {
        let raw = r#"[{"id":"a","name":"A","mapId":"m","mapName":"Map","target":"Labs_Underpass_P.upk","savedAt":"2026-11-27T10:00:00Z"}]"#;
        let list: Vec<HostPreset> = serde_json::from_str(raw).unwrap();
        assert_eq!(list[0].options, MatchOptions::default());
    }
}
