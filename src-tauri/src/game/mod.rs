//! Everything about the installed game: where it is, which build, whether it
//! runs, and the write layer every feature must use to modify it.

pub mod fingerprint;
pub mod install;
pub mod tfc;
pub mod watcher;
pub mod writer;

use serde::Serialize;
use tauri::AppHandle;

use crate::base::config::{self, AppConfigPatch};
use crate::base::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub install: Option<install::RlInstall>,
    pub running: watcher::RunningState,
    pub build: Option<fingerprint::BuildFingerprint>,
}

/// Result of a feature re-building its game files after an update or a
/// "verify files" pass. Returned by every `<feature>::reapply_after_update`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "kind", content = "reason")]
pub enum ReapplyOutcome {
    NothingToDo,
    Reapplied,
    /// Could not be done automatically; the message explains why.
    NeedsUser(String),
}

pub fn status_snapshot(app: &AppHandle) -> GameStatus {
    let install = install::current_install(app).ok();
    let build = install.as_ref().and_then(fingerprint::current);
    GameStatus {
        install,
        running: watcher::latest(),
        build,
    }
}

pub fn init(app: &AppHandle) -> AppResult<()> {
    watcher::spawn(app.clone());
    Ok(())
}

#[tauri::command]
pub fn game_status(app: AppHandle) -> AppResult<GameStatus> {
    Ok(status_snapshot(&app))
}

/// Sets (or clears, with `None`) the manual install folder.
#[tauri::command]
pub fn game_set_install_dir(app: AppHandle, path: Option<String>) -> AppResult<GameStatus> {
    let root = match path {
        Some(p) if !p.trim().is_empty() => {
            Some(install::validate_user_root(std::path::Path::new(&p))?)
        }
        _ => None,
    };
    config::update(
        &app,
        AppConfigPatch {
            rl_install_override: Some(root),
            ..Default::default()
        },
    )?;
    let status = status_snapshot(&app);
    if status.install.is_none() {
        return Err(AppError::RocketLeagueNotFound);
    }
    Ok(status)
}
