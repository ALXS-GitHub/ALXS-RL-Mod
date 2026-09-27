//! IPC commands of the extras module.

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::extras::launch::LaunchResult;
use crate::extras::{bakkes, launch, replays};
use crate::game::{install, watcher};

/// Starts the game. With EAC it goes through the store launcher; it is
/// refused while a custom map is installed (servers flag modified arenas).
#[tauri::command]
pub fn launch_game(app: AppHandle, with_eac: bool) -> AppResult<LaunchResult> {
    if watcher::probe_now().running {
        return Err(AppError::Conflict(
            "Rocket League is already running".into(),
        ));
    }
    if with_eac {
        if let Some(session) = crate::maps::active_session() {
            return Err(AppError::Conflict(format!(
                "custom map \"{}\" is installed — restore the stock arena before playing online",
                session.map_name
            )));
        }
    }
    let rl = install::current_install(&app)?;
    launch::launch(&app, &rl, with_eac)
}

#[tauri::command]
pub async fn replays_list() -> AppResult<replays::ReplayList> {
    tauri::async_runtime::spawn_blocking(replays::list)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub fn bakkesmod_status() -> AppResult<bakkes::BakkesStatus> {
    Ok(bakkes::status())
}
