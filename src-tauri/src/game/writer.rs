//! The single gate for writing into the game install.
//!
//! Every feature (swaps, palette, decals, maps) writes game files through
//! [`install_file`] and removes them through [`restore_file`] /
//! [`restore_owner`]. The manifest (`manifest.json`) records, per file:
//! who wrote it, what was there before (hash + content-addressed backup),
//! what we wrote, and on which game build. That makes every change
//! reversible and lets `integrity` tell apart "still ours", "the game put
//! its own file back" and "a game update replaced it".

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};
use crate::game::{fingerprint, install, watcher};

static MANIFEST_LOCK: Mutex<()> = Mutex::new(());

/// Feature that owns a written file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Owner {
    Swap,
    Palette,
    Decals,
    /// Custom ball texture.
    Ball,
    Maps,
    /// Stats API ini in `TAGame/Config`.
    Stats,
}

/// How the file reaches the game.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Placement {
    /// Dropped into `CookedPCConsole/mods/` (stock file untouched).
    ModsOverride,
    /// Replaces a stock file in place (stock copy kept in `backups/`).
    RootReplace,
    /// A new file next to stock files (e.g. a custom `.tfc`); nothing to back up.
    NewFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub owner: Owner,
    /// Path relative to the install root, `/`-separated.
    pub rel_path: String,
    pub placement: Placement,
    /// Hash of the file that was there before us (`None` = no file).
    pub original_sha256: Option<String>,
    pub written_sha256: String,
    /// Free-form key the owner uses to find its entries (swap id, map id…).
    pub tag: String,
    pub build: String,
    pub written_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Manifest {
    entries: Vec<ManifestEntry>,
}

