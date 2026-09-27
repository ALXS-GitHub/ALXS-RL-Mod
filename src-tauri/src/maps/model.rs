//! Map library data model + migration of the previous app's `meta.json`.
//!
//! Layout: `<data>/maps/<map_id>/` holds the map package(s), an optional
//! `preview.(jpg|png)` and `map.json` (this model). Folders created by the
//! previous app only have a snake_case `meta.json`; it is read transparently
//! and a `map.json` is written next to it (the original is never touched).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Labs arenas a custom map can replace. The first one is the community
/// default (least likely to be picked by online playlists).
pub const LABS_TARGETS: &[&str] = &[
    "Labs_Underpass_P.upk",
    "Labs_Utopia_P.upk",
    "Labs_Cosmic_V4_P.upk",
    "Labs_Cosmic_P.upk",
    "Labs_Octagon_02_P.upk",
    "Labs_Octagon_P.upk",
    "Labs_Galleon_P.upk",
    "Labs_Holyfield_P.upk",
    "Labs_Basin_P.upk",
    "Labs_CirclePillars_P.upk",
    "Labs_Corridor_P.upk",
    "Labs_DoubleGoal_V2_P.upk",
    "Labs_DoubleGoal_P.upk",
    "Labs_PillarGlass_P.upk",
    "Labs_PillarHeat_P.upk",
    "Labs_PillarWings_P.upk",
];

pub const DEFAULT_TARGET: &str = "Labs_Underpass_P.upk";
pub const MAP_EXTENSIONS: &[&str] = &["upk", "udk"];
pub const ENTRY_FILE: &str = "map.json";
pub const LEGACY_META_FILE: &str = "meta.json";

