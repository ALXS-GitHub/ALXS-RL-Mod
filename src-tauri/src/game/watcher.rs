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

/// Easy Anti-Cheat's client service: it runs exactly while an EAC-protected
/// game runs.
const EAC_SERVICE: &str = "EasyAntiCheat_EOS.exe";

/// Game state from the running processes (`(pid, image name)`).
///
/// With EAC, `RocketLeague_EAC.exe` is only a bootstrap: it starts a
/// protected `RocketLeague.exe` and exits, so the image name alone says
/// nothing. EAC is detected from the bootstrap or from EAC's service.
pub(crate) fn classify<'a>(processes: impl IntoIterator<Item = (u32, &'a str)>) -> RunningState {
    let mut state = RunningState::default();
    let mut bootstrap = None;
    let mut eac_service = false;
    for (pid, name) in processes {
        if name.eq_ignore_ascii_case(EXE_NO_EAC) {
            state.running = true;
            state.pid = Some(pid);
        } else if name.eq_ignore_ascii_case(EXE_EAC) {
            state.running = true;
            bootstrap = Some(pid);
        } else if name.eq_ignore_ascii_case(EAC_SERVICE) {
            eac_service = true;
        }
    }
    state.with_eac = state.running && (bootstrap.is_some() || eac_service);
    state.pid = state.pid.or(bootstrap);
    state
}

fn probe(sys: &mut System) -> RunningState {
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::new());
    let names: Vec<(u32, String)> = sys
        .processes()
        .iter()
        .map(|(pid, p)| (pid.as_u32(), p.name().to_string_lossy().into_owned()))
        .collect();
    let mut state = classify(names.iter().map(|(pid, n)| (*pid, n.as_str())));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eac_is_detected_after_the_bootstrap_exits() {
        // Protected game + EAC service, bootstrap gone.
        let s = classify([(10, "RocketLeague.exe"), (3, "EasyAntiCheat_EOS.exe")]);
        assert!(s.running && s.with_eac);
        assert_eq!(s.pid, Some(10));
        // Bootstrap still there: the game pid wins.
        let s = classify([(2, "RocketLeague_EAC.exe"), (10, "RocketLeague.exe")]);
        assert!(s.with_eac);
        assert_eq!(s.pid, Some(10));
    }

    #[test]
    fn no_eac_and_not_running() {
        let s = classify([(10, "rocketleague.exe"), (4, "Launcher.exe")]);
        assert!(s.running && !s.with_eac);
        // EAC's service alone (another game) is not Rocket League.
        let s = classify([(3, "EasyAntiCheat_EOS.exe")]);
        assert!(!s.running && !s.with_eac);
    }
}
