//! Personal edition — compiled only with `--features personal`.
//!
//! Everything MMR lives here: tracker.gg lookups through a hidden browser
//! (`browser`, `mmr`), the ratings of the players of your match (`lobby`)
//! and the session MMR state (`ratings`, `service`). tracker.gg forbids
//! automated access and has no Rocket League API, so none of this ships in
//! the public releases: the default build does not even compile it.
//!
//! The match tracker reaches this module only through `stats::hooks`.
//! How to build and install this edition: `docs/PERSONAL_EDITION.md`.

pub mod browser;
pub mod commands;
pub mod lobby;
pub mod mmr;
pub mod ratings;
pub mod service;

use tauri::AppHandle;

/// Called once at startup by `stats::hooks::init`.
pub fn init(app: &AppHandle) {
    service::init(app);
}
