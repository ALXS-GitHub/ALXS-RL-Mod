//! The single active map session: which library map replaces a Labs arena
//! right now.
//!
//! The map is written over the arena file itself (`CookedPCConsole/Labs_*_P.upk`,
//! `RootReplace`: the stock file is kept in `backups/` and put back on
//! restore). The game resolves package paths once, at startup: a file added
//! to `mods/` later is ignored, while the stock arena path is always known —
//! so a swap written there is picked up the next time the arena loads, even
//! while the game runs. The game holds the arena open while you play on it:
//! swapping then fails with `ArenaInUse` (go back to the main menu).
//!
//! Companion packages (rare) go to `mods/`; one that did not exist when the
//! game started is only seen after a restart. While the game runs, stale
//! companions are left in place and removed once it exits ([`settle`]).

use std::path::PathBuf;
use std::sync::RwLock;
use std::time::Duration;

use chrono::Utc;
use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::base::{config, fsx};
use crate::game::writer::{self, EntryState, ManifestEntry, Owner, Placement};
use crate::game::{install, watcher, ReapplyOutcome};
use crate::maps::library;
use crate::maps::model::{normalize_target, MapSession, DEFAULT_TARGET};

static SESSION: RwLock<Option<MapSession>> = RwLock::new(None);

fn session_file() -> AppResult<PathBuf> {
    Ok(library::library_dir()?.join("session.json"))
}

pub fn current() -> Option<MapSession> {
    SESSION.read().ok().and_then(|s| s.clone())
}

fn set(session: Option<MapSession>) -> AppResult<()> {
    let path = session_file()?;
    match &session {
        Some(s) => fsx::write_json(&path, s)?,
        None => {
            if path.is_file() {
                std::fs::remove_file(&path)?;
            }
        }
    }
    if let Ok(mut guard) = SESSION.write() {
        *guard = session;
    }
    Ok(())
}

/// Loads the persisted session and reconciles it with the write layer.
pub fn load() -> AppResult<()> {
    let path = session_file()?;
    let stored: Option<MapSession> = if path.is_file() {
        serde_json::from_slice(&std::fs::read(&path)?).ok()
    } else {
        None
    };
    // A session without files in the manifest is stale (manual cleanup…).
    let entries = writer::entries_for(Owner::Maps)?;
    let session = stored.filter(|s| entries.iter().any(|e| e.tag == s.map_id));
    if let Ok(mut guard) = SESSION.write() {
        *guard = session;
    }
    Ok(())
}

/// The arena file is open in the game (you are playing on it).
fn arena_in_use(err: AppError, target: &str) -> AppError {
    match err {
        AppError::FileLocked(_) => AppError::ArenaInUse(target.to_string()),
        other => other,
    }
}

/// Installs map `id` over a Labs arena, replacing any previous session.
/// Blocking (reads and hashes up to ~500 MB): call from a blocking context.
pub fn activate_blocking(
    app: &AppHandle,
    id: &str,
    target: Option<String>,
    offline: bool,
) -> AppResult<MapSession> {
    let entry = library::get(id)?;
    let target_name = target
        .or_else(|| entry.preferred_target.clone())
        .unwrap_or_else(|| DEFAULT_TARGET.to_string());
    let target_name = normalize_target(&target_name)
        .ok_or_else(|| AppError::InvalidInput(format!("unknown Labs target {target_name}")))?
        .to_string();

    if current().is_some() {
        deactivate_blocking(app)?;
    }

    let rl = install::current_install(app)?;
    let folder = PathBuf::from(&entry.folder);
    let install_all = || -> AppResult<()> {
        // The write layer takes the bytes in memory; one package at a time.
        let main = folder.join(&entry.file_name);
        let bytes = std::fs::read(&main).map_err(|e| AppError::from_game_io(e, &main))?;
        writer::install_file(
            app,
            Owner::Maps,
            &entry.id,
            &rl.cooked_dir.join(&target_name),
            Placement::RootReplace,
            &bytes,
        )
        .map_err(|e| arena_in_use(e, &target_name))?;
        drop(bytes);
        for companion in &entry.companions {
            let src = folder.join(companion);
            let bytes = std::fs::read(&src).map_err(|e| AppError::from_game_io(e, &src))?;
            writer::install_file(
                app,
                Owner::Maps,
                &entry.id,
                &rl.mods_dir.join(companion),
                Placement::ModsOverride,
                &bytes,
            )?;
        }
        Ok(())
    };
    if let Err(err) = install_all() {
        // Never leave half a map installed.
        let _ = writer::restore_where(app, Owner::Maps, |e| {
            e.tag == entry.id && e.placement == Placement::RootReplace
        });
        return Err(err);
    }

    let session = MapSession {
        map_id: entry.id.clone(),
        map_name: entry.name.clone(),
        target: target_name,
        activated_at: Utc::now(),
        offline,
    };
    set(Some(session.clone()))?;
    library::touch_played(&entry.id);
    tracing::info!(map = %entry.name, target = %session.target, "map activated");
    Ok(session)
}

