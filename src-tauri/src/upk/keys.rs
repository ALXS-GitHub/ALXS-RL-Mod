//! AES-256 key ring for cooked package header tables.
//!
//! Keys are the public community list (`keys.txt`, one base64 key per line).
//! The app never ships them: the user downloads the community list
//! ([`COMMUNITY_URL`], on explicit action) or imports a file; both are
//! validated and merged into the drop-in file. The ring is loaded from, in order,
//! 1. `%LOCALAPPDATA%\ALXS-RL-Mod\keys.txt` (user drop-in, wins),
//! 2. the bundled resource `keys/keys.txt` (if a release ever ships one),
//! 3. `src-tauri/resources/keys/keys.txt` (dev builds on this machine).
//!
//! The ring is cached; [`reload`] re-reads it (e.g. after the user added the file).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use base64::prelude::{Engine as _, BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::base::security::{self, Purpose};
use crate::base::{fsx, paths, AppError, AppResult};

pub type AesKey = [u8; 32];

/// Community key list: `aes.txt` of the Toga-Files repo, updated after each
/// game patch (one base64 key per line, same format as `keys.txt`).
pub const COMMUNITY_URL: &str =
    "https://raw.githubusercontent.com/ShinyEmii/Toga-Files/master/aes.txt";
/// Where the user can see the list before downloading it.
pub const COMMUNITY_PAGE: &str = "https://github.com/ShinyEmii/Toga-Files";
/// The real list is ~50 KB (1 100 keys); anything far larger is not it.
const MAX_LIST_BYTES: usize = 1024 * 1024;
const MAX_KEYS: usize = 20_000;

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

/// Strict parse for a downloaded list: every non-blank, non-comment line
/// must be one base64 AES-256 key, otherwise the whole list is refused (an
/// error page or a changed format must never be half-imported).
pub fn parse_strict(text: &str) -> Result<Vec<AesKey>, String> {
    let mut keys = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let key = BASE64_STANDARD
            .decode(line)
            .ok()
            .and_then(|b| AesKey::try_from(b).ok())
            .ok_or_else(|| format!("line {} is not an AES-256 key", n + 1))?;
        keys.push(key);
        if keys.len() > MAX_KEYS {
            return Err(format!("more than {MAX_KEYS} keys"));
        }
    }
    if keys.is_empty() {
        return Err("no key in the list".into());
    }
    Ok(keys)
}

/// `existing` followed by the keys of `incoming` it lacks (order kept,
/// duplicates dropped). Returns the merged list and how many were added.
pub fn merge(existing: &[AesKey], incoming: &[AesKey]) -> (Vec<AesKey>, usize) {
    let mut seen: HashSet<AesKey> = HashSet::new();
    let mut out: Vec<AesKey> = existing
        .iter()
        .filter(|k| seen.insert(**k))
        .copied()
        .collect();
    let before = out.len();
    out.extend(incoming.iter().filter(|k| seen.insert(**k)));
    let added = out.len() - before;
    (out, added)
}

fn to_text(keys: &[AesKey]) -> String {
    keys.iter()
        .map(|k| BASE64_STANDARD.encode(k) + "\n")
        .collect()
}

fn drop_in() -> AppResult<PathBuf> {
    Ok(paths::data_dir()?.join("keys.txt"))
}

/// Merges `incoming` into the current ring and writes the result to the
/// drop-in file (existing keys are never removed). Returns (added, total).
fn store(app: &AppHandle, incoming: &[AesKey]) -> AppResult<(usize, usize)> {
    let current = ring(app);
    let (merged, added) = merge(current.keys(), incoming);
    let target = drop_in()?;
    if added > 0 || current.source() != Some(&target) {
        fsx::write_atomic(&target, to_text(&merged).as_bytes())?;
    }
    let total = reload(app).len();
    Ok((added, total))
}

/// Downloads the community list ([`COMMUNITY_URL`]) and validates it with
/// [`parse_strict`].
pub async fn download_community() -> AppResult<Vec<AesKey>> {
    security::ensure_allowed(COMMUNITY_URL, Purpose::Keys)?;
    let res = security::http_client(Purpose::Keys)?
        .get(COMMUNITY_URL)
        .send()
        .await?
        .error_for_status()?;
    let too_large = || AppError::InvalidInput("key list larger than 1 MB".into());
    if res
        .content_length()
        .is_some_and(|n| n as usize > MAX_LIST_BYTES)
    {
        return Err(too_large());
    }
    let bytes = res.bytes().await?;
    if bytes.len() > MAX_LIST_BYTES {
        return Err(too_large());
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| AppError::InvalidInput("key list is not text".into()))?;
    parse_strict(text)
        .map_err(|e| AppError::InvalidInput(format!("community key list refused: {e}")))
}

