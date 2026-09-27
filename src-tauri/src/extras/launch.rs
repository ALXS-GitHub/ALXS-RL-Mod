//! Starting Rocket League.
//!
//! - **Without EAC** (offline modding, custom maps): the no-EAC executable is
//!   spawned directly.
//! - **With EAC** (online play): we go through the store launcher so the
//!   game gets its normal authentication — Epic's `com.epicgames.launcher://`
//!   URI or Steam's `steam://rungameid/`. Manual installs fall back to the
//!   EAC executable.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::base::error::{AppError, AppResult};
use crate::game::install::{InstallSource, RlInstall};

const EPIC_LAUNCH_URI: &str = "com.epicgames.launcher://apps/Sugar?action=launch&silent=true";
const STEAM_LAUNCH_URI: &str = "steam://rungameid/252950";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LaunchMethod {
    Direct,
    EpicLauncher,
    SteamLauncher,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub with_eac: bool,
    pub method: LaunchMethod,
    pub pid: Option<u32>,
}

/// Spawns the no-EAC executable from its own folder. Returns its pid.
pub fn spawn_no_eac(rl: &RlInstall) -> AppResult<u32> {
    spawn(&rl.exe_no_eac)
}

fn spawn(exe: &std::path::Path) -> AppResult<u32> {
    if !exe.is_file() {
        return Err(AppError::FileNotFound(exe.display().to_string()));
    }
    let mut cmd = std::process::Command::new(exe);
    if let Some(dir) = exe.parent() {
        cmd.current_dir(dir);
    }
    let child = cmd
        .spawn()
        .map_err(|e| AppError::Internal(format!("could not start {}: {e}", exe.display())))?;
    tracing::info!(exe = %exe.display(), pid = child.id(), "game started");
    Ok(child.id())
}

/// Picks how to start the game for the given install and mode.
pub fn method_for(source: InstallSource, with_eac: bool) -> LaunchMethod {
    match (with_eac, source) {
        (false, _) => LaunchMethod::Direct,
        (true, InstallSource::Epic) => LaunchMethod::EpicLauncher,
        (true, InstallSource::Steam) => LaunchMethod::SteamLauncher,
        (true, InstallSource::Manual) => LaunchMethod::Direct,
    }
}

pub fn launch(app: &AppHandle, rl: &RlInstall, with_eac: bool) -> AppResult<LaunchResult> {
    let method = method_for(rl.source, with_eac);
    let pid = match method {
        LaunchMethod::Direct => Some(if with_eac {
            spawn(&rl.exe_eac)?
        } else {
            spawn_no_eac(rl)?
        }),
        LaunchMethod::EpicLauncher => {
            app.opener()
                .open_url(EPIC_LAUNCH_URI, None::<&str>)
                .map_err(|e| AppError::Internal(format!("Epic Games launcher: {e}")))?;
            None
        }
        LaunchMethod::SteamLauncher => {
            app.opener()
                .open_url(STEAM_LAUNCH_URI, None::<&str>)
                .map_err(|e| AppError::Internal(format!("Steam: {e}")))?;
            None
        }
    };
    Ok(LaunchResult {
        with_eac,
        method,
        pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eac_goes_through_the_store_launcher() {
        assert_eq!(
            method_for(InstallSource::Epic, true),
            LaunchMethod::EpicLauncher
        );
        assert_eq!(
            method_for(InstallSource::Steam, true),
            LaunchMethod::SteamLauncher
        );
        assert_eq!(
            method_for(InstallSource::Manual, true),
            LaunchMethod::Direct
        );
        assert_eq!(method_for(InstallSource::Epic, false), LaunchMethod::Direct);
    }
}