pub async fn activate(
    app: &AppHandle,
    id: &str,
    target: Option<String>,
    offline: bool,
) -> AppResult<MapSession> {
    let (app2, id2) = (app.clone(), id.to_string());
    tauri::async_runtime::spawn_blocking(move || activate_blocking(&app2, &id2, target, offline))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Puts the stock arena back and closes the session. While the game runs,
/// companion files in `mods/` stay until it exits ([`settle`]).
pub fn deactivate_blocking(app: &AppHandle) -> AppResult<()> {
    let target = current().map(|s| s.target).unwrap_or_default();
    if watcher::probe_now().running {
        writer::restore_where(app, Owner::Maps, |e| e.placement == Placement::RootReplace)
            .map_err(|e| arena_in_use(e, &target))?;
    } else {
        writer::restore_owner(app, Owner::Maps, None)?;
    }
    set(None)?;
    tracing::info!("map session closed");
    Ok(())
}

fn file_name(rel_path: &str) -> &str {
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

/// Files the current session did not write, and arenas written into `mods/`
/// by earlier versions (moved over the stock file instead).
fn is_stale(e: &ManifestEntry, session: Option<&MapSession>) -> bool {
    match session {
        None => true,
        Some(s) => {
            let legacy_arena = e.placement == Placement::ModsOverride
                && normalize_target(file_name(&e.rel_path)).is_some();
            e.tag != s.map_id || legacy_arena
        }
    }
}

/// Game closed only: removes stale map files, and re-installs the session's
/// arena over the stock file if it was still in `mods/`.
pub fn settle(app: &AppHandle) -> AppResult<()> {
    if watcher::probe_now().running {
        return Ok(());
    }
    let session = current();
    let entries = writer::entries_for(Owner::Maps)?;
    if entries.iter().any(|e| is_stale(e, session.as_ref())) {
        writer::restore_where(app, Owner::Maps, |e| is_stale(e, session.as_ref()))?;
        tracing::info!("leftover map files removed");
    }
    if let Some(s) = session {
        let installed = writer::entries_for(Owner::Maps)?
            .iter()
            .any(|e| e.tag == s.map_id && e.placement == Placement::RootReplace);
        if !installed {
            set(None)?;
            activate_blocking(app, &s.map_id, Some(s.target.clone()), s.offline)?;
            tracing::info!(map = %s.map_name, "map session moved over the stock arena");
        }
    }
    Ok(())
}

/// Runs [`settle`] every time the game exits.
pub fn watch_game_exits(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut rx = watcher::subscribe();
        let mut was_running = rx.borrow().running;
        while rx.changed().await.is_ok() {
            let running = rx.borrow().running;
            if was_running && !running {
                // Give the game a moment to release file handles.
                tokio::time::sleep(Duration::from_secs(3)).await;
                let app2 = app.clone();
                let done = tauri::async_runtime::spawn_blocking(move || settle(&app2)).await;
                if let Ok(Err(err)) = done {
                    tracing::warn!(%err, "could not clean up map files after the game exited");
                }
            }
            was_running = running;
        }
    });
}