/// Returns the canonical spelling of `target` if it is a known Labs arena.
pub fn normalize_target(target: &str) -> Option<&'static str> {
    let t = target.trim();
    let with_ext = if t.to_ascii_lowercase().ends_with(".upk") {
        t.to_string()
    } else {
        format!("{t}.upk")
    };
    LABS_TARGETS
        .iter()
        .copied()
        .find(|known| known.eq_ignore_ascii_case(&with_ext))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum RemoteSource {
    BakkesPlugins,
    Lethamyr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum MapOrigin {
    Imported,
    /// Imported from a BakkesMod-era workshop folder.
    BakkesMod {
        folder_name: String,
    },
    Remote {
        source: RemoteSource,
        remote_id: String,
        version: Option<String>,
    },
    Other {
        url: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapEntry {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub origin: MapOrigin,
    /// Main package file name inside the map folder (e.g. `map.upk`).
    pub file_name: String,
    /// Extra packages shipped with the map (textures…), installed next to it in `mods/`.
    #[serde(default)]
    pub companions: Vec<String>,
    /// Preview image file name inside the map folder.
    pub preview_file: Option<String>,
    /// Remote preview (bakkesplugins banner) when no local image exists.
    pub preview_url: Option<String>,
    pub size_bytes: u64,
    pub added_at: DateTime<Utc>,
    #[serde(default)]
    pub favorite: bool,
    pub preferred_target: Option<String>,
    pub last_played_at: Option<DateTime<Utc>>,
    /// Absolute folder, filled at read time (never persisted meaningfully).
    #[serde(default)]
    pub folder: String,
}

/// Fields the user can edit. `preferred_target: Some(None)` clears it.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MapPatch {
    pub name: Option<String>,
    pub favorite: Option<bool>,
    pub tags: Option<Vec<String>>,
    #[serde(deserialize_with = "crate::base::patch::nullable")]
    pub preferred_target: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapSession {
    pub map_id: String,
    pub map_name: String,
    pub target: String,
    pub activated_at: DateTime<Utc>,
    /// Started through "play offline": restored automatically when the game exits.
    pub offline: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMap {
    pub source: RemoteSource,
    pub remote_id: String,
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub preview_url: Option<String>,
    pub size_bytes: Option<u64>,
    pub tags: Vec<String>,
    pub updated_at: Option<String>,
    /// Page on the source website.
    pub page_url: String,
    /// False when the source only offers an external link (Lethamyr → Google Drive).
    pub downloadable: bool,
    /// Set when this remote map is already in the library.
    pub installed_map_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseResult {
    pub source: RemoteSource,
    pub page: u32,
    pub total_pages: Option<u32>,
    pub has_next: bool,
    pub items: Vec<RemoteMap>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub source: RemoteSource,
    pub remote_id: String,
    pub phase: DownloadPhase,
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadPhase {
    Download,
    Verify,
    Extract,
    Done,
    Failed,
}

/// Converts the previous app's snake_case `meta.json` into a [`MapEntry`].
/// `main_file` is the package actually found on disk (the legacy
/// `file_path` points to the old Windows profile and is ignored).
pub fn migrate_legacy_meta(
    raw: &serde_json::Value,
    main_file: &str,
    size_bytes: u64,
) -> Option<MapEntry> {
    let obj = raw.as_object()?;
    let id = obj.get("id")?.as_str()?.to_string();
    let s = |k: &str| obj.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let origin = match obj.get("source") {
        Some(src) => {
            let kind = src
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("imported");
            let field = |k: &str| {
                src.get(k)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            match kind {
                "bakkes_mod" => MapOrigin::BakkesMod {
                    folder_name: field("folder_name"),
                },
                "bakkes_plugins" => MapOrigin::Remote {
                    source: RemoteSource::BakkesPlugins,
                    remote_id: field("external_id"),
                    version: None,
                },
                "lethamyr" => MapOrigin::Remote {
                    source: RemoteSource::Lethamyr,
                    remote_id: field("external_id"),
                    version: None,
                },
                "other" => MapOrigin::Other {
                    url: src.get("url").and_then(|v| v.as_str()).map(str::to_string),
                },
                _ => MapOrigin::Imported,
            }
        }
        None => MapOrigin::Imported,
    };
    let added_at = s("installed_at")
        .and_then(|d| DateTime::parse_from_rfc3339(&d).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let tags = obj
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|t| t.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Some(MapEntry {
        name: s("name")
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| id.clone()),
        id,
        author: s("author"),
        description: s("description"),
        tags,
        origin,
        file_name: main_file.to_string(),
        companions: Vec::new(),
        preview_file: None,
        preview_url: None,
        size_bytes,
        added_at,
        favorite: false,
        preferred_target: s("preferred_target")
            .and_then(|t| normalize_target(&t).map(str::to_string)),
        last_played_at: None,
        folder: String::new(),
    })
}

/// Picks the main map package among candidate file names: a `*_P.upk`
/// level wins, otherwise the largest file.
pub fn pick_main_package(files: &[(String, u64)]) -> Option<String> {
    let is_level = |n: &str| {
        let l = n.to_ascii_lowercase();
        l.ends_with("_p.upk") || l.ends_with("_p.udk")
    };
    files
        .iter()
        .filter(|(n, _)| is_level(n))
        .max_by_key(|(_, size)| *size)
        .or_else(|| files.iter().max_by_key(|(_, size)| *size))
        .map(|(n, _)| n.clone())
}

/// `map_<hex nanos>` — same scheme as the previous app so ids stay unique.
pub fn new_map_id() -> String {
    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    format!("map_{nanos:x}")
}

/// Human name from a file or folder name (`Dribble_2_Overhaul.upk` → `Dribble 2 Overhaul`).
pub fn display_name(raw: &str) -> String {
    let stem = raw
        .rsplit_once('.')
        .map(|(s, ext)| {
            if MAP_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
                s
            } else {
                raw
            }
        })
        .unwrap_or(raw);
    let cleaned = stem.replace(['_', '-'], " ");
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        "Untitled map".into()
    } else {
        collapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_null_target_clears_it() {
        let clear: MapPatch = serde_json::from_str(r#"{"preferredTarget": null}"#).unwrap();
        assert_eq!(clear.preferred_target, Some(None));
        let keep: MapPatch = serde_json::from_str(r#"{"favorite": true}"#).unwrap();
        assert_eq!(keep.preferred_target, None);
        let set: MapPatch =
            serde_json::from_str(r#"{"preferredTarget": "Labs_Utopia_P.upk"}"#).unwrap();
        assert_eq!(set.preferred_target, Some(Some("Labs_Utopia_P.upk".into())));
    }

    #[test]
    fn migrates_legacy_bakkesmod_meta() {
        let raw = serde_json::json!({
            "id": "map_18ad606862c784f4",
            "name": "1500 Eversaxs Olympics  made by gidek",
            "author": null,
            "description": null,
            "tags": [],
            "source": { "kind": "bakkes_mod", "folder_name": "1500_Eversaxs_Olympics__made_by_gidek" },
            "file_path": "C:\\Users\\old\\AppData\\Local\\ALXS-RL-Mod\\maps\\map_18ad606862c784f4\\map.upk",
            "thumbnail_path": null,
            "size_bytes": 453839110,
            "installed_at": "2026-05-07T19:50:29.807522600+00:00",
            "preferred_target": null
        });
        let e = migrate_legacy_meta(&raw, "map.upk", 453_839_110).unwrap();
        assert_eq!(e.id, "map_18ad606862c784f4");
        assert_eq!(e.file_name, "map.upk");
        assert!(
            matches!(e.origin, MapOrigin::BakkesMod { ref folder_name } if folder_name.starts_with("1500"))
        );
        assert_eq!(e.added_at.to_rfc3339().get(..10), Some("2026-05-07"));
        assert!(e.preferred_target.is_none());
    }

    #[test]
    fn migrates_remote_and_unknown_kinds() {
        let raw = serde_json::json!({ "id": "m1", "name": "", "source": { "kind": "lethamyr", "external_id": "42" }, "preferred_target": "labs_utopia_p" });
        let e = migrate_legacy_meta(&raw, "map.udk", 1).unwrap();
        assert_eq!(e.name, "m1");
        assert_eq!(e.preferred_target.as_deref(), Some("Labs_Utopia_P.upk"));
        assert!(
            matches!(e.origin, MapOrigin::Remote { source: RemoteSource::Lethamyr, ref remote_id, .. } if remote_id == "42")
        );
        let weird = serde_json::json!({ "id": "m2", "source": { "kind": "martian" } });
        assert!(matches!(
            migrate_legacy_meta(&weird, "a.upk", 0).unwrap().origin,
            MapOrigin::Imported
        ));
        assert!(migrate_legacy_meta(&serde_json::json!({"name": "no id"}), "a.upk", 0).is_none());
    }

    #[test]
    fn target_validation() {
        assert_eq!(
            normalize_target("labs_underpass_p.upk"),
            Some("Labs_Underpass_P.upk")
        );
        assert_eq!(normalize_target("Labs_Utopia_P"), Some("Labs_Utopia_P.upk"));
        assert_eq!(normalize_target("Park_P.upk"), None);
        assert_eq!(normalize_target("../../Engine.upk"), None);
    }

    #[test]
    fn main_package_prefers_level_then_size() {
        let files = vec![
            ("Textures.upk".to_string(), 900),
            ("Cheese_P.upk".to_string(), 100),
        ];
        assert_eq!(pick_main_package(&files).as_deref(), Some("Cheese_P.upk"));
        let files = vec![("a.upk".to_string(), 10), ("b.udk".to_string(), 20)];
        assert_eq!(pick_main_package(&files).as_deref(), Some("b.udk"));
        assert_eq!(pick_main_package(&[]), None);
    }

    #[test]
    fn display_names() {
        assert_eq!(display_name("Dribble_2_Overhaul.upk"), "Dribble 2 Overhaul");
        assert_eq!(display_name("my-map"), "my map");
        assert_eq!(display_name("___"), "Untitled map");
    }
}
