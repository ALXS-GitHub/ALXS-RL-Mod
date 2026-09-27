//! Filesystem locations owned by the app. Nothing is hard-coded to a user
//! profile: everything derives from the OS known folders.

use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::base::error::{AppError, AppResult};

/// Folder name under `%LOCALAPPDATA%`. Kept identical to the previous app so
/// existing user data (maps, palettes) is picked up.
pub const APP_DIR_NAME: &str = "ALXS-RL-Mod";

/// `%LOCALAPPDATA%\ALXS-RL-Mod\` (created on demand).
pub fn data_dir() -> AppResult<PathBuf> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| AppError::Internal("cannot resolve the user data directory".into()))?;
    ensure(base.data_local_dir().join(APP_DIR_NAME))
}

/// A sub-directory of [`data_dir`], created on demand.
pub fn data_subdir(name: &str) -> AppResult<PathBuf> {
    ensure(data_dir()?.join(name))
}

pub fn logs_dir() -> AppResult<PathBuf> {
    data_subdir("logs")
}

pub fn config_file() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("config.json"))
}

/// Bundled resources (`src-tauri/resources/**` mapped in `tauri.conf.json`).
/// In dev, Tauri resolves this to the source tree.
pub fn resource_path(app: &tauri::AppHandle, relative: &str) -> AppResult<PathBuf> {
    app.path()
        .resolve(relative, tauri::path::BaseDirectory::Resource)
        .map_err(|e| AppError::Internal(format!("resource {relative}: {e}")))
}

/// `%APPDATA%` (roaming) — used to locate BakkesMod / AlphaConsole data.
pub fn roaming_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.data_dir().to_path_buf())
}

/// `%LOCALAPPDATA%` — used to locate the game's own user folder (replays, logs).
pub fn local_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.data_local_dir().to_path_buf())
}

/// `Documents\My Games\Rocket League\TAGame` — replays and user config live here.
pub fn rl_user_dir() -> Option<PathBuf> {
    directories::UserDirs::new()
        .and_then(|u| u.document_dir().map(Path::to_path_buf))
        .map(|d| d.join("My Games").join("Rocket League").join("TAGame"))
}

fn ensure(p: PathBuf) -> AppResult<PathBuf> {
    std::fs::create_dir_all(&p)?;
    Ok(p)
}
