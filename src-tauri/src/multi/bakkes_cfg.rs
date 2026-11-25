//! BakkesMod setup for the modded multiplayer page: lets this app drive
//! Rocket Plugin through BakkesMod's local RCON (websocket).
//!
//! Two small, reversible edits in `%APPDATA%\bakkesmod\bakkesmod`:
//! - `cfg/autoexec.cfg` (run by BakkesMod at load, never rewritten by it):
//!   a marked block loading BakkesMod's RCON plugin (`RCONPlugin.dll`, not
//!   loaded by default: its cvars do not exist until it is) and turning it
//!   on with a random password;
//! - `data/rcon_commands.cfg` (RCON whitelist, first word of a command):
//!   the `rp` line (Rocket Plugin's console command).
//!
//! [`undo`] removes both. Nothing is written in the game install.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};

pub const BLOCK_BEGIN: &str =
    "// >>> ALXS-RL-Mod multiplayer (RCON for Rocket Plugin) - delete this block to undo";
pub const BLOCK_END: &str = "// <<< ALXS-RL-Mod multiplayer";
/// Rocket Plugin's console command (`rp host …`, `rp join …`).
pub const WHITELIST_COMMAND: &str = "rp";
pub const DEFAULT_RCON_PORT: u16 = 9002;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BakkesSetup {
    /// `%APPDATA%\bakkesmod\bakkesmod` exists.
    pub data_dir: Option<String>,
    pub injector_path: Option<String>,
    /// `plugins/RocketPlugin.dll` present.
    pub rocket_plugin_installed: bool,
    /// `plugin load rocketplugin` in `cfg/plugins.cfg`.
    pub rocket_plugin_loaded: bool,
    /// Our RCON block and the `rp` whitelist line are both in place.
    pub rcon_ready: bool,
    /// `plugins/RCONPlugin.dll` present (ships with BakkesMod).
    pub rcon_plugin_installed: bool,
    pub rcon_port: u16,
    /// BakkesMod's injector process is running.
    pub injector_running: bool,
}

/// What [`apply`] changed, so [`undo`] only removes our own lines.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupState {
    whitelist_added: bool,
}

fn state_file() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("multi")?.join("bakkes.json"))
}

pub fn data_dir() -> Option<PathBuf> {
    paths::roaming_dir()
        .map(|r| r.join("bakkesmod").join("bakkesmod"))
        .filter(|d| d.is_dir())
}

fn read_text(path: &Path) -> String {
    std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

/// Lines without their line ending, keeping the file's own ending style.
fn eol_of(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

// --- pure helpers (tested) --------------------------------------------------

/// The password of our block, if the block is there.
pub(crate) fn block_password(autoexec: &str) -> Option<String> {
    let mut inside = false;
    for line in autoexec.lines() {
        let l = line.trim();
        if l == BLOCK_BEGIN {
            inside = true;
        } else if l == BLOCK_END {
            inside = false;
        } else if inside {
            if let Some(v) = cvar_value(l, "rcon_password") {
                return Some(v);
            }
        }
    }
    None
}

/// Value of `name "value" //comment` (or `name value`) on one line.
pub(crate) fn cvar_value(line: &str, name: &str) -> Option<String> {
    let l = line.trim();
    let rest = l.strip_prefix(name)?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim_start();
    if let Some(quoted) = rest.strip_prefix('"') {
        quoted.split('"').next().map(str::to_string)
    } else {
        rest.split_whitespace().next().map(str::to_string)
    }
}

/// `autoexec` without our block.
pub(crate) fn remove_block(autoexec: &str) -> String {
    let eol = eol_of(autoexec);
    let mut out: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in autoexec.lines() {
        let l = line.trim();
        if l == BLOCK_BEGIN {
            inside = true;
            continue;
        }
        if inside {
            if l == BLOCK_END {
                inside = false;
            }
            continue;
        }
        out.push(line);
    }
    let mut text = out.join(eol);
    if !text.is_empty() {
        text.push_str(eol);
    }
    text
}

/// `autoexec` with our block (replacing any previous one) at the end.
pub(crate) fn with_block(autoexec: &str, password: &str) -> String {
    let eol = eol_of(autoexec);
    let mut text = remove_block(autoexec);
    text.push_str(BLOCK_BEGIN);
    text.push_str(eol);
    text.push_str("plugin load rconplugin");
    text.push_str(eol);
    text.push_str("rcon_enabled \"1\"");
    text.push_str(eol);
    text.push_str(&format!("rcon_password \"{password}\""));
    text.push_str(eol);
    text.push_str(BLOCK_END);
    text.push_str(eol);
    text
}

pub(crate) fn whitelist_has(whitelist: &str, command: &str) -> bool {
    whitelist.lines().any(|l| l.trim() == command)
}

pub(crate) fn whitelist_with(whitelist: &str, command: &str) -> String {
    if whitelist_has(whitelist, command) {
        return whitelist.to_string();
    }
    let eol = eol_of(whitelist);
    let mut text = whitelist.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push_str(eol);
    }
    text.push_str(command);
    text.push_str(eol);
    text
}

pub(crate) fn whitelist_without(whitelist: &str, command: &str) -> String {
    let eol = eol_of(whitelist);
    let kept: Vec<&str> = whitelist.lines().filter(|l| l.trim() != command).collect();
    let mut text = kept.join(eol);
    if !text.is_empty() {
        text.push_str(eol);
    }
    text
}

/// RCON port from BakkesMod's `config.cfg` (`rcon_port "9002"`).
pub(crate) fn rcon_port_in(config: &str) -> Option<u16> {
    config
        .lines()
        .find_map(|l| cvar_value(l, "rcon_port"))
        .and_then(|v| v.parse().ok())
        .filter(|p| *p > 0)
}

// --- disk -------------------------------------------------------------------

pub fn injector_path() -> Option<PathBuf> {
    crate::extras::bakkes::status()
        .injector_path
        .map(PathBuf::from)
        .filter(|p| p.is_file())
}

pub fn injector_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::new());
    sys.processes()
        .values()
        .any(|p| p.name().eq_ignore_ascii_case("BakkesMod.exe"))
}