pub async fn deactivate(app: &AppHandle) -> AppResult<()> {
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || deactivate_blocking(&app2))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Waits for the game to start, then to exit, then restores the stock map
/// (if the user wants that). Runs detached after "play offline".
pub fn watch_offline_session(app: AppHandle, map_id: String) {
    tauri::async_runtime::spawn(async move {
        let mut rx = watcher::subscribe();
        // Phase 1: wait up to 3 minutes for the game to appear.
        let started = tokio::time::timeout(Duration::from_secs(180), async {
            loop {
                if rx.borrow().running {
                    return;
                }
                if rx.changed().await.is_err() {
                    return;
                }
            }
        })
        .await;
        if started.is_err() {
            tracing::warn!("offline session: game never started, keeping the map installed");
            return;
        }
        // Phase 2: wait for it to exit.
        loop {
            if !rx.borrow().running {
                break;
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
        // Give the game a moment to release file handles.
        tokio::time::sleep(Duration::from_secs(3)).await;
        let still_ours = current().is_some_and(|s| s.map_id == map_id && s.offline);
        if still_ours && config::current(&app).restore_maps_on_game_exit {
            if let Err(err) = deactivate(&app).await {
                tracing::error!(%err, "offline session: automatic restore failed");
            }
        }
    });
}

/// After a game update / verify: our `mods/` files are independent from
/// stock files, so only a missing file needs work (re-install it).
pub fn reapply(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    let Some(session) = current() else {
        return Ok(ReapplyOutcome::NothingToDo);
    };
    let audit = writer::audit(app)?;
    let broken: Vec<_> = audit
        .into_iter()
        .filter(|a| a.entry.owner == Owner::Maps && a.state != EntryState::Intact)
        .collect();
    if broken.is_empty() {
        return Ok(ReapplyOutcome::NothingToDo);
    }
    if library::get(&session.map_id).is_err() {
        let rels: Vec<String> = broken.iter().map(|a| a.entry.rel_path.clone()).collect();
        writer::forget(Owner::Maps, &rels)?;
        set(None)?;
        return Ok(ReapplyOutcome::NeedsUser(format!(
            "map {} is no longer in the library",
            session.map_name
        )));
    }
    let rels: Vec<String> = broken.iter().map(|a| a.entry.rel_path.clone()).collect();
    writer::forget(Owner::Maps, &rels)?;
    activate_blocking(
        app,
        &session.map_id,
        Some(session.target.clone()),
        session.offline,
    )?;
    Ok(ReapplyOutcome::Reapplied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rel: &str, tag: &str, placement: Placement) -> ManifestEntry {
        ManifestEntry {
            owner: Owner::Maps,
            rel_path: rel.into(),
            placement,
            original_sha256: None,
            written_sha256: String::new(),
            tag: tag.into(),
            build: String::new(),
            written_at: Utc::now(),
        }
    }

    fn session(id: &str) -> MapSession {
        MapSession {
            map_id: id.into(),
            map_name: "m".into(),
            target: DEFAULT_TARGET.into(),
            activated_at: Utc::now(),
            offline: false,
        }
    }

    #[test]
    fn stale_entries() {
        let s = session("a");
        let root = entry(
            "TAGame/CookedPCConsole/Labs_Underpass_P.upk",
            "a",
            Placement::RootReplace,
        );
        let companion = entry(
            "TAGame/CookedPCConsole/mods/Extra.upk",
            "a",
            Placement::ModsOverride,
        );
        let legacy = entry(
            "TAGame/CookedPCConsole/mods/Labs_Utopia_P.upk",
            "a",
            Placement::ModsOverride,
        );
        let other = entry(
            "TAGame/CookedPCConsole/Labs_Utopia_P.upk",
            "b",
            Placement::RootReplace,
        );
        assert!(!is_stale(&root, Some(&s)));
        assert!(!is_stale(&companion, Some(&s)));
        assert!(
            is_stale(&legacy, Some(&s)),
            "arenas in mods/ move over the stock file"
        );
        assert!(is_stale(&other, Some(&s)));
        assert!(is_stale(&root, None));
    }
}
