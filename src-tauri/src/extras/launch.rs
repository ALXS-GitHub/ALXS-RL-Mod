//! Starting Rocket League.
//!
//! - **Without EAC** (offline modding, custom maps): Epic installs can only
//!   be started that way from the Epic Games Launcher ("Launch without
//!   Anti-Cheat": Epic adds `-noeac` to Psyonix's `Launcher.exe`). Started
//!   directly, `RocketLeague.exe` has no Epic login, exits and asks Epic to
//!   relaunch it — with EAC. So for Epic we only bring the launcher forward
//!   and the user picks that menu entry; Steam / manual installs spawn the
//!   no-EAC executable directly.
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
    /// The user starts it from Epic ("Launch without Anti-Cheat").
    EpicNoEac,
}

/// How a no-EAC game start went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoEacStart {
    Spawned(u32),
    /// Epic was brought forward: the user picks "Launch without Anti-Cheat".
    InEpic,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub with_eac: bool,
    pub method: LaunchMethod,
    pub pid: Option<u32>,
}

/// Spawns the no-EAC executable from its own folder. Returns its pid.
/// Not for Epic installs (see the module doc): use [`start_no_eac`].
fn spawn_no_eac(rl: &RlInstall) -> AppResult<u32> {
    spawn(&rl.exe_no_eac)
}

/// Starts the game without EAC the only way that works for this install.
pub fn start_no_eac(rl: &RlInstall) -> AppResult<NoEacStart> {
    match rl.source {
        InstallSource::Epic => {
            open_epic_launcher();
            Ok(NoEacStart::InEpic)
        }
        InstallSource::Steam | InstallSource::Manual => spawn_no_eac(rl).map(NoEacStart::Spawned),
    }
}

/// Executable registered for `com.epicgames.launcher://` links.
#[cfg(windows)]
fn epic_launcher_exe() -> Option<std::path::PathBuf> {
    use winreg::enums::HKEY_CLASSES_ROOT;
    use winreg::RegKey;
    let command: String = RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(r"com.epicgames.launcher\shell\open\command")
        .ok()?
        .get_value("")
        .ok()?;
    epic_exe_from_command(&command)
}

#[cfg(not(windows))]
fn epic_launcher_exe() -> Option<std::path::PathBuf> {
    None
}

/// `"C:\…\EpicGamesLauncher.exe" %1` → the executable path.
fn epic_exe_from_command(command: &str) -> Option<std::path::PathBuf> {
    let c = command.trim();
    let exe = match c.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => c.split_whitespace().next()?,
    };
    Some(std::path::PathBuf::from(exe))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")))
}

/// Brings the Epic Games Launcher forward (starting it if needed). Best effort.
fn open_epic_launcher() {
    match epic_launcher_exe().filter(|p| p.is_file()) {
        Some(exe) => {
            if let Err(err) = std::process::Command::new(&exe).spawn() {
                tracing::warn!(%err, "could not open the Epic Games Launcher");
            }
        }
        None => tracing::warn!("Epic Games Launcher not found"),
    }
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
        (false, InstallSource::Epic) => LaunchMethod::EpicNoEac,
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
        LaunchMethod::EpicNoEac => {
            open_epic_launcher();
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
        assert_eq!(
            method_for(InstallSource::Epic, false),
            LaunchMethod::EpicNoEac
        );
        assert_eq!(
            method_for(InstallSource::Steam, false),
            LaunchMethod::Direct
        );
    }

    #[test]
    fn epic_exe_from_registry_command() {
        assert_eq!(
            epic_exe_from_command(
                r#""C:\Program Files\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe" %1"#
            ),
            Some(std::path::PathBuf::from(
                r"C:\Program Files\Epic Games\Launcher\Portal\Binaries\Win64\EpicGamesLauncher.exe"
            ))
        );
        assert_eq!(
            epic_exe_from_command(r"C:\Epic\EpicGamesLauncher.exe %1"),
            Some(std::path::PathBuf::from(r"C:\Epic\EpicGamesLauncher.exe"))
        );
        assert_eq!(epic_exe_from_command(""), None);
        assert_eq!(epic_exe_from_command("notepad"), None);
    }
}
