//! Runs the tracker.gg lookups of the personal edition and publishes the
//! ratings. Driven by the match tracker through `stats::hooks`:
//! - game session start → your MMR,
//! - live match → the MMR of new players of the lobby,
//! - match recorded → your MMR again (tracker.gg lags a few minutes),
//! - session reset → the MMR delta restarts.
//!
//! State: `%LOCALAPPDATA%\ALXS-RL-Mod\stats\ratings.json`.
//! Event: `personal://ratings` (the whole [`RatingsState`]).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::base::{config, fsx, paths};
use crate::personal::lobby::{rating_for, tracker_target, LobbyRating, LobbyStatus};
use crate::personal::mmr;
use crate::personal::ratings::{team_size, MmrLine, RatingsState};
use crate::stats::tracker::PlayerLine;

pub const EVENT_RATINGS: &str = "personal://ratings";
/// tracker.gg takes a few minutes to pick a match up: a single early lookup
/// can keep a stale rating until the next match.
const MMR_REFRESH_DELAYS: [Duration; 2] = [Duration::from_secs(60), Duration::from_secs(240)];
/// Players met again in a rematch or a requeue are not looked up twice.
const LOBBY_TTL: Duration = Duration::from_secs(15 * 60);

/// Managed state.
pub struct RatingsShared(pub Arc<Mutex<RatingsState>>);

fn state_path() -> Option<std::path::PathBuf> {
    paths::data_subdir("stats")
        .ok()
        .map(|d| d.join("ratings.json"))
}

/// Before the split, the MMR lines lived in `session.json`: take them over once.
fn legacy_mmr() -> Vec<MmrLine> {
    let Some(path) = paths::data_subdir("stats")
        .ok()
        .map(|d| d.join("session.json"))
    else {
        return Vec::new();
    };
    fsx::read_json_or_default::<serde_json::Value>(&path)
        .ok()
        .and_then(|v| v.get("mmr").cloned())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn load() -> RatingsState {
    let path = state_path();
    let mut state: RatingsState = match path.as_ref().filter(|p| p.is_file()) {
        Some(p) => fsx::read_json_or_default(p).unwrap_or_default(),
        None => RatingsState {
            mmr: legacy_mmr(),
            ..Default::default()
        },
    };
    // The lobby belongs to a match that is over.
    state.lobby.clear();
    state
}

pub fn init(app: &AppHandle) {
    app.manage(RatingsShared(Arc::new(Mutex::new(load()))));
}

fn shared(app: &AppHandle) -> Option<Arc<Mutex<RatingsState>>> {
    app.try_state::<RatingsShared>().map(|s| Arc::clone(&s.0))
}

pub fn snapshot(app: &AppHandle) -> RatingsState {
    shared(app)
        .and_then(|s| s.lock().ok().map(|r| r.clone()))
        .unwrap_or_default()
}

fn persist(state: &RatingsState) {
    if let Some(p) = state_path() {
        if let Err(err) = fsx::write_json(&p, state) {
            tracing::warn!(%err, "could not persist ratings");
        }
    }
}

fn emit(app: &AppHandle) {
    let _ = app.emit(EVENT_RATINGS, snapshot(app));
}

/// Applies `f` to the state, then persists and emits it.
fn update(app: &AppHandle, f: impl FnOnce(&mut RatingsState)) {
    let Some(shared) = shared(app) else {
        return;
    };
    if let Ok(mut r) = shared.lock() {
        f(&mut r);
        persist(&r);
    }
    emit(app);
}

/// Fetches your MMR (if a tracker profile is configured) after `delay`.
fn schedule_mmr_refresh(app: &AppHandle, delay: Duration) {
    let cfg = config::current(app).tracker;
    if !cfg.online_ratings {
        return;
    }
    let (Some(platform), Some(player)) = (cfg.platform, cfg.player_id) else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        match mmr::lookup(&app, &platform, &player).await {
            Ok(profile) => {
                let lines: Vec<(String, i64, Option<String>)> = profile
                    .playlists
                    .into_iter()
                    .filter_map(|p| Some((p.playlist, p.rating?, p.tier_icon)))
                    .collect();
                tracing::debug!(
                    ratings = ?lines.iter().map(|(p, r, _)| format!("{p}={r}")).collect::<Vec<_>>(),
                    "MMR refreshed"
                );
                update(&app, |r| r.merge_mmr(&lines));
            }
            Err(err) => tracing::debug!(%err, "automatic MMR refresh skipped"),
        }
    });
}

type LobbyCache = Mutex<HashMap<String, (Instant, LobbyRating)>>;

fn lobby_cache() -> &'static LobbyCache {
    static C: OnceLock<LobbyCache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Looks up the MMR of new lobby players, one after the other (the hidden
/// browser is shared), emitting the ratings after each result.
fn schedule_lobby_lookups(app: &AppHandle, entries: Vec<LobbyRating>, team_size: usize) {
    if entries.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        for mut entry in entries {
            let cached = lobby_cache().lock().ok().and_then(|c| {
                c.get(&entry.key)
                    .filter(|(at, _)| at.elapsed() < LOBBY_TTL)
                    .map(|(_, r)| r.clone())
            });
            let result = match cached {
                Some(hit) => hit,
                None => {
                    if let Some((platform, id)) = tracker_target(&entry.key, &entry.name) {
                        match mmr::lookup(&app, platform, &id).await {
                            Ok(profile) => rating_for(&mut entry, &profile.playlists, team_size),
                            Err(err) => {
                                tracing::debug!(player = %entry.name, %err, "lobby MMR unavailable");
                                entry.status = LobbyStatus::Unavailable;
                            }
                        }
                    } else {
                        entry.status = LobbyStatus::Unavailable;
                    }
                    if let Ok(mut c) = lobby_cache().lock() {
                        c.insert(entry.key.clone(), (Instant::now(), entry.clone()));
                    }
                    entry
                }
            };
            update(&app, |r| r.set_lobby_rating(result));
        }
    });
}

// ── Hooks (called by `stats::hooks`) ─────────────────────────────────────

/// The Stats API connected for the first time since the game started.
pub fn on_game_session_start(app: &AppHandle) {
    schedule_mmr_refresh(app, Duration::from_secs(1));
}

/// A live update: keeps the lobby in line with the players of the match.
pub fn on_live(app: &AppHandle, players: &[PlayerLine]) {
    if !config::current(app).tracker.online_ratings {
        return;
    }
    let Some(shared) = shared(app) else {
        return;
    };
    let todo = match shared.lock() {
        Ok(mut r) => r.sync_lobby(players),
        Err(_) => return,
    };
    if !todo.is_empty() {
        emit(app);
        schedule_lobby_lookups(app, todo, team_size(players));
    }
}

/// A match was recorded: your MMR moved.
pub fn on_match_recorded(app: &AppHandle) {
    for delay in MMR_REFRESH_DELAYS {
        schedule_mmr_refresh(app, delay);
    }
}

/// New tracker session.
pub fn on_session_reset(app: &AppHandle) {
    update(app, RatingsState::reset_baselines);
}
