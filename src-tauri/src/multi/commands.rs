//! IPC commands of the multi module (names listed in docs/ARCHITECTURE.md).

use serde::Serialize;
use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::game::watcher;
use crate::multi::bakkes_cfg::{self, BakkesSetup};
use crate::multi::invite::{self, Invite};
use crate::multi::session::{self, MultiSession};
use crate::multi::tailscale::{self, TailscaleStatus};

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiStatus {
    pub tailscale: TailscaleStatus,
    pub bakkes: BakkesSetup,
    pub game_running: bool,
    pub game_with_eac: bool,
    pub port: u16,
}

#[tauri::command]
pub async fn multi_status() -> AppResult<MultiStatus> {
    blocking(|| {
        let game = watcher::probe_now();
        Ok(MultiStatus {
            tailscale: tailscale::status(),
            bakkes: bakkes_cfg::setup(),
            game_running: game.running,
            game_with_eac: game.with_eac,
            port: invite::DEFAULT_PORT,
        })
    })
    .await
}

/// Turns BakkesMod's RCON on for Rocket Plugin (taken into account at the
/// next game start).
#[tauri::command]
pub async fn multi_bakkes_setup() -> AppResult<BakkesSetup> {
    blocking(|| {
        bakkes_cfg::apply()?;
        Ok(bakkes_cfg::setup())
    })
    .await
}

/// Removes everything [`multi_bakkes_setup`] added.
#[tauri::command]
pub async fn multi_bakkes_undo() -> AppResult<BakkesSetup> {
    blocking(|| {
        bakkes_cfg::undo()?;
        Ok(bakkes_cfg::setup())
    })
    .await
}

#[tauri::command]
pub async fn multi_host(app: AppHandle, map_id: String) -> AppResult<MultiSession> {
    blocking(move || session::host(&app, map_id)).await
}

/// Reads an invite code without acting on it (preview before joining).
#[tauri::command]
pub fn multi_decode(code: String) -> AppResult<Invite> {
    invite::decode(&code)
}

#[tauri::command]
pub async fn multi_join(app: AppHandle, code: String) -> AppResult<MultiSession> {
    blocking(move || session::join(&app, &code)).await
}

#[tauri::command]
pub fn multi_session() -> AppResult<Option<MultiSession>> {
    Ok(session::current())
}

/// Stops the automation; the game, BakkesMod and the installed map stay.
#[tauri::command]
pub fn multi_cancel(app: AppHandle) -> AppResult<()> {
    session::cancel(&app);
    Ok(())
}