/// State of a manifest entry compared with what is on disk now.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EntryState {
    /// Our file is still there.
    Intact,
    /// The stock file is back (game "verify files" or manual restore).
    RevertedByGame,
    /// Neither ours nor the original: a game update wrote a new version.
    ChangedByGame,
    /// The file disappeared.
    Missing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryAudit {
    pub entry: ManifestEntry,
    pub state: EntryState,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RestoreOutcome {
    /// The original content (or absence) is back.
    Restored,
    /// Nothing to do: the game already put its own file back.
    AlreadyStock,
    /// The file changed behind our back (game update); left as is.
    LeftUntouched,
}

fn manifest_path() -> AppResult<PathBuf> {
    Ok(paths::data_dir()?.join("manifest.json"))
}

fn backups_dir() -> AppResult<PathBuf> {
    paths::data_subdir("backups")
}

fn load() -> AppResult<Manifest> {
    fsx::read_json_or_default(&manifest_path()?)
}

fn save(m: &Manifest) -> AppResult<()> {
    fsx::write_json(&manifest_path()?, m)
}

fn rel_of(install: &install::RlInstall, target: &Path) -> AppResult<String> {
    if !install.contains(target) {
        return Err(AppError::NotAllowed(format!(
            "{} is outside the game install",
            target.display()
        )));
    }
    let rel = dunce::simplified(target)
        .strip_prefix(&install.root)
        .map_err(|_| {
            AppError::NotAllowed(format!("{} is outside the game install", target.display()))
        })?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

fn abs_of(install: &install::RlInstall, rel: &str) -> PathBuf {
    install
        .root
        .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
}

/// Refuses game writes while the game runs (it locks packages and would
/// read half-applied state).
pub fn ensure_game_closed() -> AppResult<()> {
    if watcher::probe_now().running {
        return Err(AppError::GameRunning);
    }
    Ok(())
}

/// Maps live in `mods/` and the game only opens them when a match loads
/// them, so they can change while it runs (a file the game holds open fails
/// with `FileLocked`). Everything else is read at startup: game closed only.
pub fn hot_swappable(owner: Owner) -> bool {
    owner == Owner::Maps
}

fn ensure_writable(owner: Owner) -> AppResult<()> {
    if hot_swappable(owner) {
        Ok(())
    } else {
        ensure_game_closed()
    }
}

/// Writes `bytes` to `target` (absolute, inside the install) on behalf of
/// `owner`. Re-writing a file we already own keeps the original backup.
pub fn install_file(
    app: &AppHandle,
    owner: Owner,
    tag: &str,
    target: &Path,
    placement: Placement,
    bytes: &[u8],
) -> AppResult<ManifestEntry> {
    ensure_writable(owner)?;
    let install = install::current_install(app)?;
    let rel = rel_of(&install, target)?;
    let _guard = MANIFEST_LOCK
        .lock()
        .map_err(|_| AppError::Internal("manifest lock poisoned".into()))?;
    let mut manifest = load()?;

    let existing = manifest
        .entries
        .iter()
        .position(|e| e.rel_path.eq_ignore_ascii_case(&rel));
    if let Some(i) = existing {
        if manifest.entries[i].owner != owner {
            return Err(AppError::Conflict(format!(
                "{rel} is already modified by {:?}; restore it first",
                manifest.entries[i].owner
            )));
        }
    }

    let original_sha256 = match existing {
        Some(i) => manifest.entries[i].original_sha256.clone(),
        None => {
            let current = fsx::sha256_file_opt(target)?;
            if let (Some(sha), Placement::RootReplace) = (&current, placement) {
                let backup = backups_dir()?.join(format!("{sha}.bak"));
                if !backup.is_file() {
                    fsx::copy_atomic(target, &backup)?;
                }
            }
            current
        }
    };

    fsx::write_atomic(target, bytes)?;
    let entry = ManifestEntry {
        owner,
        rel_path: rel,
        placement,
        original_sha256,
        written_sha256: fsx::sha256_bytes(bytes),
        tag: tag.to_string(),
        build: fingerprint::current(&install)
            .map(|f| f.id())
            .unwrap_or_default(),
        written_at: chrono::Utc::now(),
    };
    match existing {
        Some(i) => manifest.entries[i] = entry.clone(),
        None => manifest.entries.push(entry.clone()),
    }
    save(&manifest)?;
    tracing::info!(owner = ?entry.owner, path = %entry.rel_path, "game file written");
    Ok(entry)
}

fn restore_entry(install: &install::RlInstall, entry: &ManifestEntry) -> AppResult<RestoreOutcome> {
    let target = abs_of(install, &entry.rel_path);
    let current = fsx::sha256_file_opt(&target)?;
    if current.as_deref() != Some(entry.written_sha256.as_str()) {
        // Not our bytes anymore: either the stock file is back or the game
        // shipped a new one. In both cases the right move is to leave it.
        return Ok(if current == entry.original_sha256 {
            RestoreOutcome::AlreadyStock
        } else {
            RestoreOutcome::LeftUntouched
        });
    }
    match &entry.original_sha256 {
        None => std::fs::remove_file(&target).map_err(|e| AppError::from_game_io(e, &target))?,
        Some(sha) => {
            let backup = backups_dir()?.join(format!("{sha}.bak"));
            if !backup.is_file() || fsx::sha256_file(&backup)? != *sha {
                return Err(AppError::HashMismatch(format!(
                    "backup of {}",
                    entry.rel_path
                )));
            }
            fsx::copy_atomic(&backup, &target)?;
        }
    }
    Ok(RestoreOutcome::Restored)
}

/// Deletes backups no manifest entry references anymore.
fn prune_backups(manifest: &Manifest) {
    let Ok(dir) = backups_dir() else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(sha) = name.strip_suffix(".bak") else {
            continue;
        };
        let used = manifest
            .entries
            .iter()
            .any(|m| m.original_sha256.as_deref() == Some(sha));
        if !used {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Restores one file (absolute path) and forgets it.
pub fn restore_file(app: &AppHandle, target: &Path) -> AppResult<RestoreOutcome> {
    let install = install::current_install(app)?;
    let rel = rel_of(&install, target)?;
    let _guard = MANIFEST_LOCK
        .lock()
        .map_err(|_| AppError::Internal("manifest lock poisoned".into()))?;
    let mut manifest = load()?;
    let Some(i) = manifest
        .entries
        .iter()
        .position(|e| e.rel_path.eq_ignore_ascii_case(&rel))
    else {
        return Ok(RestoreOutcome::AlreadyStock);
    };
    ensure_writable(manifest.entries[i].owner)?;
    let outcome = restore_entry(&install, &manifest.entries[i])?;
    manifest.entries.remove(i);
    save(&manifest)?;
    prune_backups(&manifest);
    Ok(outcome)
}

/// Restores every file of every owner (the uninstaller's `--restore-stock`).
/// Feature state files are left as they are: each feature reports its files
/// as missing and rebuilds or forgets them. Returns the files restored.
pub fn restore_everything(app: &AppHandle) -> AppResult<usize> {
    let mut restored = 0;
    let mut first_err = None;
    for owner in [
        Owner::Swap,
        Owner::Palette,
        Owner::Decals,
        Owner::Ball,
        Owner::Maps,
        Owner::Stats,
    ] {
        match restore_owner(app, owner, None) {
            Ok(outcomes) => restored += outcomes.len(),
            Err(err) => {
                tracing::warn!(?owner, %err, "restore failed");
                first_err.get_or_insert(err);
            }
        }
    }
    match first_err {
        Some(err) => Err(err),
        None => Ok(restored),
    }
}

/// Restores every file of `owner` whose tag matches (`None` = all tags).
pub fn restore_owner(
    app: &AppHandle,
    owner: Owner,
    tag: Option<&str>,
) -> AppResult<Vec<RestoreOutcome>> {
    restore_where(app, owner, |e| tag.is_none_or(|t| e.tag == t))
}

/// Restores every file of `owner` selected by `pick`.
pub fn restore_where(
    app: &AppHandle,
    owner: Owner,
    pick: impl Fn(&ManifestEntry) -> bool,
) -> AppResult<Vec<RestoreOutcome>> {
    ensure_writable(owner)?;
    let install = install::current_install(app)?;
    let _guard = MANIFEST_LOCK
        .lock()
        .map_err(|_| AppError::Internal("manifest lock poisoned".into()))?;
    let mut manifest = load()?;
    let mut outcomes = Vec::new();
    let mut kept = Vec::with_capacity(manifest.entries.len());
    let mut failure = None;
    for entry in std::mem::take(&mut manifest.entries) {
        if failure.is_none() && entry.owner == owner && pick(&entry) {
            match restore_entry(&install, &entry) {
                Ok(outcome) => outcomes.push(outcome),
                Err(err) => {
                    // Keep the entry (still ours on disk) and stop there.
                    failure = Some(err);
                    kept.push(entry);
                }
            }
        } else {
            kept.push(entry);
        }
    }
    manifest.entries = kept;
    save(&manifest)?;
    prune_backups(&manifest);
    match failure {
        Some(err) => Err(err),
        None => Ok(outcomes),
    }
}

/// Forgets entries without touching disk (used after a game update made
/// them obsolete — the stock file is already the new version).
pub fn forget(owner: Owner, rel_paths: &[String]) -> AppResult<()> {
    let _guard = MANIFEST_LOCK
        .lock()
        .map_err(|_| AppError::Internal("manifest lock poisoned".into()))?;
    let mut manifest = load()?;
    manifest.entries.retain(|e| {
        !(e.owner == owner
            && rel_paths
                .iter()
                .any(|r| r.eq_ignore_ascii_case(&e.rel_path)))
    });
    save(&manifest)?;
    prune_backups(&manifest);
    Ok(())
}

/// Stock bytes of a game file, whoever modified it: if the file currently
/// holds bytes we wrote, the pristine copy comes from the verified backup
/// (or `NotFound` if there was no stock file); otherwise the file on disk
/// is stock and is returned as is.
pub fn stock_bytes(install: &install::RlInstall, target: &Path) -> AppResult<Vec<u8>> {
    let rel = rel_of(install, target)?;
    let entry = load()?
        .entries
        .into_iter()
        .find(|e| e.rel_path.eq_ignore_ascii_case(&rel));
    if let Some(entry) = entry {
        if fsx::sha256_file_opt(target)?.as_deref() == Some(entry.written_sha256.as_str()) {
            let Some(sha) = entry.original_sha256 else {
                return Err(AppError::NotFound(format!("no stock copy of {rel}")));
            };
            let backup = backups_dir()?.join(format!("{sha}.bak"));
            let bytes = std::fs::read(&backup).map_err(|e| AppError::from_game_io(e, &backup))?;
            if fsx::sha256_bytes(&bytes) != sha {
                return Err(AppError::HashMismatch(format!("backup of {rel}")));
            }
            return Ok(bytes);
        }
    }
    std::fs::read(target).map_err(|e| AppError::from_game_io(e, target))
}

pub fn entries_for(owner: Owner) -> AppResult<Vec<ManifestEntry>> {
    Ok(load()?
        .entries
        .into_iter()
        .filter(|e| e.owner == owner)
        .collect())
}

/// Compares every entry with the disk. Read-only.
pub fn audit(app: &AppHandle) -> AppResult<Vec<EntryAudit>> {
    let install = install::current_install(app)?;
    let manifest = load()?;
    manifest
        .entries
        .into_iter()
        .map(|entry| {
            let current = fsx::sha256_file_opt(&abs_of(&install, &entry.rel_path))?;
            let state = match current {
                None => EntryState::Missing,
                Some(ref h) if *h == entry.written_sha256 => EntryState::Intact,
                Some(ref h) if Some(h) == entry.original_sha256.as_ref() => {
                    EntryState::RevertedByGame
                }
                Some(_) => EntryState::ChangedByGame,
            };
            Ok(EntryAudit { entry, state })
        })
        .collect()
}
