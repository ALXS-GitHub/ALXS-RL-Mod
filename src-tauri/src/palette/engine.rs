//! Palette engine: reads the stock picker colours from `TAGame.upk`, builds
//! the patched copy for `CookedPCConsole/mods/TAGame.upk`, and tracks what
//! is applied.
//!
//! The stock `TAGame.upk` is never modified. The override is the stock file
//! with only the blocks holding the three picker arrays recompressed (same
//! file size, padded blocks) — RL's `mods/` loader rejects a `TAGame.upk`
//! whose size differs from the stock one.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::base::{config, fsx, paths, AppError, AppResult};
use crate::game::writer::{self, Owner, Placement};
use crate::game::{fingerprint, install, ReapplyOutcome};
use crate::palette::scan::{self, Layout, ACCENT_COUNT, PRIMARY_COUNT, ROWS};
use crate::palette::store::{self, Palette, Rgb};
use crate::upk::chunks::{self, ChunkMap, StreamPatch};
use crate::upk::UpkError;

const TAGAME: &str = "TAGame.upk";

// ── Stock file identity + layout cache ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct FileStamp {
    size: u64,
    mtime: u64,
}

fn stamp(path: &Path) -> AppResult<FileStamp> {
    let meta = std::fs::metadata(path).map_err(|e| AppError::from_game_io(e, path))?;
    let mtime = meta
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(FileStamp {
        size: meta.len(),
        mtime,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EngineCache {
    stamp: FileStamp,
    layout: Layout,
    primary_blue: Vec<Rgb>,
    primary_orange: Vec<Rgb>,
    accent: Vec<Rgb>,
}

fn cache_path() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("palettes")?.join("engine-cache.json"))
}

fn stock_path(install: &install::RlInstall) -> PathBuf {
    install.cooked_dir.join(TAGAME)
}

fn override_path(install: &install::RlInstall) -> PathBuf {
    install.mods_dir.join(TAGAME)
}

fn analyse(bytes: &[u8], stamp: FileStamp) -> AppResult<EngineCache> {
    let map = ChunkMap::scan(bytes)?;
    let layout = scan::locate(bytes, &map)?.ok_or_else(|| {
        AppError::Unsupported("colour picker arrays not found in TAGame.upk".into())
    })?;
    let read = |pos: usize, count: usize| -> AppResult<Vec<Rgb>> {
        let raw = chunks::read_stream(bytes, &map, pos, scan::array_len(count))?;
        Ok(scan::read_colors(&raw, count))
    };
    Ok(EngineCache {
        stamp,
        layout,
        primary_blue: read(layout.blue, PRIMARY_COUNT)?,
        primary_orange: read(layout.orange, PRIMARY_COUNT)?,
        accent: read(layout.accent, ACCENT_COUNT)?,
    })
}

/// Layout + stock colours of the current `TAGame.upk` (cached by size/mtime;
/// a game update invalidates the cache automatically).
fn engine(install: &install::RlInstall) -> AppResult<EngineCache> {
    let path = stock_path(install);
    let current = stamp(&path)?;
    let cache_file = cache_path()?;
    if let Ok(Some(cached)) = fsx::read_json_or_default::<Option<EngineCache>>(&cache_file) {
        if cached.stamp == current {
            return Ok(cached);
        }
    }
    tracing::info!("analysing TAGame.upk for the colour picker layout");
    let bytes = std::fs::read(&path).map_err(|e| AppError::from_game_io(e, &path))?;
    let fresh = analyse(&bytes, current)?;
    fsx::write_json(&cache_file, &Some(fresh.clone()))?;
    Ok(fresh)
}

/// Stock `TAGame.upk` with the palette spliced in. Output size == input size.
pub fn build_override(stock: &[u8], layout: Layout, palette: &Palette) -> AppResult<Vec<u8>> {
    let map = ChunkMap::scan(stock)?;
    let orange: &[Rgb] = if palette.primary_orange.is_empty() {
        &palette.primary_blue
    } else {
        &palette.primary_orange
    };
    let mut patch = StreamPatch::new();
    for (pos, count, colors) in [
        (layout.accent, ACCENT_COUNT, palette.accent.as_slice()),
        (layout.blue, PRIMARY_COUNT, palette.primary_blue.as_slice()),
        (layout.orange, PRIMARY_COUNT, orange),
    ] {
        let original = chunks::read_stream(stock, &map, pos, scan::array_len(count))?;
        if !scan::is_color_array(&original, 0, count) {
            return Err(AppError::Unsupported(
                "TAGame.upk changed since it was analysed".into(),
            ));
        }
        patch.put(pos, scan::encode(&original, colors, count));
    }
    let mut out = stock.to_vec();
    patch.apply(&mut out, &map).map_err(|e| match e {
        UpkError::BudgetExceeded { .. } => AppError::Unsupported(
            "this palette compresses too poorly to fit the game file — use fewer distinct colours"
                .into(),
        ),
        other => other.into(),
    })?;
    debug_assert_eq!(out.len(), stock.len());
    Ok(out)
}

// ── Active state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveState {
    palette_id: String,
    palette_name: String,
    stock: FileStamp,
    build: String,
    applied_at: DateTime<Utc>,
}

