//! Persisted user configuration (`config.json`).
//!
//! Only cross-cutting settings live here. Feature state (active swaps,
//! palettes, maps…) is owned by each feature module in its own files.

use std::path::PathBuf;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    En,
    Fr,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum FxQuality {
    Off,
    Low,
    #[default]
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackerConfig {
    /// `epic` | `steam` | `psn` | `xbl` | `switch`
    pub platform: Option<String>,
    pub player_id: Option<String>,
    /// Players followed from the MMR tab (one click to look them up).
    pub favorites: Vec<FavoritePlayer>,
    /// Online ratings (MMR, lobby ranks) come from tracker.gg's public
    /// pages; off until the user turns them on.
    pub online_ratings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FavoritePlayer {
    pub platform: String,
    pub player_id: String,
}

pub const MAX_FAVORITES: usize = 30;

impl TrackerConfig {
    /// Trims ids, drops empty ones and duplicates (same platform, id
    /// ignoring case), keeps at most [`MAX_FAVORITES`].
    fn normalized(mut self) -> Self {
        let mut seen = std::collections::HashSet::new();
        self.favorites = self
            .favorites
            .into_iter()
            .map(|f| FavoritePlayer {
                platform: f.platform.trim().to_ascii_lowercase(),
                player_id: f.player_id.trim().to_string(),
            })
            .filter(|f| !f.platform.is_empty() && !f.player_id.is_empty())
            .filter(|f| seen.insert((f.platform.clone(), f.player_id.to_lowercase())))
            .take(MAX_FAVORITES)
            .collect();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    /// Manual install root (`None` = auto-detect Epic / Steam).
    pub rl_install_override: Option<PathBuf>,
    pub locale: Locale,
    pub fx_quality: FxQuality,
    /// Pause the WebGL layer while Rocket League is the foreground window.
    pub fx_pause_when_game_focused: bool,
    /// Rebuild swaps / palette / decals automatically after a game update.
    pub auto_reapply_after_update: bool,
    /// Put the stock map back when the game exits after an offline session.
    pub restore_maps_on_game_exit: bool,
    pub tracker: TrackerConfig,
    /// The user turned the Stats API off: stop enabling it at startup
    /// (it is on by default).
    pub stats_api_opt_out: bool,
    /// AlphaConsole-style decal libraries (empty = BakkesMod default path).
    pub decal_library_folders: Vec<PathBuf>,
    pub onboarding_done: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            rl_install_override: None,
            locale: Locale::En,
            fx_quality: FxQuality::High,
            fx_pause_when_game_focused: true,
            auto_reapply_after_update: true,
            restore_maps_on_game_exit: true,
            tracker: TrackerConfig::default(),
            stats_api_opt_out: false,
            decal_library_folders: Vec::new(),
            onboarding_done: false,
        }
    }
}

/// Partial update sent by the front-end. `None` = leave untouched.
/// `rl_install_override: Some(None)` clears the override.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfigPatch {
    #[serde(deserialize_with = "crate::base::patch::nullable")]
    pub rl_install_override: Option<Option<PathBuf>>,
    pub locale: Option<Locale>,
    pub fx_quality: Option<FxQuality>,
    pub fx_pause_when_game_focused: Option<bool>,
    pub auto_reapply_after_update: Option<bool>,
    pub restore_maps_on_game_exit: Option<bool>,
    pub tracker: Option<TrackerConfig>,
    pub stats_api_opt_out: Option<bool>,
    pub decal_library_folders: Option<Vec<PathBuf>>,
    pub onboarding_done: Option<bool>,
}

impl AppConfig {
    pub fn load() -> AppResult<Self> {
        fsx::read_json_or_default(&paths::config_file()?)
    }

    pub fn save(&self) -> AppResult<()> {
        fsx::write_json(&paths::config_file()?, self)
    }

