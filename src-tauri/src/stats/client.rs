//! Stats API WebSocket client.
//!
//! Connects to `ws://127.0.0.1:<WebPort>` only while the game runs, feeds
//! the session tracker, persists the session and emits:
//! - `tracker://session` — the whole `SessionState` (≤ 4 per second while
//!   a match is live, immediately on results / connection changes),
//! - `stats://event` — `{ event, data }` for every non-`UpdateState` message
//!   and at most one `UpdateState` per second (debug / timeline feed).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio_tungstenite::tungstenite::Message;

use crate::base::{config, fsx, paths};
use crate::game::watcher;
use crate::stats::ini;
use crate::stats::protocol::{self, StatsMessage};
use crate::stats::tracker::{Change, Connection, MatchRecord, SessionState, Tracker};

pub const EVENT_SESSION: &str = "tracker://session";
pub const EVENT_RAW: &str = "stats://event";
const LIVE_EMIT_INTERVAL: Duration = Duration::from_millis(250);
const RAW_UPDATE_INTERVAL: Duration = Duration::from_secs(1);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
/// tracker.gg needs a moment to register a finished match.
/// MMR lookups after a match: tracker.gg takes a few minutes to pick a
/// match up, so a single early lookup can keep a stale rating until the
/// next match.
const MMR_REFRESH_DELAYS: [Duration; 2] = [Duration::from_secs(60), Duration::from_secs(240)];

pub type Shared = Arc<Mutex<Tracker>>;

#[derive(Serialize, Clone)]
struct RawEvent<'a> {
    event: &'a str,
    data: &'a serde_json::Value,
}

fn session_path() -> Option<std::path::PathBuf> {
    paths::data_subdir("stats")
        .ok()
        .map(|d| d.join("session.json"))
}

pub fn load_session() -> SessionState {
    let mut s: SessionState = session_path()
        .and_then(|p| fsx::read_json_or_default(&p).ok())
        .unwrap_or_default();
    s.live = None;
    s.connection = Connection::Idle;
    s
}

pub fn persist(state: &SessionState) {
    if let Some(p) = session_path() {
        if let Err(err) = fsx::write_json(&p, state) {
            tracing::warn!(%err, "could not persist tracker session");
        }
    }
}

fn append_history(rec: &MatchRecord) {
    let Ok(dir) = paths::data_subdir("stats") else {
        return;
    };
    let Ok(line) = serde_json::to_string(rec) else {
        return;
    };
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("history.jsonl"))
    {
        let _ = writeln!(f, "{line}");
    }
}

pub fn emit_session(app: &AppHandle, shared: &Shared) {
    let snapshot = shared.lock().map(|t| t.state.clone());
    if let Ok(state) = snapshot {
        let _ = app.emit(EVENT_SESSION, state);
    }
}

fn set_connection(app: &AppHandle, shared: &Shared, c: Connection) {
    let changed = shared
        .lock()
        .map(|mut t| {
            let changed = t.state.connection != c;
            t.state.connection = c;
            if c == Connection::Idle {
                t.state.live = None;
            }
            changed
        })
        .unwrap_or(false);
    if changed {
        emit_session(app, shared);
    }
}

/// Fetches MMR (if a tracker profile is configured) and merges it.
pub fn schedule_mmr_refresh(app: &AppHandle, shared: &Shared, delay: Duration) {
    let cfg = config::current(app).tracker;
    if !cfg.online_ratings {
        return;
    }
    let (Some(platform), Some(player)) = (cfg.platform, cfg.player_id) else {
        return;
    };
    let (app, shared) = (app.clone(), shared.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        match crate::stats::mmr::lookup(&app, &platform, &player).await {
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
                if let Ok(mut t) = shared.lock() {
                    t.merge_mmr(&lines);
                    persist(&t.state);
                }
                emit_session(&app, &shared);
            }
            Err(err) => tracing::debug!(%err, "automatic MMR refresh skipped"),
        }
    });
}

/// Lookups already done this app run (players met again in a rematch or a
/// requeue are not looked up twice within the TTL).
fn lobby_cache() -> &'static Mutex<HashMap<String, (Instant, crate::stats::lobby::LobbyRating)>> {
    static C: std::sync::OnceLock<
        Mutex<HashMap<String, (Instant, crate::stats::lobby::LobbyRating)>>,
    > = std::sync::OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}
const LOBBY_TTL: Duration = Duration::from_secs(15 * 60);

/// Looks up the MMR of new lobby players, one after the other (the hidden
/// browser is shared), emitting the session after each result.
fn schedule_lobby_lookups(
    app: &AppHandle,
    shared: &Shared,
    entries: Vec<crate::stats::lobby::LobbyRating>,
    team_size: usize,
) {
    use crate::stats::lobby::{rating_for, tracker_target, LobbyStatus};
    if entries.is_empty() {
        return;
    }
    let (app, shared) = (app.clone(), shared.clone());
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
                        match crate::stats::mmr::lookup(&app, platform, &id).await {
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
            if let Ok(mut t) = shared.lock() {
                t.set_lobby_rating(result);
            }
            emit_session(&app, &shared);
        }
    });
}

