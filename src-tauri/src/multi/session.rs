//! One modded multiplayer session at a time: host or join a Rocket Plugin
//! game over Tailscale.
//!
//! Steps (run in the background, progress on `multi://session`):
//! 1. resolve the map (join: from the library, else downloaded from
//!    bakkesplugins) and install it over the Labs arena (`maps::session`);
//! 2. set up BakkesMod's RCON if the game is not running yet;
//! 3. start the game without EAC, then BakkesMod's injector (the app never
//!    injects anything itself);
//! 4. wait for BakkesMod's RCON and send `rp host <arena>` / `rp join <ip> <port>`.
//!
//! If the RCON never answers, the session ends in `Manual`: the UI shows the
//! command to type in the BakkesMod console instead.

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::base::error::{AppError, AppResult};
use crate::extras::launch;
use crate::game::writer::{self, Owner, Placement};
use crate::game::{install, watcher};
use crate::maps::model::{MapEntry, MapOrigin, RemoteSource};
use crate::maps::{library, session as maps_session, sources};
use crate::multi::invite::{self, Invite, InviteMap};
use crate::multi::tailscale::{self, TailscaleState};
use crate::multi::{bakkes_cfg, rcon};

pub const EVENT_SESSION: &str = "multi://session";

/// How long the game + BakkesMod get to bring RCON up.
const RCON_WAIT: Duration = Duration::from_secs(240);
const RCON_POLL: Duration = Duration::from_secs(3);
/// After RCON answers, BakkesMod may still be loading its plugins.
const MENU_GRACE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Host,
    Join,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    DownloadingMap,
    InstallingMap,
    LaunchingGame,
    StartingBakkes,
    WaitingBakkes,
    /// The Rocket Plugin command was sent: the game should be loading.
    CommandSent,
    /// Automation stopped: the user types `command` in the BakkesMod console.
    Manual,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiSession {
    pub id: String,
    pub role: Role,
    pub phase: Phase,
    pub map_id: Option<String>,
    pub map_name: String,
    pub target: String,
    /// `ip:port` players connect to.
    pub address: Option<String>,
    /// Host only: the code to send to friends.
    pub invite_code: Option<String>,
    /// The Rocket Plugin console command (also the manual fallback).
    pub command: Option<String>,
    /// Join only: the installed map is not the host's exact file.
    pub map_mismatch: bool,
    /// Why the session is `Manual` / `Failed`.
    pub message: Option<String>,
    pub started_at: DateTime<Utc>,
}

static SESSION: RwLock<Option<MultiSession>> = RwLock::new(None);
static TASK: Mutex<Option<tauri::async_runtime::JoinHandle<()>>> = Mutex::new(None);

pub fn current() -> Option<MultiSession> {
    SESSION.read().ok().and_then(|s| s.clone())
}

fn publish(app: &AppHandle) {
    let _ = app.emit(EVENT_SESSION, current());
}

/// Applies `f` if session `id` is still the current one.
fn update(app: &AppHandle, id: &str, f: impl FnOnce(&mut MultiSession)) -> bool {
    let changed = match SESSION.write() {
        Ok(mut guard) => match guard.as_mut() {
            Some(s) if s.id == id => {
                f(s);
                true
            }
            _ => false,
        },
        Err(_) => false,
    };
    if changed {
        publish(app);
    }
    changed
}

fn set_phase(app: &AppHandle, id: &str, phase: Phase) {
    update(app, id, |s| s.phase = phase);
}

fn end(app: &AppHandle, id: &str, phase: Phase, message: impl Into<String>) {
    let message = message.into();
    tracing::warn!(?phase, %message, "multiplayer session stopped");
    update(app, id, |s| {
        s.phase = phase;
        s.message = Some(message);
    });
}

/// Stops the background steps (the game, BakkesMod and the map stay as they are).
pub fn cancel(app: &AppHandle) {
    if let Ok(mut task) = TASK.lock() {
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }
    if let Ok(mut guard) = SESSION.write() {
        *guard = None;
    }
    publish(app);
}

/// Checks shared by host and join, before anything is written.
fn preflight(need_tailscale: bool) -> AppResult<tailscale::TailscaleStatus> {
    let state = watcher::probe_now();
    if state.running && state.with_eac {
        return Err(AppError::Conflict(
            "Rocket League is running with Easy Anti-Cheat — close it first (modded games need the no-EAC mode)".into(),
        ));
    }
    let setup = bakkes_cfg::setup();
    if setup.data_dir.is_none() {
        return Err(AppError::NotFound("BakkesMod is not installed".into()));
    }
    if !setup.rocket_plugin_installed {
        return Err(AppError::NotFound(
            "Rocket Plugin is not installed in BakkesMod".into(),
        ));
    }
    let ts = tailscale::status();
    if need_tailscale && ts.state != TailscaleState::Running {
        return Err(AppError::Conflict(
            "Tailscale is not connected on this PC".into(),
        ));
    }
    Ok(ts)
}

