//! Extension points of the match tracker.
//!
//! The public build does nothing here. The personal edition
//! (`--features personal`, see `docs/PERSONAL_EDITION.md`) plugs its MMR
//! features in. This is the only place where `stats` reaches `personal`:
//! keep it that way so the public build stays free of MMR code.

use tauri::AppHandle;

use crate::stats::tracker::PlayerLine;

/// Startup (after the tracker state is managed).
#[cfg(feature = "personal")]
pub fn init(app: &AppHandle) {
    crate::personal::init(app);
}
#[cfg(not(feature = "personal"))]
pub fn init(_: &AppHandle) {}

/// The Stats API connected for the first time since the game started.
#[cfg(feature = "personal")]
pub fn on_game_session_start(app: &AppHandle) {
    crate::personal::service::on_game_session_start(app);
}
#[cfg(not(feature = "personal"))]
pub fn on_game_session_start(_: &AppHandle) {}

/// A live update of the match (players with their Stats API ids).
#[cfg(feature = "personal")]
pub fn on_live(app: &AppHandle, players: &[PlayerLine]) {
    crate::personal::service::on_live(app, players);
}
#[cfg(not(feature = "personal"))]
pub fn on_live(_: &AppHandle, _: &[PlayerLine]) {}

/// A match was recorded in the session.
#[cfg(feature = "personal")]
pub fn on_match_recorded(app: &AppHandle) {
    crate::personal::service::on_match_recorded(app);
}
#[cfg(not(feature = "personal"))]
pub fn on_match_recorded(_: &AppHandle) {}

/// The user started a new session.
#[cfg(feature = "personal")]
pub fn on_session_reset(app: &AppHandle) {
    crate::personal::service::on_session_reset(app);
}
#[cfg(not(feature = "personal"))]
pub fn on_session_reset(_: &AppHandle) {}
