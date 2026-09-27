//! IPC commands of the stats / tracker module.

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::base::error::{AppError, AppResult};
use crate::game::watcher;
use crate::stats::client;
use crate::stats::ini::{self, IniSettings};
use crate::stats::mmr::{self, MmrProfile};
use crate::stats::overlay;
use crate::stats::tracker::{Connection, SessionState};
use crate::stats::StatsState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsStatus {
    /// The user ini we edit (`None` if the game never created its config folder).
    pub ini_path: Option<String>,
    pub settings: IniSettings,
    pub enabled: bool,
    pub connection: Connection,
    pub game_running: bool,
    pub overlay_open: bool,
    /// Why the ini could not be read (e.g. game never launched).
    pub problem: Option<String>,
}

fn status(app: &AppHandle, state: &StatsState) -> StatsStatus {
    let connection = state
        .0
        .lock()
        .map(|t| t.state.connection)
        .unwrap_or_default();
    let base =
        |problem: Option<String>, ini_path: Option<String>, settings: IniSettings| StatsStatus {
            enabled: settings.enabled(),
            ini_path,
            settings,
            connection,
            game_running: watcher::latest().running,
            overlay_open: overlay::is_open(app),
            problem,
        };
    match ini::current() {
        Ok((path, settings)) => base(None, Some(path.to_string_lossy().into_owned()), settings),
        Err(err) => base(Some(err.to_string()), None, IniSettings::default()),
    }
}

#[tauri::command]
pub fn stats_status(app: AppHandle, state: State<'_, StatsState>) -> AppResult<StatsStatus> {
    Ok(status(&app, &state))
}

/// Enables the Stats API in the user ini. Takes effect at the next game start.
/// The Stats API is on by default; turning it off is remembered so the app
/// does not switch it back on at the next start.
fn remember_choice(app: &AppHandle, opt_out: bool) {
    let patch = crate::base::config::AppConfigPatch {
        stats_api_opt_out: Some(opt_out),
        ..Default::default()
    };
    if let Err(err) = crate::base::config::update(app, patch) {
        tracing::warn!(%err, "could not save the Stats API choice");
    }
}

#[tauri::command]
pub fn stats_enable(app: AppHandle, state: State<'_, StatsState>) -> AppResult<StatsStatus> {
    ini::enable()?;
    remember_choice(&app, false);
    Ok(status(&app, &state))
}

#[tauri::command]
pub fn stats_disable(app: AppHandle, state: State<'_, StatsState>) -> AppResult<StatsStatus> {
    ini::disable()?;
    remember_choice(&app, true);
    Ok(status(&app, &state))
}

#[tauri::command]
pub fn tracker_session(state: State<'_, StatsState>) -> AppResult<SessionState> {
    state
        .0
        .lock()
        .map(|t| t.state.clone())
        .map_err(|_| AppError::Internal("tracker lock poisoned".into()))
}

/// Starts a new session (counters, streak, history of this session).
#[tauri::command]
pub fn tracker_reset(app: AppHandle, state: State<'_, StatsState>) -> AppResult<SessionState> {
    let snapshot = {
        let mut t = state
            .0
            .lock()
            .map_err(|_| AppError::Internal("tracker lock poisoned".into()))?;
        t.reset();
        client::persist(&t.state);
        t.state.clone()
    };
    client::emit_session(&app, &state.0);
    Ok(snapshot)
}

#[tauri::command]
pub async fn mmr_lookup(app: AppHandle, platform: String, player: String) -> AppResult<MmrProfile> {
    mmr::lookup(&app, &platform, &player).await
}

// Async on purpose: creating or destroying a window from a synchronous
// command deadlocks the main thread on Windows.
#[tauri::command]
pub async fn overlay_open(app: AppHandle, state: State<'_, StatsState>) -> AppResult<StatsStatus> {
    overlay::open(&app)?;
    // The overlay listens to events only: push the current state right away.
    client::emit_session(&app, &state.0);
    Ok(status(&app, &state))
}

#[tauri::command]
pub async fn overlay_close(app: AppHandle, state: State<'_, StatsState>) -> AppResult<StatsStatus> {
    overlay::close(&app)?;
    Ok(status(&app, &state))
}