fn start(
    app: &AppHandle,
    session: MultiSession,
    work: impl std::future::Future<Output = ()> + Send + 'static,
) {
    cancel(app);
    if let Ok(mut guard) = SESSION.write() {
        *guard = Some(session);
    }
    publish(app);
    let handle = tauri::async_runtime::spawn(work);
    if let Ok(mut task) = TASK.lock() {
        *task = Some(handle);
    }
}

fn new_session(role: Role, map_name: String, target: String) -> MultiSession {
    MultiSession {
        id: uuid::Uuid::new_v4().to_string(),
        role,
        phase: Phase::InstallingMap,
        map_id: None,
        map_name,
        target,
        address: None,
        invite_code: None,
        command: None,
        map_mismatch: false,
        message: None,
        started_at: Utc::now(),
    }
}

// --- host -------------------------------------------------------------------

pub fn host(app: &AppHandle, map_id: String) -> AppResult<MultiSession> {
    let ts = preflight(true)?;
    let ip: Ipv4Addr = ts
        .self_ipv4
        .as_deref()
        .and_then(|ip| ip.parse().ok())
        .ok_or_else(|| AppError::Conflict("no Tailscale IPv4 address on this PC".into()))?;
    let entry = library::get(&map_id)?;
    let target = entry
        .preferred_target
        .clone()
        .unwrap_or_else(|| crate::maps::model::DEFAULT_TARGET.to_string());
    let mut session = new_session(Role::Host, entry.name.clone(), target);
    session.map_id = Some(map_id.clone());
    session.address = Some(format!("{ip}:{}", invite::DEFAULT_PORT));
    let id = session.id.clone();
    let returned = session.clone();
    let app2 = app.clone();
    let host_name = ts.self_name.clone();
    start(app, session, async move {
        let Some(target) = install_map(&app2, &id, &map_id, None).await else {
            return;
        };
        let code = invite::encode(&Invite {
            v: invite::VERSION,
            host: ip,
            port: invite::DEFAULT_PORT,
            host_name,
            map: InviteMap {
                name: entry.name.clone(),
                target: target.clone(),
                sha256: installed_sha(&map_id),
                source: remote_of(&entry).map(|r| r.0),
                remote_id: remote_of(&entry).map(|r| r.1),
            },
        });
        let command = invite::host_command(&target);
        update(&app2, &id, |s| {
            s.invite_code = code.ok();
            s.command = Some(command.clone());
            s.target = target.clone();
        });
        drive(app2, id, map_id, command).await;
    });
    Ok(returned)
}

// --- join -------------------------------------------------------------------

pub fn join(app: &AppHandle, code: &str) -> AppResult<MultiSession> {
    let invite = invite::decode(code)?;
    // A tailnet address needs Tailscale here too; a LAN address does not.
    preflight(invite::is_tailnet(invite.host))?;
    let mut session = new_session(
        Role::Join,
        invite.map.name.clone(),
        invite.map.target.clone(),
    );
    session.address = Some(format!("{}:{}", invite.host, invite.port));
    let command = invite::join_command(invite.host, invite.port);
    session.command = Some(command.clone());
    let id = session.id.clone();
    let returned = session.clone();
    let app2 = app.clone();
    start(app, session, async move {
        let entry = match find_or_download(&app2, &id, &invite.map).await {
            Ok(e) => e,
            Err(err) => return end(&app2, &id, Phase::Failed, err.to_string()),
        };
        update(&app2, &id, |s| s.map_id = Some(entry.id.clone()));
        if install_map(&app2, &id, &entry.id, Some(invite.map.target.clone()))
            .await
            .is_none()
        {
            return;
        }
        let mismatch = match (&invite.map.sha256, installed_sha(&entry.id)) {
            (Some(theirs), Some(ours)) => !theirs.eq_ignore_ascii_case(&ours),
            _ => false,
        };
        update(&app2, &id, |s| s.map_mismatch = mismatch);
        drive(app2, id, entry.id, command).await;
    });
    Ok(returned)
}

fn remote_of(entry: &MapEntry) -> Option<(RemoteSource, String)> {
    match &entry.origin {
        MapOrigin::Remote {
            source, remote_id, ..
        } => Some((*source, remote_id.clone())),
        _ => None,
    }
}

