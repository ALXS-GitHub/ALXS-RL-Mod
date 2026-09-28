//! Cooked package engine (`.upk`): summary, AES header tables, names,
//! import/export tables, RL chunked-zlib bodies, tagged properties,
//! textures, material parameters, renames and thumbnails.
//!
//! Format notes live in `docs_rl/` (palette.md, custom_decals*.md).
//! Everything here is pure data manipulation; writing into the game install
//! is the job of `game::writer`.

pub mod chunks;
pub mod commands;
pub mod crypto;
pub mod error;
pub mod keys;
pub mod material;
pub mod names;
pub mod package;
pub mod props;
pub mod reader;
pub mod recolor;
pub mod rename;
pub mod summary;
pub mod tables;
pub mod texture;
pub mod thumbs;

pub use error::{UpkError, UpkResult};
pub use keys::KeyRing;
pub use package::{Body, Package};
pub use rename::{can_decrypt, rename_package, Rename};

use tauri::AppHandle;

use crate::base::AppResult;

pub fn init(app: &AppHandle) -> AppResult<()> {
    let ring = keys::ring(app);
    if ring.is_empty() {
        tracing::warn!("no AES keys (keys.txt) — item swaps, thumbnails and decals are disabled");
    } else {
        tracing::info!(keys = ring.len(), "package engine ready");
    }
    Ok(())
}
