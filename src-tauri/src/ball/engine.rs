//! Applies / removes the custom ball texture and tracks it across updates.
//!
//! The standard ball lives in `GameInfo_Soccar_SF.upk` (texture
//! `Ball_Default00_D`, material `MAT_Ball_V3`). Its image is swapped with
//! [`pipeline::build_swap`]; the package goes to `mods/`, its texture cache
//! next to the stock ones. Only this player sees the ball.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::ball::library::{self, BallPack};
use crate::base::{fsx, paths, AppError, AppResult};
use crate::decals::import::{self, AlphaConsoleSource, ImportReport};
use crate::decals::pipeline::{self, Select, SwapJob, SwapSpec};
use crate::game::writer::{self, EntryState, Owner, Placement};
use crate::game::{fingerprint, install, ReapplyOutcome};
use crate::upk::keys;

pub const BALL_PACKAGE: &str = "GameInfo_Soccar_SF.upk";
pub const BALL_TEXTURE: &str = "Ball_Default00_D";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveState {
    pack_id: String,
    display_name: String,
    build: String,
    applied_at: DateTime<Utc>,
    #[serde(default)]
    preview_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveBall {
    pub pack_id: String,
    pub display_name: String,
    pub applied_at: DateTime<Utc>,
    /// The game changed since the ball was applied.
    pub stale: bool,
    pub preview_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BallStatus {
    pub active: Option<ActiveBall>,
    pub keys_present: bool,
    pub game_found: bool,
    /// AlphaConsole ball packs that can be imported.
    pub alpha_console: Option<AlphaConsoleSource>,
}

fn active_path() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("ball")?.join("active.json"))
}

fn load_active() -> AppResult<Option<ActiveState>> {
    fsx::read_json_or_default(&active_path()?)
}

fn save_active(s: &Option<ActiveState>) -> AppResult<()> {
    fsx::write_json(&active_path()?, s)
}

fn resolve_ci(dir: &Path, file: &str) -> Option<PathBuf> {
    let exact = dir.join(file);
    if exact.is_file() {
        return Some(exact);
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(file))
        .map(|e| e.path())
}

fn alphaconsole_source() -> Option<AlphaConsoleSource> {
    import::source_in(&library::alphaconsole_root()?, &library::app_root()?)
}

pub fn status(app: &AppHandle) -> AppResult<BallStatus> {
    let game = install::current_install(app).ok();
    let build = game
        .as_ref()
        .and_then(fingerprint::current)
        .map(|f| f.id())
        .unwrap_or_default();
    let files_ok = writer::entries_for(Owner::Ball)
        .map(|e| !e.is_empty())
        .unwrap_or(false);
    let active = load_active()?.map(|a| ActiveBall {
        stale: a.build != build || !files_ok,
        pack_id: a.pack_id,
        display_name: a.display_name,
        applied_at: a.applied_at,
        preview_path: a.preview_path,
    });
    Ok(BallStatus {
        active,
        keys_present: !keys::ring(app).is_empty(),
        game_found: game.is_some(),
        alpha_console: alphaconsole_source(),
    })
}

pub fn library() -> Vec<BallPack> {
    library::scan(true)
}

