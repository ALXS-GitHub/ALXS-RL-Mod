//! IPC commands of the multi module (names listed in docs/ARCHITECTURE.md).

use serde::Serialize;
use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::base::winsec::{self, SmartAppControl};
use crate::game::watcher;
use crate::multi::bakkes_cfg::{self, BakkesSetup};
use crate::multi::invite;
use crate::multi::presets::{self, HostPreset, HostPresetInput};
use crate::multi::rp_options::{self, MatchOptions, RpCatalog};
use crate::multi::session::{self, InviteCheck, MultiSession};
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
    /// Smart App Control blocks unsigned programs such as BakkesMod.exe.
    pub smart_app_control: SmartAppControl,
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
            smart_app_control: winsec::smart_app_control(),
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

/// Opens BakkesMod's plugin installer on Rocket Plugin (the user confirms there).
#[tauri::command]
pub async fn multi_install_rocket_plugin() -> AppResult<()> {
    blocking(bakkes_cfg::install_rocket_plugin).await
}

/// `target`: Labs arena to replace (`None` = the map's preferred one, else
/// Underpass). `variant`: 0 = original, N = copy repaired with mapping vN,
/// `None` = the map's default.
#[tauri::command]
pub async fn multi_host(
    app: AppHandle,
    map_id: String,
    target: Option<String>,
    variant: Option<u32>,
    options: Option<MatchOptions>,
) -> AppResult<MultiSession> {
    blocking(move || session::host(&app, map_id, target, variant, options.unwrap_or_default()))
        .await
}

#[tauri::command]
pub async fn multi_presets_list() -> AppResult<Vec<HostPreset>> {
    blocking(presets::list).await
}

/// Saves a host setup (`id` set = overwrite it).
#[tauri::command]
pub async fn multi_presets_save(preset: HostPresetInput) -> AppResult<HostPreset> {
    blocking(move || presets::save(preset)).await
}

#[tauri::command]
pub async fn multi_presets_delete(id: String) -> AppResult<()> {
    blocking(move || presets::delete(&id)).await
}

/// Rocket Plugin game modes, mutators and the user's presets.
#[tauri::command]
pub async fn multi_rp_options() -> AppResult<RpCatalog> {
    blocking(|| Ok(rp_options::catalog())).await
}

/// Reads an invite code and looks its map up in the library, without
/// acting on it (the join card offers the download if it is missing).
#[tauri::command]
pub async fn multi_check(code: String) -> AppResult<InviteCheck> {
    blocking(move || session::check(&code)).await
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