pub fn rcon_port(dir: &Path) -> u16 {
    rcon_port_in(&read_text(&dir.join("cfg").join("config.cfg"))).unwrap_or(DEFAULT_RCON_PORT)
}

/// The RCON password we set, if our setup is complete.
pub fn rcon_password() -> Option<String> {
    let dir = data_dir()?;
    let autoexec = read_text(&dir.join("cfg").join("autoexec.cfg"));
    let whitelist = read_text(&dir.join("data").join("rcon_commands.cfg"));
    block_password(&autoexec).filter(|_| whitelist_has(&whitelist, WHITELIST_COMMAND))
}

pub fn setup() -> BakkesSetup {
    let dir = data_dir();
    let (installed, loaded, port) = match &dir {
        Some(d) => (
            d.join("plugins").join("RocketPlugin.dll").is_file(),
            read_text(&d.join("cfg").join("plugins.cfg"))
                .lines()
                .any(|l| l.trim().eq_ignore_ascii_case("plugin load rocketplugin")),
            rcon_port(d),
        ),
        None => (false, false, DEFAULT_RCON_PORT),
    };
    BakkesSetup {
        data_dir: dir.as_ref().map(|d| d.to_string_lossy().into_owned()),
        injector_path: injector_path().map(|p| p.to_string_lossy().into_owned()),
        rocket_plugin_installed: installed,
        rocket_plugin_loaded: loaded,
        rcon_ready: rcon_password().is_some(),
        rcon_plugin_installed: dir
            .as_ref()
            .is_some_and(|d| d.join("plugins").join("RCONPlugin.dll").is_file()),
        rcon_port: port,
        injector_running: injector_running(),
    }
}

fn require_dir() -> AppResult<PathBuf> {
    data_dir().ok_or_else(|| AppError::NotFound("BakkesMod folder (%APPDATA%\\bakkesmod)".into()))
}

/// Rocket Plugin's id on bakkesplugins.com (`/plugin/26`).
pub const ROCKET_PLUGIN_ID: u32 = 26;
pub const ROCKET_PLUGIN_PAGE: &str = "https://bakkesplugins.com/plugin/26";

/// The `bakkesmod://install/<id>` link bakkesplugins' "Install" button uses.
pub(crate) fn install_uri(plugin_id: u32) -> String {
    format!("bakkesmod://install/{plugin_id}")
}

/// Hands Rocket Plugin to BakkesMod's own plugin installer (the program
/// registered for `bakkesmod://` links), which downloads it and asks the
/// user to confirm. Nothing is downloaded by this app.
pub fn install_rocket_plugin() -> AppResult<()> {
    let installer = require_dir()?.join("plugininstaller.exe");
    if !installer.is_file() {
        return Err(AppError::FileNotFound(installer.display().to_string()));
    }
    let mut cmd = std::process::Command::new(&installer);
    if let Some(dir) = installer.parent() {
        cmd.current_dir(dir);
    }
    cmd.arg(install_uri(ROCKET_PLUGIN_ID))
        .spawn()
        .map_err(|e| AppError::Internal(format!("BakkesMod plugin installer: {e}")))?;
    tracing::info!("Rocket Plugin handed to BakkesMod's plugin installer");
    Ok(())
}