/// Library match by source id, then by name (hashing every map would read
/// hundreds of MB; the installed file's hash is compared afterwards).
/// Downloads from bakkesplugins when missing.
async fn find_or_download(app: &AppHandle, id: &str, map: &InviteMap) -> AppResult<MapEntry> {
    let maps = tauri::async_runtime::spawn_blocking(library::list)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
    let by_remote = |e: &&MapEntry| match (&map.source, &map.remote_id, remote_of(e)) {
        (Some(src), Some(rid), Some((s, r))) => *src == s && *rid == r,
        _ => false,
    };
    if let Some(e) = maps.iter().find(by_remote) {
        return Ok(e.clone());
    }
    if let Some(e) = maps
        .iter()
        .find(|e| e.name.trim().eq_ignore_ascii_case(map.name.trim()))
    {
        return Ok(e.clone());
    }
    match (&map.source, &map.remote_id) {
        (Some(RemoteSource::BakkesPlugins), Some(rid)) => {
            set_phase(app, id, Phase::DownloadingMap);
            sources::download(app, RemoteSource::BakkesPlugins, rid).await
        }
        _ => Err(AppError::NotFound(format!(
            "map \"{}\" is not in your library — ask the host for the file and import it in Maps",
            map.name
        ))),
    }
}

/// SHA-256 of the map package the write layer installed for `map_id`.
fn installed_sha(map_id: &str) -> Option<String> {
    writer::entries_for(Owner::Maps)
        .ok()?
        .into_iter()
        .find(|e| e.tag == map_id && e.placement == Placement::RootReplace)
        .map(|e| e.written_sha256)
}

/// Installs the map (unless the same map is already on that arena).
/// Returns the arena it replaces, or `None` after ending the session.
async fn install_map(
    app: &AppHandle,
    id: &str,
    map_id: &str,
    target: Option<String>,
) -> Option<String> {
    set_phase(app, id, Phase::InstallingMap);
    if let Some(s) = maps_session::current() {
        let same_target = target
            .as_deref()
            .is_none_or(|t| t.eq_ignore_ascii_case(&s.target));
        if s.map_id == map_id && same_target {
            return Some(s.target);
        }
    }
    match maps_session::activate(app, map_id, target, true).await {
        Ok(s) => Some(s.target),
        Err(err) => {
            end(app, id, Phase::Failed, err.to_string());
            None
        }
    }
}

fn spawn_injector(path: &PathBuf) -> AppResult<()> {
    let mut cmd = std::process::Command::new(path);
    if let Some(dir) = path.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn()
        .map_err(|e| AppError::Internal(format!("could not start BakkesMod: {e}")))?;
    tracing::info!(exe = %path.display(), "BakkesMod started");
    Ok(())
}

/// Game → BakkesMod → RCON → command.
async fn drive(app: AppHandle, id: String, map_id: String, command: String) {
    let manual = |app: &AppHandle, msg: &str| end(app, &id, Phase::Manual, msg);

    // RCON can only be (re)configured before BakkesMod loads.
    let was_running = watcher::probe_now().running;
    let password = if was_running {
        bakkes_cfg::rcon_password()
    } else {
        match tauri::async_runtime::spawn_blocking(bakkes_cfg::apply).await {
            Ok(Ok(pw)) => Some(pw),
            Ok(Err(err)) => {
                tracing::warn!(%err, "could not set up BakkesMod RCON");
                None
            }
            Err(_) => None,
        }
    };

    if !was_running {
        set_phase(&app, &id, Phase::LaunchingGame);
        let launched = install::current_install(&app).and_then(|rl| launch::spawn_no_eac(&rl));
        if let Err(err) = launched {
            return end(&app, &id, Phase::Failed, err.to_string());
        }
        // Puts the stock arena back when the game exits (if the user wants that).
        maps_session::watch_offline_session(app.clone(), map_id);
    }

    set_phase(&app, &id, Phase::StartingBakkes);
    if !bakkes_cfg::injector_running() {
        match bakkes_cfg::injector_path() {
            Some(path) => {
                if let Err(err) = spawn_injector(&path) {
                    tracing::warn!(%err, "BakkesMod injector");
                }
            }
            None => tracing::warn!("BakkesMod injector not found"),
        }
    }

    let Some(password) = password else {
        return manual(
            &app,
            "The game was already running before the app set up BakkesMod's remote control — type the command in the BakkesMod console (F6), or restart the game",
        );
    };
    let port = bakkes_cfg::data_dir()
        .map(|d| bakkes_cfg::rcon_port(&d))
        .unwrap_or(bakkes_cfg::DEFAULT_RCON_PORT);

    set_phase(&app, &id, Phase::WaitingBakkes);
    let deadline = tokio::time::Instant::now() + RCON_WAIT;
    let mut seen_game = was_running;
    loop {
        let state = watcher::probe_now();
        seen_game |= state.running;
        if seen_game && !state.running {
            return end(&app, &id, Phase::Failed, "Rocket League was closed");
        }
        if state.running && rcon::reachable(port).await {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return manual(
                &app,
                "BakkesMod did not answer — check it is injected (green in its window), then type the command in its console (F6)",
            );
        }
        tokio::time::sleep(RCON_POLL).await;
    }
    tokio::time::sleep(MENU_GRACE).await;
    match rcon::send(port, &password, &[command]).await {
        Ok(()) => set_phase(&app, &id, Phase::CommandSent),
        Err(err) => manual(&app, &err.to_string()),
    }
}