pub fn spawn(app: AppHandle, shared: Shared) {
    tauri::async_runtime::spawn(async move {
        let mut failures: u32 = 0;
        let mut mmr_fetched_this_run = false;
        loop {
            // Nothing to connect to: game closed, or Stats API turned off.
            let settings = ini::current().ok().map(|(_, s)| s);
            let enabled = settings.as_ref().is_some_and(|s| s.enabled());
            if !watcher::latest().running || !enabled {
                set_connection(&app, &shared, Connection::Idle);
                failures = 0;
                mmr_fetched_this_run = false;
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            let port = settings
                .and_then(|s| s.web_port)
                .filter(|p| *p > 0)
                .unwrap_or(ini::DEFAULT_WEB_PORT);
            set_connection(
                &app,
                &shared,
                if failures >= 3 {
                    Connection::Unreachable
                } else {
                    Connection::Connecting
                },
            );
            let url = format!("ws://127.0.0.1:{port}");
            match tokio::time::timeout(
                CONNECT_TIMEOUT,
                tokio_tungstenite::connect_async(url.as_str()),
            )
            .await
            {
                Ok(Ok((stream, _))) => {
                    failures = 0;
                    set_connection(&app, &shared, Connection::Connected);
                    tracing::info!(port, "stats API connected");
                    if !mmr_fetched_this_run {
                        mmr_fetched_this_run = true;
                        schedule_mmr_refresh(&app, &shared, Duration::from_secs(1));
                    }
                    read_loop(&app, &shared, stream).await;
                    tracing::info!("stats API disconnected");
                }
                _ => {
                    failures = failures.saturating_add(1);
                    if failures >= 3 {
                        set_connection(&app, &shared, Connection::Unreachable);
                    }
                }
            }
            let backoff = Duration::from_secs(u64::from(failures.clamp(1, 5)) * 2);
            tokio::time::sleep(backoff).await;
        }
    });
}

async fn read_loop<S>(
    app: &AppHandle,
    shared: &Shared,
    mut stream: tokio_tungstenite::WebSocketStream<S>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let long_ago = Instant::now()
        .checked_sub(RAW_UPDATE_INTERVAL)
        .unwrap_or_else(Instant::now);
    let mut last_live_emit = long_ago;
    let mut last_raw_update = long_ago;
    let mut pending_live = false;
    loop {
        // Flush a throttled live update even when the game goes quiet.
        let next = tokio::time::timeout(LIVE_EMIT_INTERVAL, stream.next()).await;
        let msg = match next {
            Err(_) => {
                if pending_live {
                    pending_live = false;
                    last_live_emit = Instant::now();
                    emit_session(app, shared);
                }
                if !watcher::latest().running {
                    return;
                }
                continue;
            }
            Ok(None) | Ok(Some(Err(_))) => return,
            Ok(Some(Ok(m))) => m,
        };
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => return,
            _ => continue,
        };
        let Some((parsed, data)) = protocol::parse(&text) else {
            continue;
        };
        let is_update = matches!(parsed, StatsMessage::UpdateState(_));
        if !is_update || last_raw_update.elapsed() >= RAW_UPDATE_INTERVAL {
            if is_update {
                last_raw_update = Instant::now();
            }
            let _ = app.emit(
                EVENT_RAW,
                RawEvent {
                    event: parsed.name(),
                    data: &data,
                },
            );
        }
        let mut lobby_todo = Vec::new();
        let mut team_size = 3;
        let change = match shared.lock() {
            Ok(mut t) => {
                t.hint = config::current(app).tracker.player_id;
                let change = t.apply(&parsed);
                if change == Change::Live {
                    lobby_todo = t.sync_lobby();
                    team_size = t.team_size();
                }
                if change == Change::MatchRecorded {
                    if let Some(rec) = t.state.matches.first() {
                        append_history(rec);
                    }
                    persist(&t.state);
                }
                change
            }
            Err(_) => Change::None,
        };
        schedule_lobby_lookups(app, shared, lobby_todo, team_size);
        match change {
            Change::MatchRecorded => {
                pending_live = false;
                last_live_emit = Instant::now();
                emit_session(app, shared);
                for delay in MMR_REFRESH_DELAYS {
                    schedule_mmr_refresh(app, shared, delay);
                }
            }
            Change::Live => {
                if last_live_emit.elapsed() >= LIVE_EMIT_INTERVAL {
                    last_live_emit = Instant::now();
                    pending_live = false;
                    emit_session(app, shared);
                } else {
                    pending_live = true;
                }
            }
            Change::None => {}
        }
    }
}
