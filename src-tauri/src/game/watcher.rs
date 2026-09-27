//! Background watcher: is Rocket League running, with or without EAC, and is
//! it the foreground window?
//!
//! Emits `game://status` to the front-end on every change (the FX layer
//! pauses while the game has focus) and publishes the same state on a tokio
//! `watch` channel for backend subscribers (map sessions restore on exit).

use std::sync::OnceLock;
use std::time::Duration;

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};
use tokio::sync::watch;

use crate::game::install::{EXE_EAC, EXE_NO_EAC};

pub const EVENT_GAME_STATUS: &str = "game://status";
const POLL_INTERVAL: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunningState {
    pub running: bool,
    pub with_eac: bool,
    pub pid: Option<u32>,
    /// The game window currently has focus.
    pub foreground: bool,
}

static CHANNEL: OnceLock<(watch::Sender<RunningState>, watch::Receiver<RunningState>)> =
    OnceLock::new();

fn channel() -> &'static (watch::Sender<RunningState>, watch::Receiver<RunningState>) {
    CHANNEL.get_or_init(|| watch::channel(RunningState::default()))
}

/// Latest known state (non-blocking).
pub fn latest() -> RunningState {
    *channel().1.borrow()
}

/// Backend subscription (e.g. restore a map when the game exits).
pub fn subscribe() -> watch::Receiver<RunningState> {
    channel().1.clone()
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut sys = System::new();
        loop {
            let state = probe(&mut sys);
            let tx = &channel().0;
            if *tx.borrow() != state {
                tracing::debug!(?state, "game state changed");
                let _ = tx.send(state);
                let _ = app.emit(EVENT_GAME_STATUS, state);
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

/// One synchronous probe — also used by commands that must not race the
/// watcher (e.g. refusing to write game files while the game runs).
pub fn probe_now() -> RunningState {
    let mut sys = System::new();
    probe(&mut sys)
}

fn probe(sys: &mut System) -> RunningState {
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::new());
    let mut state = RunningState::default();
    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy();
        if name.eq_ignore_ascii_case(EXE_EAC) || name.eq_ignore_ascii_case(EXE_NO_EAC) {
            state.running = true;
            state.with_eac |= name.eq_ignore_ascii_case(EXE_EAC);
            state.pid = Some(pid.as_u32());
        }
    }
    if let Some(pid) = state.pid {
        state.foreground = foreground_pid() == Some(pid);
    }
    state
}

#[cfg(windows)]
fn foreground_pid() -> Option<u32> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };
    // SAFETY: plain Win32 queries with no pointers retained; a null window
    // handle is handled by the pid check below.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        (pid != 0).then_some(pid)
    }
}

#[cfg(not(windows))]
fn foreground_pid() -> Option<u32> {
    None
}