pub fn apply(app: &AppHandle, pack_id: &str) -> AppResult<BallStatus> {
    let pack =
        library::find(pack_id).ok_or_else(|| AppError::NotFound(format!("ball pack {pack_id}")))?;
    if !pack.supported {
        return Err(AppError::Unsupported(format!(
            "the {} variant of this pack has no image",
            pack.ball
        )));
    }
    let image_path = pack
        .image_path
        .as_deref()
        .ok_or_else(|| AppError::Unsupported("this pack has no image".into()))?;
    let install = install::current_install(app)?;
    let ring = keys::ring(app);
    if ring.is_empty() {
        return Err(AppError::KeysMissing);
    }
    let donor_path = resolve_ci(&install.cooked_dir, BALL_PACKAGE)
        .ok_or_else(|| AppError::FileNotFound(BALL_PACKAGE.to_string()))?;
    let donor = std::fs::read(&donor_path).map_err(|e| AppError::from_game_io(e, &donor_path))?;
    let image = pipeline::load_mask(Path::new(image_path))?;
    let read_cache = |name: &str, offset: u64, len: usize| {
        crate::game::tfc::read_range(&install.cooked_dir, name, offset, len)
    };
    let started = std::time::Instant::now();
    let swap = pipeline::build_swap(
        donor,
        &ring,
        &SwapSpec {
            renames: &[],
            target_key: None,
            cache_prefix: "AlxsBall",
            jobs: vec![SwapJob {
                select: Select::Name(BALL_TEXTURE.into()),
                image: &image,
            }],
        },
        &read_cache,
    )?;

    writer::ensure_game_closed()?;
    writer::restore_owner(app, Owner::Ball, None)?;
    save_active(&None)?;
    writer::install_file(
        app,
        Owner::Ball,
        &pack.id,
        &install.cooked_dir.join(format!("{}.tfc", swap.tfc_name)),
        Placement::NewFile,
        &swap.tfc,
    )?;
    if let Err(err) = writer::install_file(
        app,
        Owner::Ball,
        &pack.id,
        &install.mods_dir.join(BALL_PACKAGE),
        Placement::ModsOverride,
        &swap.package,
    ) {
        let _ = writer::restore_owner(app, Owner::Ball, None);
        return Err(err);
    }
    save_active(&Some(ActiveState {
        pack_id: pack.id.clone(),
        display_name: pack.display_name.clone(),
        build: fingerprint::current(&install)
            .map(|f| f.id())
            .unwrap_or_default(),
        applied_at: Utc::now(),
        preview_path: library::preview_of(&pack),
    }))?;
    tracing::info!(pack = %pack.display_name, relocated = ?swap.relocated, inline = swap.inline_replaced, ms = started.elapsed().as_millis() as u64, "custom ball applied");
    status(app)
}

pub fn remove(app: &AppHandle) -> AppResult<BallStatus> {
    writer::restore_owner(app, Owner::Ball, None)?;
    save_active(&None)?;
    status(app)
}

/// Copies the AlphaConsole ball packs into the app's library.
pub fn import_alphaconsole() -> AppResult<ImportReport> {
    let root = library::alphaconsole_root()
        .filter(|r| r.is_dir())
        .ok_or_else(|| AppError::NotFound("AlphaConsole ball folder".into()))?;
    let app = library::app_root().ok_or_else(|| AppError::Internal("no app data folder".into()))?;
    import::import_from(&root, &app, false)
}

/// Pack id of the active ball (used by presets).
pub fn active_pack() -> AppResult<Option<String>> {
    Ok(load_active()?.map(|a| a.pack_id))
}

pub fn reapply_after_update(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    let Some(active) = load_active()? else {
        return Ok(ReapplyOutcome::NothingToDo);
    };
    let install = install::current_install(app)?;
    let build = fingerprint::current(&install)
        .map(|f| f.id())
        .unwrap_or_default();
    let intact = writer::audit(app)?
        .iter()
        .filter(|a| a.entry.owner == Owner::Ball)
        .all(|a| a.state == EntryState::Intact);
    if intact && active.build == build {
        return Ok(ReapplyOutcome::NothingToDo);
    }
    // A ball built from an older package must not stay next to a new build.
    let removed = remove(app);
    if !crate::base::config::current(app).auto_reapply_after_update {
        removed?;
        return Ok(ReapplyOutcome::NeedsUser(format!(
            "custom ball \"{}\" was removed after a game update — apply it again",
            active.display_name
        )));
    }
    match apply(app, &active.pack_id) {
        Ok(_) => Ok(ReapplyOutcome::Reapplied),
        Err(err) => Ok(ReapplyOutcome::NeedsUser(format!(
            "custom ball could not be re-applied: {err}"
        ))),
    }
}
