//! Map library, community sources and the active map session.
//!
//! A map is "played" by dropping it into `CookedPCConsole/mods/` under the
//! name of a Labs arena (the game's native override layer). The stock arena
//! is never modified. Only one map session exists at a time.

pub mod commands;
pub mod library;
pub mod model;
pub mod session;
pub mod sources;

use tauri::AppHandle;

use crate::base::config;
use crate::base::error::AppResult;
use crate::game::{watcher, ReapplyOutcome};

pub use model::{MapEntry, MapSession};

pub fn init(app: &AppHandle) -> AppResult<()> {
    library::clean_staging();
    session::load()?;
    // Hashing map files can take a moment: off the startup path.
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(err) = session::settle(&app2) {
            tracing::warn!(%err, "could not clean up leftover map files");
        }
    });
    session::watch_game_exits(app.clone());
    // An offline session whose game already exited (app closed meanwhile):
    // restore the stock arena now, as the user asked for it.
    if let Some(s) = session::current() {
        let running = watcher::probe_now().running;
        if s.offline && !running && config::current(app).restore_maps_on_game_exit {
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = session::deactivate(&app2).await {
                    tracing::warn!(%err, "could not close the leftover offline map session");
                }
            });
        } else if s.offline && running {
            session::watch_offline_session(app.clone(), s.map_id);
        }
    }
    Ok(())
}

/// Active map session, if any (used by `extras::launch_game` to refuse EAC).
pub fn active_session() -> Option<MapSession> {
    session::current()
}

/// Id of the map installed by the active session, if any (used by presets).
pub fn active_map_id(_app: &AppHandle) -> AppResult<Option<String>> {
    Ok(session::current().map(|s| s.map_id))
}

/// Called by `integrity` after a game update or a verify pass.
pub fn reapply_after_update(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    session::reapply(app)
}
