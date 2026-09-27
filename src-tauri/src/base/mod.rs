//! Cross-cutting building blocks: errors, paths, config, filesystem helpers,
//! network policy, logging and app-level commands.

pub mod app;
pub mod config;
pub mod error;
pub mod fsx;
pub mod logging;
pub mod logs;
pub mod patch;
pub mod paths;
pub mod security;

pub use error::{AppError, AppResult};
