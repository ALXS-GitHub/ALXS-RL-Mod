//! Modded multiplayer: custom-map games between friends, offline (no EAC),
//! hosted and joined with BakkesMod's Rocket Plugin over a Tailscale tailnet.
//!
//! The app only orchestrates tools the user installed: it reads Tailscale's
//! status, installs the map through `maps`, starts the no-EAC game and
//! BakkesMod's own injector, and sends Rocket Plugin commands through
//! BakkesMod's local RCON. It never injects anything itself and never
//! touches the game's network traffic. See `docs/MULTIPLAYER.md`.

pub mod bakkes_cfg;
pub mod commands;
pub mod invite;
pub mod rcon;
pub mod rp_options;
pub mod session;
pub mod tailscale;

use tauri::AppHandle;

use crate::base::error::AppResult;

pub fn init(_app: &AppHandle) -> AppResult<()> {
    Ok(())
}
