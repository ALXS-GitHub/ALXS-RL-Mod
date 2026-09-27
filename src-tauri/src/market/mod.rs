//! Marketplace: community decal and ball packs from online sources —
//! AlphaConsole's library (alphaconsole.io) and RL-Designer's GitHub repo —
//! installed into the app's own libraries, converted to real-colour packs
//! on request. Maps have their own sources (`maps::sources`). Network only
//! on explicit user action, through `base::security` (`Purpose::Market`).

pub mod alphaconsole;
pub mod commands;
pub mod install;
pub mod model;
pub mod rl_designer;
