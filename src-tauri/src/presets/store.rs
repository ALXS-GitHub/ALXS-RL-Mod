//! One JSON file per preset in `%LOCALAPPDATA%\ALXS-RL-Mod\presets\`.

use std::path::PathBuf;

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};
use crate::presets::model::Preset;

pub const NAME_MAX: usize = 60;

fn dir() -> AppResult<PathBuf> {
    paths::data_subdir("presets")
}

fn file_for(id: &str) -> AppResult<PathBuf> {
    // Ids are UUIDs we generate; refuse anything that could escape the folder.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(AppError::InvalidInput(format!("invalid preset id {id:?}")));
    }
    Ok(dir()?.join(format!("{id}.json")))
}

pub fn clean_name(raw: &str) -> AppResult<String> {
    let name: String = raw.trim().chars().take(NAME_MAX).collect();
    if name.is_empty() {
        return Err(AppError::InvalidInput("preset name is empty".into()));
    }
    Ok(name)
}

pub fn list() -> AppResult<Vec<Preset>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir()?)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match std::fs::read(&path)
            .map_err(AppError::from)
            .and_then(|raw| Ok(serde_json::from_slice::<Preset>(&raw)?))
        {
            Ok(p) => out.push(p),
            Err(err) => tracing::warn!(path = %path.display(), %err, "skipping unreadable preset"),
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.updated_at));
    Ok(out)
}

pub fn get(id: &str) -> AppResult<Preset> {
    let path = file_for(id)?;
    let raw = std::fs::read(&path).map_err(|_| AppError::NotFound(format!("preset {id}")))?;
    Ok(serde_json::from_slice(&raw)?)
}

/// Creates (empty id) or updates a preset; returns the stored version.
pub fn save(mut preset: Preset) -> AppResult<Preset> {
    preset.name = clean_name(&preset.name)?;
    let now = chrono::Utc::now();
    if preset.id.is_empty() {
        preset.id = uuid::Uuid::new_v4().to_string();
        preset.created_at = now;
    }
    preset.updated_at = now;
    fsx::write_json(&file_for(&preset.id)?, &preset)?;
    Ok(preset)
}

pub fn delete(id: &str) -> AppResult<()> {
    let path = file_for(id)?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(AppError::NotFound(format!("preset {id}")))
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_capped() {
        assert_eq!(clean_name("  Octane  ").unwrap(), "Octane");
        assert_eq!(clean_name(&"x".repeat(200)).unwrap().len(), NAME_MAX);
        assert!(clean_name("   ").is_err());
    }

    #[test]
    fn ids_cannot_escape_the_folder() {
        assert!(file_for("../config").is_err());
        assert!(file_for("a/b").is_err());
        assert!(file_for("").is_err());
    }
}