/// Merges downloaded keys (see [`download_community`]) into the ring and
/// records the game build they were checked for. Returns (added, total).
pub fn add_downloaded(app: &AppHandle, keys: &[AesKey], build: &str) -> AppResult<(usize, usize)> {
    let out = store(app, keys)?;
    mark_checked(build)?;
    tracing::info!(added = out.0, total = out.1, "community keys downloaded");
    Ok(out)
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

/// Merges the keys of a user-chosen file into the app's drop-in location
/// (`%LOCALAPPDATA%\ALXS-RL-Mod\keys.txt`, read first) and reloads the
/// ring. Malformed lines are skipped; a file without any key is refused.
/// Returns (added, total).
pub fn import_file(app: &AppHandle, source: &Path) -> AppResult<(usize, usize)> {
    if std::fs::metadata(source)?.len() as usize > MAX_LIST_BYTES {
        return Err(AppError::InvalidInput(format!(
            "{} is too large for a key list",
            source.display()
        )));
    }
    let text = std::fs::read_to_string(source)?;
    let keys = parse_keys(&text);
    if keys.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "{} holds no valid AES-256 key (one base64 key per line expected)",
            source.display()
        )));
    }
    let out = store(app, &keys)?;
    tracing::info!(added = out.0, total = out.1, "key file imported");
    Ok(out)
}

/// Game build for which the community list was last checked, so the app
/// suggests an update only after a game patch.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct KeysMeta {
    checked_build: Option<String>,
}

fn meta_path() -> AppResult<PathBuf> {
    Ok(paths::data_dir()?.join("keys-meta.json"))
}

fn load_meta() -> KeysMeta {
    meta_path()
        .and_then(|p| fsx::read_json_or_default(&p))
        .unwrap_or_default()
}

/// Records that the update suggestion for `build` was acted on or dismissed.
pub fn mark_checked(build: &str) -> AppResult<()> {
    fsx::write_json(
        &meta_path()?,
        &KeysMeta {
            checked_build: Some(build.to_string()),
        },
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeysStatus {
    pub present: bool,
    pub count: usize,
    pub source: Option<String>,
    /// Where the user can drop `keys.txt`.
    pub drop_in_path: Option<String>,
    /// Game build for which the community list was last checked.
    pub checked_build: Option<String>,
    /// Page of the community list (shown before downloading).
    pub community_page: &'static str,
}

/// Result of a download or an import.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeysUpdate {
    /// Keys the ring did not have yet.
    pub added: usize,
    pub total: usize,
    /// Catalog items readable now that were not before.
    pub unlocked: usize,
    pub status: KeysStatus,
}

pub fn status(app: &AppHandle) -> KeysStatus {
    let ring = ring(app);
    KeysStatus {
        present: !ring.is_empty(),
        count: ring.len(),
        source: ring.source().map(|p| p.display().to_string()),
        drop_in_path: drop_in().ok().map(|p| p.display().to_string()),
        checked_build: load_meta().checked_build,
        community_page: COMMUNITY_PAGE,
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

    #[test]
    fn strict_parse_refuses_anything_but_keys() {
        let key = BASE64_STANDARD.encode([7u8; 32]);
        let list = format!("{key}\r\n\n# note\n{key}\n");
        assert_eq!(parse_strict(&list).unwrap().len(), 2);
        assert!(parse_strict("404: Not Found").is_err());
        assert!(parse_strict("<html><body>error</body></html>").is_err());
        let short = BASE64_STANDARD.encode([1u8; 16]);
        assert!(parse_strict(&format!("{key}\n{short}\n")).is_err());
        assert!(parse_strict("\n# empty\n").is_err());
    }

    #[test]
    fn merge_keeps_existing_and_adds_new_once() {
        let (a, b, c) = ([1u8; 32], [2u8; 32], [3u8; 32]);
        let (merged, added) = merge(&[a, b, a], &[b, c, c]);
        assert_eq!(merged, vec![a, b, c]);
        assert_eq!(added, 1);
        assert_eq!(parse_keys(&to_text(&merged)), merged);
    }
}
