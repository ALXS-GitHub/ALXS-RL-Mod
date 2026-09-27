//! Rocket League install detection (manual override → Epic → Steam).

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::AppHandle;

use crate::base::config;
use crate::base::error::{AppError, AppResult};

/// Epic's internal app name for Rocket League.
const EPIC_APP_NAME: &str = "Sugar";
const STEAM_APP_ID: &str = "252950";
pub const EXE_NO_EAC: &str = "RocketLeague.exe";
pub const EXE_EAC: &str = "RocketLeague_EAC.exe";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallSource {
    Manual,
    Epic,
    Steam,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RlInstall {
    pub root: PathBuf,
    pub cooked_dir: PathBuf,
    /// `CookedPCConsole/mods/` — native override layer (may not exist yet).
    pub mods_dir: PathBuf,
    pub config_dir: PathBuf,
    pub exe_no_eac: PathBuf,
    pub exe_eac: PathBuf,
    pub source: InstallSource,
}

impl RlInstall {
    fn from_root(root: PathBuf, source: InstallSource) -> Option<Self> {
        let root = dunce::canonicalize(&root).unwrap_or(root);
        let cooked_dir = root.join("TAGame").join("CookedPCConsole");
        if !cooked_dir.is_dir() {
            return None;
        }
        let bin = root.join("Binaries").join("Win64");
        Some(Self {
            mods_dir: cooked_dir.join("mods"),
            config_dir: root.join("TAGame").join("Config"),
            exe_no_eac: bin.join(EXE_NO_EAC),
            exe_eac: bin.join(EXE_EAC),
            cooked_dir,
            root,
            source,
        })
    }

    /// True when `path` is inside this install (used by the write layer to
    /// refuse writes anywhere else).
    pub fn contains(&self, path: &Path) -> bool {
        let p = dunce::simplified(path);
        p.starts_with(&self.root)
    }
}

/// Resolves the install from config override or auto-detection.
pub fn current_install(app: &AppHandle) -> AppResult<RlInstall> {
    let cfg = config::current(app);
    if let Some(root) = cfg.rl_install_override {
        return RlInstall::from_root(root, InstallSource::Manual)
            .ok_or(AppError::RocketLeagueNotFound);
    }
    detect().ok_or(AppError::RocketLeagueNotFound)
}

/// Validates a user-picked folder. Accepts the install root or any folder
/// below it (e.g. the user picked `TAGame/CookedPCConsole`).
pub fn validate_user_root(picked: &Path) -> AppResult<PathBuf> {
    for candidate in picked.ancestors() {
        if RlInstall::from_root(candidate.to_path_buf(), InstallSource::Manual).is_some() {
            return Ok(candidate.to_path_buf());
        }
    }
    Err(AppError::InvalidInput(format!(
        "{} is not a Rocket League install",
        picked.display()
    )))
}

pub fn detect() -> Option<RlInstall> {
    detect_epic()
        .and_then(|r| RlInstall::from_root(r, InstallSource::Epic))
        .or_else(|| detect_steam().and_then(|r| RlInstall::from_root(r, InstallSource::Steam)))
}

/// Epic writes one `*.item` JSON manifest per installed game.
fn detect_epic() -> Option<PathBuf> {
    let program_data = std::env::var_os("ProgramData").map(PathBuf::from)?;
    let manifests = program_data
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    if let Ok(entries) = std::fs::read_dir(&manifests) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("item") {
                continue;
            }
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            if json.get("AppName").and_then(|v| v.as_str()) == Some(EPIC_APP_NAME) {
                if let Some(loc) = json.get("InstallLocation").and_then(|v| v.as_str()) {
                    return Some(PathBuf::from(loc));
                }
            }
        }
    }
    // Default location as a last resort.
    let pf = std::env::var_os("ProgramFiles").map(PathBuf::from)?;
    let guess = pf.join("Epic Games").join("rocketleague");
    guess.is_dir().then_some(guess)
}

#[cfg(windows)]
fn steam_root() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Valve\\Steam")
        .ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path))
}

#[cfg(not(windows))]
fn steam_root() -> Option<PathBuf> {
    None
}

fn detect_steam() -> Option<PathBuf> {
    let steam = steam_root()?;
    let vdf = std::fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf")).ok()?;
    let parsed = keyvalues_parser::parse(&vdf)
        .map(keyvalues_parser::Vdf::from)
        .ok()?;
    let folders = parsed.value.get_obj()?;
    for libs in folders.values() {
        for lib in libs {
            let Some(obj) = lib.get_obj() else { continue };
            let Some(path) = obj
                .get("path")
                .and_then(|v| v.first())
                .and_then(|v| v.get_str())
            else {
                continue;
            };
            let has_rl = obj
                .get("apps")
                .and_then(|v| v.first())
                .and_then(|v| v.get_obj())
                .is_some_and(|apps| apps.contains_key(STEAM_APP_ID));
            if has_rl {
                let root = PathBuf::from(path)
                    .join("steamapps")
                    .join("common")
                    .join("rocketleague");
                if root.is_dir() {
                    return Some(root);
                }
            }
        }
    }
    None
}