/// Turns RCON on (random password) and whitelists `rp`. Idempotent: an
/// existing block keeps its password. BakkesMod reads both files when it
/// loads, so a running BakkesMod only picks this up at the next game start.
pub fn apply() -> AppResult<String> {
    let dir = require_dir()?;
    let cfg_dir = dir.join("cfg");
    let autoexec_path = cfg_dir.join("autoexec.cfg");
    let whitelist_path = dir.join("data").join("rcon_commands.cfg");
    std::fs::create_dir_all(&cfg_dir)?;
    if let Some(parent) = whitelist_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let autoexec = read_text(&autoexec_path);
    let password =
        block_password(&autoexec).unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());
    let next = with_block(&autoexec, &password);
    if next != autoexec {
        fsx::write_atomic(&autoexec_path, next.as_bytes())?;
    }

    let whitelist = read_text(&whitelist_path);
    if !whitelist_has(&whitelist, WHITELIST_COMMAND) {
        fsx::write_atomic(
            &whitelist_path,
            whitelist_with(&whitelist, WHITELIST_COMMAND).as_bytes(),
        )?;
        let mut state: SetupState = fsx::read_json_or_default(&state_file()?)?;
        state.whitelist_added = true;
        fsx::write_json(&state_file()?, &state)?;
    }
    tracing::info!("BakkesMod RCON set up for Rocket Plugin");
    Ok(password)
}

/// Removes our block and the `rp` whitelist line (only if we added it).
pub fn undo() -> AppResult<()> {
    let dir = require_dir()?;
    let autoexec_path = dir.join("cfg").join("autoexec.cfg");
    let autoexec = read_text(&autoexec_path);
    let cleaned = remove_block(&autoexec);
    if cleaned != autoexec {
        fsx::write_atomic(&autoexec_path, cleaned.as_bytes())?;
    }
    let state: SetupState = fsx::read_json_or_default(&state_file()?)?;
    if state.whitelist_added {
        let whitelist_path = dir.join("data").join("rcon_commands.cfg");
        let whitelist = read_text(&whitelist_path);
        if whitelist_has(&whitelist, WHITELIST_COMMAND) {
            fsx::write_atomic(
                &whitelist_path,
                whitelist_without(&whitelist, WHITELIST_COMMAND).as_bytes(),
            )?;
        }
    }
    let path = state_file()?;
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    tracing::info!("BakkesMod RCON setup removed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_round_trip_keeps_user_lines() {
        let original = "bind F9 \"ballspeed\"\r\nsv_soccar_gamespeed 1\r\n";
        let with = with_block(original, "abc123");
        assert!(with.starts_with(original));
        assert!(with.contains("plugin load rconplugin\r\nrcon_enabled \"1\"\r\n"));
        assert_eq!(block_password(&with).as_deref(), Some("abc123"));
        // Re-applying replaces, never duplicates.
        let again = with_block(&with, "zzz");
        assert_eq!(again.matches(BLOCK_BEGIN).count(), 1);
        assert_eq!(block_password(&again).as_deref(), Some("zzz"));
        assert_eq!(remove_block(&again), original);
    }

    #[test]
    fn empty_autoexec() {
        let with = with_block("", "pw");
        assert!(with.starts_with(BLOCK_BEGIN));
        assert!(with.ends_with(&format!("{BLOCK_END}\n")));
        assert_eq!(remove_block(&with), "");
        // A user's own rcon_password outside the block is not ours.
        assert_eq!(block_password("rcon_password \"mine\"\n"), None);
    }

    #[test]
    fn cvar_values() {
        assert_eq!(
            cvar_value("rcon_port \"9002\" //", "rcon_port").as_deref(),
            Some("9002")
        );
        assert_eq!(
            cvar_value("rcon_port 9100", "rcon_port").as_deref(),
            Some("9100")
        );
        assert_eq!(cvar_value("rcon_portable \"1\"", "rcon_port"), None);
        assert_eq!(
            rcon_port_in("mp_port \"7777\"\nrcon_port \"9003\" //\n"),
            Some(9003)
        );
        assert_eq!(rcon_port_in("rcon_port \"0\""), None);
    }

    #[test]
    fn rocket_plugin_install_link() {
        assert_eq!(install_uri(ROCKET_PLUGIN_ID), "bakkesmod://install/26");
        assert!(ROCKET_PLUGIN_PAGE.ends_with(&ROCKET_PLUGIN_ID.to_string()));
    }

    #[test]
    fn whitelist_edits() {
        let wl = "ws_inventory\r\nsendback\r\nrcon_password\r\nplugin";
        assert!(!whitelist_has(wl, "rp"));
        let added = whitelist_with(wl, "rp");
        assert_eq!(
            added,
            "ws_inventory\r\nsendback\r\nrcon_password\r\nplugin\r\nrp\r\n"
        );
        assert_eq!(whitelist_with(&added, "rp"), added);
        assert!(whitelist_has(&added, "rp"));
        assert_eq!(
            whitelist_without(&added, "rp"),
            "ws_inventory\r\nsendback\r\nrcon_password\r\nplugin\r\n"
        );
        // "rp_something" is another command.
        assert!(!whitelist_has("rp_enable_debug_mode\n", "rp"));
    }
}
