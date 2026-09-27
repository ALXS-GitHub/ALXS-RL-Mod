//! AES-256 key ring for cooked package header tables.
//!
//! Keys are the public community list (`keys.txt`, one base64 key per line).
//! They are never committed: the ring is loaded at runtime from, in order,
//! 1. `%LOCALAPPDATA%\ALXS-RL-Mod\keys.txt` (user drop-in, wins),
//! 2. the bundled resource `keys/keys.txt` (if a release ever ships one),
//! 3. `src-tauri/resources/keys/keys.txt` (dev builds on this machine).
//!
//! The ring is cached; [`reload`] re-reads it (e.g. after the user added the file).

use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

use base64::prelude::{Engine as _, BASE64_STANDARD};
use serde::Serialize;
use tauri::AppHandle;

use crate::base::{paths, AppError, AppResult};

pub type AesKey = [u8; 32];

#[derive(Debug, Default)]
pub struct KeyRing {
    keys: Vec<AesKey>,
    source: Option<PathBuf>,
}

impl KeyRing {
    pub fn from_text(text: &str, source: Option<PathBuf>) -> Self {
        Self {
            keys: parse_keys(text),
            source,
        }
    }

    pub fn from_keys(keys: Vec<AesKey>) -> Self {
        Self { keys, source: None }
    }

    pub fn keys(&self) -> &[AesKey] {
        &self.keys
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn source(&self) -> Option<&PathBuf> {
        self.source.as_ref()
    }
}

/// Parses one base64 key per line; blank lines, comments (`#`) and
/// malformed entries are ignored.
pub fn parse_keys(text: &str) -> Vec<AesKey> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| BASE64_STANDARD.decode(l).ok())
        .filter_map(|bytes| AesKey::try_from(bytes).ok())
        .collect()
}

fn candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(dir) = paths::data_dir() {
        out.push(dir.join("keys.txt"));
    }
    if let Ok(p) = paths::resource_path(app, "keys/keys.txt") {
        out.push(p);
    }
    out.push(PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    )));
    out
}

fn load(app: &AppHandle) -> KeyRing {
    for path in candidates(app) {
        if let Ok(text) = std::fs::read_to_string(&path) {
            let ring = KeyRing::from_text(&text, Some(path.clone()));
            if !ring.is_empty() {
                tracing::info!(keys = ring.len(), source = %path.display(), "AES key ring loaded");
                return ring;
            }
        }
    }
    // debug: `ring()` retries while empty, a warn here would spam the log.
    tracing::debug!("no AES keys found");
    KeyRing::default()
}

static RING: OnceLock<RwLock<Arc<KeyRing>>> = OnceLock::new();

/// Cached key ring (loaded on first use).
pub fn ring(app: &AppHandle) -> Arc<KeyRing> {
    let cell = RING.get_or_init(|| RwLock::new(Arc::new(load(app))));
    let current = cell.read().map(|g| Arc::clone(&g)).unwrap_or_default();
    if current.is_empty() {
        // The user may have dropped keys.txt since the last attempt.
        return reload(app);
    }
    current
}

/// Forces a re-read of the key files.
pub fn reload(app: &AppHandle) -> Arc<KeyRing> {
    let fresh = Arc::new(load(app));
    let cell = RING.get_or_init(|| RwLock::new(Arc::clone(&fresh)));
    if let Ok(mut guard) = cell.write() {
        *guard = Arc::clone(&fresh);
    }
    fresh
}

/// Copies a user-chosen key file to the app's drop-in location
/// (`%LOCALAPPDATA%\ALXS-RL-Mod\keys.txt`, read first) and reloads the
/// ring. Returns the number of valid keys; refuses a file with none.
pub fn import_file(app: &AppHandle, source: &std::path::Path) -> AppResult<usize> {
    let text = std::fs::read_to_string(source)?;
    let count = parse_keys(&text).len();
    if count == 0 {
        return Err(AppError::InvalidInput(format!(
            "{} holds no valid AES-256 key (one base64 key per line expected)",
            source.display()
        )));
    }
    crate::base::fsx::write_atomic(&paths::data_dir()?.join("keys.txt"), text.as_bytes())?;
    reload(app);
    tracing::info!(keys = count, "key file imported");
    Ok(count)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeysStatus {
    pub present: bool,
    pub count: usize,
    pub source: Option<String>,
    /// Where the user can drop `keys.txt`.
    pub drop_in_path: Option<String>,
}

pub fn status(app: &AppHandle) -> KeysStatus {
    let ring = ring(app);
    KeysStatus {
        present: !ring.is_empty(),
        count: ring.len(),
        source: ring.source().map(|p| p.display().to_string()),
        drop_in_path: paths::data_dir()
            .ok()
            .map(|d| d.join("keys.txt").display().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_keys_and_skips_noise() {
        let key = BASE64_STANDARD.encode([7u8; 32]);
        let text = format!(
            "# comment\n\n{key}\nnot-base64\n{}\n",
            BASE64_STANDARD.encode([1u8; 16])
        );
        let keys = parse_keys(&text);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], [7u8; 32]);
    }
}