    fn apply(&mut self, patch: AppConfigPatch) {
        if let Some(v) = patch.rl_install_override {
            self.rl_install_override = v;
        }
        if let Some(v) = patch.locale {
            self.locale = v;
        }
        if let Some(v) = patch.fx_quality {
            self.fx_quality = v;
        }
        if let Some(v) = patch.fx_pause_when_game_focused {
            self.fx_pause_when_game_focused = v;
        }
        if let Some(v) = patch.auto_reapply_after_update {
            self.auto_reapply_after_update = v;
        }
        if let Some(v) = patch.restore_maps_on_game_exit {
            self.restore_maps_on_game_exit = v;
        }
        if let Some(v) = patch.tracker {
            self.tracker = v.normalized();
        }
        if let Some(v) = patch.stats_api_opt_out {
            self.stats_api_opt_out = v;
        }
        if let Some(v) = patch.decal_library_folders {
            self.decal_library_folders = v;
        }
        if let Some(v) = patch.onboarding_done {
            self.onboarding_done = v;
        }
    }
}

pub struct ConfigState(pub RwLock<AppConfig>);

/// Snapshot of the current config (cheap clone, never holds the lock).
pub fn current(app: &AppHandle) -> AppConfig {
    app.state::<ConfigState>()
        .0
        .read()
        .map(|c| c.clone())
        .unwrap_or_default()
}

/// Applies `patch`, persists, returns the new config.
pub fn update(app: &AppHandle, patch: AppConfigPatch) -> AppResult<AppConfig> {
    let state = app.state::<ConfigState>();
    let mut guard = state
        .0
        .write()
        .map_err(|_| AppError::Internal("config lock poisoned".into()))?;
    guard.apply(patch);
    guard.save()?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn config_get(state: State<'_, ConfigState>) -> AppResult<AppConfig> {
    state
        .0
        .read()
        .map(|c| c.clone())
        .map_err(|_| AppError::Internal("config lock poisoned".into()))
}

#[tauri::command]
pub fn config_update(app: AppHandle, patch: AppConfigPatch) -> AppResult<AppConfig> {
    update(&app, patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorites_are_cleaned() {
        let fav = |p: &str, id: &str| FavoritePlayer {
            platform: p.into(),
            player_id: id.into(),
        };
        let cfg = TrackerConfig {
            favorites: vec![
                fav("Epic", " Bob "),
                fav("epic", "bob"),
                fav("steam", ""),
                fav("psn", "Bob"),
            ],
            ..Default::default()
        }
        .normalized();
        assert_eq!(cfg.favorites, vec![fav("epic", "Bob"), fav("psn", "Bob")]);
    }

    #[test]
    fn patch_null_install_override_clears_it() {
        let clear: AppConfigPatch = serde_json::from_str(r#"{"rlInstallOverride": null}"#).unwrap();
        assert!(matches!(clear.rl_install_override, Some(None)));
        let keep: AppConfigPatch = serde_json::from_str(r#"{"locale": "fr"}"#).unwrap();
        assert!(keep.rl_install_override.is_none());
    }

    #[test]
    fn patch_only_touches_provided_fields() {
        let mut cfg = AppConfig::default();
        cfg.apply(AppConfigPatch {
            locale: Some(Locale::Fr),
            ..Default::default()
        });
        assert_eq!(cfg.locale, Locale::Fr);
        assert_eq!(cfg.fx_quality, FxQuality::High);
    }

    #[test]
    fn override_can_be_cleared() {
        let mut cfg = AppConfig {
            rl_install_override: Some("C:/RL".into()),
            ..Default::default()
        };
        cfg.apply(AppConfigPatch {
            rl_install_override: Some(None),
            ..Default::default()
        });
        assert!(cfg.rl_install_override.is_none());
    }

    #[test]
    fn unknown_fields_are_ignored_and_missing_fields_defaulted() {
        let cfg: AppConfig = serde_json::from_str(r#"{"locale":"fr","legacyField":1}"#).unwrap();
        assert_eq!(cfg.locale, Locale::Fr);
        assert!(cfg.auto_reapply_after_update);
    }
}