fn active_path() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("palettes")?.join("active.json"))
}

fn load_active() -> AppResult<Option<ActiveState>> {
    fsx::read_json_or_default(&active_path()?)
}

fn save_active(state: &Option<ActiveState>) -> AppResult<()> {
    fsx::write_json(&active_path()?, state)
}

// ── Status ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EngineState {
    Ready,
    GameMissing,
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveView {
    pub palette_id: String,
    pub palette_name: String,
    pub applied_at: DateTime<Utc>,
    /// The game changed since the palette was applied (update / verify).
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteStatus {
    pub engine: EngineState,
    pub reason: Option<String>,
    pub active: Option<ActiveView>,
    pub rows: usize,
    pub primary_slots: usize,
    pub accent_slots: usize,
}

pub fn status(app: &AppHandle) -> AppResult<PaletteStatus> {
    let base = |engine, reason: Option<String>, active| PaletteStatus {
        engine,
        reason,
        active,
        rows: ROWS,
        primary_slots: PRIMARY_COUNT,
        accent_slots: ACCENT_COUNT,
    };
    let active = load_active()?;
    let Ok(install) = install::current_install(app) else {
        return Ok(base(EngineState::GameMissing, None, None));
    };
    let view = |stale: bool| {
        active.as_ref().map(|a| ActiveView {
            palette_id: a.palette_id.clone(),
            palette_name: a.palette_name.clone(),
            applied_at: a.applied_at,
            stale,
        })
    };
    let stale = match &active {
        Some(a) => {
            stamp(&stock_path(&install))
                .map(|s| s != a.stock)
                .unwrap_or(true)
                || !override_path(&install).is_file()
        }
        None => false,
    };
    match engine(&install) {
        Ok(_) => Ok(base(EngineState::Ready, None, view(stale))),
        Err(AppError::Unsupported(reason)) => {
            Ok(base(EngineState::Unsupported, Some(reason), view(stale)))
        }
        Err(e) => Err(e),
    }
}

pub fn stock(app: &AppHandle) -> AppResult<Palette> {
    let install = install::current_install(app)?;
    let e = engine(&install)?;
    Ok(Palette {
        id: "stock".into(),
        name: "Stock".into(),
        primary_blue: e.primary_blue,
        primary_orange: e.primary_orange,
        accent: e.accent,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

// ── Apply / restore / re-apply ────────────────────────────────────────────

pub fn apply(app: &AppHandle, id: &str) -> AppResult<PaletteStatus> {
    let palette = store::get(id)?;
    let install = install::current_install(app)?;
    let cache = engine(&install)?;
    let stock_file = stock_path(&install);
    let bytes = std::fs::read(&stock_file).map_err(|e| AppError::from_game_io(e, &stock_file))?;
    let patched = build_override(&bytes, cache.layout, &palette)?;
    writer::install_file(
        app,
        Owner::Palette,
        &palette.id,
        &override_path(&install),
        Placement::ModsOverride,
        &patched,
    )?;
    save_active(&Some(ActiveState {
        palette_id: palette.id.clone(),
        palette_name: palette.name.clone(),
        stock: cache.stamp,
        build: fingerprint::current(&install)
            .map(|f| f.id())
            .unwrap_or_default(),
        applied_at: Utc::now(),
    }))?;
    tracing::info!(palette = %palette.name, "palette applied");
    status(app)
}

pub fn restore(app: &AppHandle) -> AppResult<PaletteStatus> {
    writer::restore_owner(app, Owner::Palette, None)?;
    save_active(&None)?;
    tracing::info!("palette restored to stock");
    status(app)
}

/// Rebuilds the override from the current stock file after a game update.
pub fn reapply_after_update(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    let Some(active) = load_active()? else {
        return Ok(ReapplyOutcome::NothingToDo);
    };
    let install = install::current_install(app)?;
    let current = stamp(&stock_path(&install))?;
    if current == active.stock && override_path(&install).is_file() {
        return Ok(ReapplyOutcome::NothingToDo);
    }
    if !config::current(app).auto_reapply_after_update {
        // A stale override built from an older TAGame.upk can crash the game:
        // remove it rather than leave it in place.
        restore(app)?;
        return Ok(ReapplyOutcome::NeedsUser(format!(
            "palette \"{}\" was removed after a game update — apply it again",
            active.palette_name
        )));
    }
    match apply(app, &active.palette_id) {
        Ok(_) => Ok(ReapplyOutcome::Reapplied),
        Err(err) => {
            let _ = restore(app);
            Ok(ReapplyOutcome::NeedsUser(format!(
                "palette could not be re-applied: {err}"
            )))
        }
    }
}

/// The palette currently applied, if any (used by presets).
pub fn active_palette() -> AppResult<Option<Palette>> {
    match load_active()? {
        Some(a) => store::get(&a.palette_id).map(Some).or_else(|e| match e {
            AppError::NotFound(_) => Ok(None),
            other => Err(other),
        }),
        None => Ok(None),
    }
}

pub fn is_active(id: &str) -> bool {
    matches!(load_active(), Ok(Some(a)) if a.palette_id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::scan::tests::fake_stream;

    fn palette(color: Rgb) -> Palette {
        Palette {
            id: "t".into(),
            name: "t".into(),
            primary_blue: vec![color; PRIMARY_COUNT],
            primary_orange: vec![],
            accent: vec![color; 3],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn override_keeps_size_and_writes_colours() {
        let (stream, layout) = fake_stream();
        // Pad the stream with zeros so each block has slack to recompress into.
        let mut padded = stream.clone();
        padded.extend(vec![0u8; 20_000]);
        let file = chunks::wrap_chunked(&padded).unwrap();
        let red = Rgb { r: 255, g: 0, b: 0 };
        let out = build_override(&file, layout, &palette(red)).unwrap();
        assert_eq!(out.len(), file.len());
        let map = ChunkMap::read_from(&out, 0).unwrap();
        let blue =
            chunks::read_stream(&out, &map, layout.blue, scan::array_len(PRIMARY_COUNT)).unwrap();
        let orange =
            chunks::read_stream(&out, &map, layout.orange, scan::array_len(PRIMARY_COUNT)).unwrap();
        let accent =
            chunks::read_stream(&out, &map, layout.accent, scan::array_len(ACCENT_COUNT)).unwrap();
        assert!(scan::read_colors(&blue, PRIMARY_COUNT)
            .iter()
            .all(|c| *c == red));
        // Orange falls back to blue when empty.
        assert!(scan::read_colors(&orange, PRIMARY_COUNT)
            .iter()
            .all(|c| *c == red));
        let acc = scan::read_colors(&accent, ACCENT_COUNT);
        assert_eq!(acc[0], red);
        assert_ne!(acc[3], red, "unset accent slots keep stock colours");
    }
}
