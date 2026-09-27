//! User palettes library: one JSON file per palette under
//! `%LOCALAPPDATA%\ALXS-RL-Mod\palettes\library\`.
//!
//! Palettes created by the previous app (`palettes/pal_*/palette.json`,
//! snake_case) are imported once at startup.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::base::{fsx, paths, AppError, AppResult};
use crate::palette::scan::{ACCENT_COUNT, PRIMARY_COUNT};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub primary_blue: Vec<Rgb>,
    #[serde(default)]
    pub primary_orange: Vec<Rgb>,
    #[serde(default)]
    pub accent: Vec<Rgb>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

const LIBRARY: &str = "palettes/library";
const MIGRATION_MARKER: &str = ".legacy-imported";
const MAX_NAME: usize = 60;

fn library_dir() -> AppResult<PathBuf> {
    paths::data_subdir(LIBRARY)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn file_of(id: &str) -> AppResult<PathBuf> {
    if !valid_id(id) {
        return Err(AppError::InvalidInput(format!("palette id {id:?}")));
    }
    Ok(library_dir()?.join(format!("{id}.json")))
}

pub fn list() -> AppResult<Vec<Palette>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(library_dir()?)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match std::fs::read(&path).map(|raw| serde_json::from_slice::<Palette>(&raw)) {
            Ok(Ok(p)) => out.push(p),
            _ => tracing::warn!(path = %path.display(), "unreadable palette skipped"),
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.updated_at));
    Ok(out)
}

pub fn get(id: &str) -> AppResult<Palette> {
    let path = file_of(id)?;
    let raw = std::fs::read(&path).map_err(|_| AppError::NotFound(format!("palette {id}")))?;
    Ok(serde_json::from_slice(&raw)?)
}

/// Validates, stamps and persists. An empty id creates a new palette.
pub fn save(mut palette: Palette) -> AppResult<Palette> {
    palette.name = palette.name.trim().chars().take(MAX_NAME).collect();
    if palette.name.is_empty() {
        return Err(AppError::InvalidInput("palette name is required".into()));
    }
    if palette.primary_blue.len() > PRIMARY_COUNT
        || palette.primary_orange.len() > PRIMARY_COUNT
        || palette.accent.len() > ACCENT_COUNT
    {
        return Err(AppError::InvalidInput(format!(
            "at most {PRIMARY_COUNT} primary and {ACCENT_COUNT} accent colours"
        )));
    }
    let now = Utc::now();
    if palette.id.is_empty() {
        palette.id = format!("pal_{}", uuid::Uuid::new_v4().simple());
        palette.created_at = now;
    }
    palette.updated_at = now;
    fsx::write_json(&file_of(&palette.id)?, &palette)?;
    Ok(palette)
}

pub fn delete(id: &str) -> AppResult<()> {
    let path = file_of(id)?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

// ── Legacy import ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct LegacyPalette {
    id: String,
    name: String,
    #[serde(default)]
    primary_blue: Vec<Rgb>,
    #[serde(default)]
    primary_orange: Vec<Rgb>,
    #[serde(default)]
    accent: Vec<Rgb>,
    #[serde(default)]
    use_same_primary: bool,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

fn parse_date(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn convert(legacy: LegacyPalette) -> Palette {
    let orange = if legacy.use_same_primary || legacy.primary_orange.is_empty() {
        legacy.primary_blue.clone()
    } else {
        legacy.primary_orange
    };
    Palette {
        id: legacy
            .id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect(),
        name: legacy.name,
        primary_blue: legacy
            .primary_blue
            .into_iter()
            .take(PRIMARY_COUNT)
            .collect(),
        primary_orange: orange.into_iter().take(PRIMARY_COUNT).collect(),
        accent: legacy.accent.into_iter().take(ACCENT_COUNT).collect(),
        created_at: parse_date(&legacy.created_at),
        updated_at: parse_date(&legacy.updated_at),
    }
}

/// Imports `palettes/pal_*/palette.json` from the previous app, once.
/// Returns how many palettes were imported.
pub fn import_legacy() -> AppResult<usize> {
    let root = paths::data_subdir("palettes")?;
    let marker = root.join(MIGRATION_MARKER);
    if marker.exists() {
        return Ok(0);
    }
    let mut imported = 0;
    for entry in std::fs::read_dir(&root)?.flatten() {
        let meta = entry.path().join("palette.json");
        if !meta.is_file() {
            continue;
        }
        let Ok(raw) = std::fs::read(&meta) else {
            continue;
        };
        let Ok(legacy) = serde_json::from_slice::<LegacyPalette>(&raw) else {
            tracing::warn!(path = %meta.display(), "legacy palette unreadable");
            continue;
        };
        let palette = convert(legacy);
        if !valid_id(&palette.id) || file_of(&palette.id)?.exists() {
            continue;
        }
        fsx::write_json(&file_of(&palette.id)?, &palette)?;
        imported += 1;
    }
    fsx::write_atomic(&marker, chrono::Utc::now().to_rfc3339().as_bytes())?;
    if imported > 0 {
        tracing::info!(imported, "legacy palettes imported");
    }
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_format_converts() {
        let raw = r#"{"id":"pal_1","name":"ALXS Rainbow","primary_blue":[{"r":255,"g":0,"b":0}],
            "primary_orange":[],"accent":[{"r":1,"g":2,"b":3}],"use_same_primary":true,
            "created_at":"2026-05-14T10:00:00Z","updated_at":"2026-05-15T10:00:00Z"}"#;
        let p = convert(serde_json::from_str(raw).unwrap());
        assert_eq!(p.id, "pal_1");
        assert_eq!(p.primary_orange, p.primary_blue);
        assert_eq!(p.accent[0], Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(p.created_at.to_rfc3339(), "2026-05-14T10:00:00+00:00");
    }

    #[test]
    fn new_palette_json_is_camel_case() {
        let p = Palette {
            id: "pal_x".into(),
            name: "x".into(),
            primary_blue: vec![],
            primary_orange: vec![],
            accent: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("primaryBlue") && json.contains("updatedAt"));
        let back: Palette = serde_json::from_str(r#"{"name":"n"}"#).unwrap();
        assert!(back.id.is_empty());
    }

    #[test]
    fn ids_are_restricted() {
        assert!(valid_id("pal_abc-1"));
        assert!(!valid_id("../x"));
        assert!(!valid_id(""));
    }
}
