//! Match tracker built on the official Stats API: enabling it (user ini),
//! the WebSocket client, the session tracker, MMR lookups and the overlay.

pub mod browser;
pub mod client;
pub mod commands;
pub mod ini;
pub mod lobby;
pub mod mmr;
pub mod overlay;
pub mod protocol;
pub mod tracker;

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use crate::base::config;
use crate::base::error::AppResult;
use crate::game::ReapplyOutcome;

/// Managed state: the live session tracker.
pub struct StatsState(pub client::Shared);

pub fn init(app: &AppHandle) -> AppResult<()> {
    let hint = config::current(app).tracker.player_id;
    let shared: client::Shared = Arc::new(Mutex::new(tracker::Tracker::new(
        client::load_session(),
        hint,
    )));
    app.manage(StatsState(shared.clone()));
    client::spawn(app.clone(), shared);
    enable_by_default(app);
    Ok(())
}

/// The Stats API is on unless the user turned it off. The game reads the
/// ini at startup: if it is already running, this applies next launch.
fn enable_by_default(app: &AppHandle) {
    if config::current(app).stats_api_opt_out {
        return;
    }
    let enabled = ini::current().map(|(_, s)| s.enabled()).unwrap_or(false);
    if enabled {
        return;
    }
    match ini::enable() {
        Ok(_) => tracing::info!("Stats API enabled (default)"),
        Err(err) => tracing::warn!(%err, "could not enable the Stats API"),
    }
}

/// A game update can regenerate the user ini and switch the API back off.
/// Not part of the integrity contract yet; safe to call anytime.
pub fn reapply_after_update(_app: &AppHandle) -> AppResult<ReapplyOutcome> {
    if ini::needs_reapply() {
        ini::enable()?;
        return Ok(ReapplyOutcome::Reapplied);
    }
    Ok(ReapplyOutcome::NothingToDo)
}
