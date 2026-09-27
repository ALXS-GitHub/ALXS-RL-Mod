//! Applies / removes a custom decal and tracks it across game updates.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::base::{config, fsx, paths, AppError, AppResult};
use crate::decals::import;
use crate::decals::library::{self, LibraryRoot};
use crate::decals::pipeline::{self, MASK_TFC_CUSTOM};
use crate::decals::targets::{self, TargetSlot};
use crate::game::writer::{self, EntryState, Owner, Placement};
use crate::game::{fingerprint, install, ReapplyOutcome};
use crate::upk::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecalApplyRequest {
    pub pack_id: String,
    /// Target slot key (`octane-stars`…); default = first slot of the body.
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveState {
    pack_id: String,
    display_name: String,
    body_id: i32,
    target: String,
    build: String,
    applied_at: DateTime<Utc>,
    /// Cached preview of the pack (shown in the active mods panel).
    #[serde(default)]
    preview_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDecal {
    pub pack_id: String,
    pub display_name: String,
    pub body_name: String,
    pub target: Option<TargetSlot>,
    pub applied_at: DateTime<Utc>,
    /// The game changed since the decal was applied.
    pub stale: bool,
    pub preview_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecalStatus {
    pub active: Option<ActiveDecal>,
    pub targets: Vec<TargetSlot>,
    pub library_roots: Vec<LibraryRoot>,
    pub keys_present: bool,
    pub game_found: bool,
    /// AlphaConsole packs that can be imported into the app's library.
    pub alpha_console: Option<import::AlphaConsoleSource>,
}

/// Leftovers of the previous app's experiments (never tracked by the
/// manifest). Removed before writing so they can't shadow our files.
const LEGACY_LEFTOVERS: &[&str] = &[
    "MyDecal_.tfc",
    "MyDecal01.tfc",
    "MyDecal02.tfc",
    "MyDecal_3.tfc",
    "MyDecal3_.tfc",
];

/// Files produced by either pipeline.
struct Built {
    tfc_file: String,
    tfc: Vec<u8>,
    package: Vec<u8>,
}

fn active_path() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("decals")?.join("active.json"))
}

fn load_active() -> AppResult<Option<ActiveState>> {
    fsx::read_json_or_default(&active_path()?)
}

fn save_active(s: &Option<ActiveState>) -> AppResult<()> {
    fsx::write_json(&active_path()?, s)
}

fn library_roots(app: &AppHandle) -> Vec<(PathBuf, bool)> {
    library::roots(&config::current(app).decal_library_folders)
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

fn slot_for(key: &str) -> Option<TargetSlot> {
    targets::all_slots().into_iter().find(|s| s.key == key)
}

pub fn status(app: &AppHandle) -> AppResult<DecalStatus> {
    let game = install::current_install(app).ok();
    let build = game
        .as_ref()
        .and_then(fingerprint::current)
        .map(|f| f.id())
        .unwrap_or_default();
    let active = load_active()?.map(|a| {
        let files_ok = writer::entries_for(Owner::Decals)
            .map(|e| !e.is_empty())
            .unwrap_or(false);
        ActiveDecal {
            pack_id: a.pack_id.clone(),
            display_name: a.display_name.clone(),
            body_name: targets::body_name(a.body_id).unwrap_or("?").to_string(),
            target: slot_for(&a.target),
            applied_at: a.applied_at,
            stale: a.build != build || !files_ok,
            // Decals applied before previews were recorded: look the pack up.
            preview_path: a.preview_path.clone().or_else(|| {
                library::find(&library_roots(app), &a.pack_id)
                    .and_then(|p| library::art_preview(&p))
            }),
        }
    });
    Ok(DecalStatus {
        active,
        targets: targets::all_slots(),
        library_roots: library::root_infos(&library_roots(app)),
        keys_present: !keys::ring(app).is_empty(),
        game_found: game.is_some(),
        alpha_console: import::source(),
    })
}

/// Points the active decal at its pack's current id (ids moved from the full
/// Template.json path to the path inside the library; packs also move when
/// imported from AlphaConsole).
pub fn migrate_active(app: &AppHandle) -> AppResult<()> {
    let Some(mut a) = load_active()? else {
        return Ok(());
    };
    let Some(pack) = library::find(&library_roots(app), &a.pack_id) else {
        return Ok(());
    };
    if pack.id != a.pack_id {
        tracing::info!(from = %a.pack_id, to = %pack.id, "active decal pack id updated");
        a.pack_id = pack.id.clone();
        a.preview_path = library::art_preview(&pack);
        save_active(&Some(a))?;
    }
    Ok(())
}

/// Imports the AlphaConsole packs, then re-points the active decal.
pub fn import_alphaconsole(app: &AppHandle, convert: bool) -> AppResult<import::ImportReport> {
    let report = import::import(convert)?;
    migrate_active(app)?;
    Ok(report)
}

pub fn library(app: &AppHandle) -> Vec<library::DecalPack> {
    library::scan(&library_roots(app), true)
}

pub fn apply(app: &AppHandle, req: &DecalApplyRequest) -> AppResult<DecalStatus> {
    let pack = library::find(&library_roots(app), &req.pack_id)
        .ok_or_else(|| AppError::NotFound(format!("decal pack {}", req.pack_id)))?;
    let donor = targets::donor_for(pack.body_id).ok_or_else(|| {
        AppError::Unsupported(format!("body {} is not supported yet", pack.body_id))
    })?;
    let target = targets::find_target(donor, req.target.as_deref())
        .ok_or_else(|| AppError::InvalidInput(format!("unknown target slot {:?}", req.target)))?;
    let mask_path = || {
        pack.mask_path.as_deref().ok_or_else(|| {
            AppError::Unsupported("this pack has no colour-zone mask (Skin) PNG".into())
        })
    };

    let install = install::current_install(app)?;
    let ring = keys::ring(app);
    if ring.is_empty() {
        return Err(AppError::KeysMissing);
    }
    let read_donor = |file: &str| -> AppResult<Vec<u8>> {
        let path = resolve_ci(&install.cooked_dir, file)
            .ok_or_else(|| AppError::FileNotFound(file.to_string()))?;
        std::fs::read(&path).map_err(|e| AppError::from_game_io(e, &path))
    };
    // The stock package of the target slot tells which key the game will
    // use to decrypt our file.
    let target_key = resolve_ci(&install.cooked_dir, &target.file_name())
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| {
            let summary = crate::upk::summary::PackageSummary::parse(&bytes).ok()?;
            crate::upk::crypto::open_header(&bytes, &summary, &ring)
                .ok()?
                .key
        });
    let read_cache = |name: &str, offset: u64, len: usize| {
        crate::game::tfc::read_range(&install.cooked_dir, name, offset, len)
    };
    let started = std::time::Instant::now();
    let built = match (
        pack.hybrid,
        donor.hybrid.as_ref(),
        pack.diffuse_path.as_deref(),
    ) {
        _ if pack.universal => {
            let universal = donor.universal.as_ref().ok_or_else(|| {
                AppError::Unsupported("universal decals are not supported".into())
            })?;
            let art_path = pack.diffuse_path.as_deref().ok_or_else(|| {
                AppError::Unsupported("this pack has no 1_Diffuse_Skin image".into())
            })?;
            let art = pipeline::load_mask(Path::new(art_path))?;
            // No logo sheet in the pack: an empty one, so the donor's team
            // logo does not show.
            let trim = match library::role_path(&pack, pipeline::TRIM_PARAM) {
                Some(p) => pipeline::load_mask(Path::new(&p))?,
                None => image::RgbaImage::new(512, 512),
            };
            let swap = pipeline::build_swap(
                read_donor(universal.file)?,
                &ring,
                &pipeline::SwapSpec {
                    renames: &targets::hybrid_renames(universal, target),
                    target_key,
                    cache_prefix: "AlxsUniv",
                    jobs: vec![
                        pipeline::SwapJob {
                            select: pipeline::Select::Param(pipeline::ART_PARAM.into()),
                            image: &art,
                        },
                        pipeline::SwapJob {
                            select: pipeline::Select::Param(pipeline::TRIM_PARAM.into()),
                            image: &trim,
                        },
                    ],
                },
                &read_cache,
            )?;
            tracing::info!(textures = ?swap.textures, relocated = ?swap.relocated, mips = swap.mips_replaced, cache = %swap.tfc_name, "universal decal built");
            Built {
                tfc_file: format!("{}.tfc", swap.tfc_name),
                tfc: swap.tfc,
                package: swap.package,
            }
        }
        (true, Some(hybrid), Some(art_path)) => {
            let mask = pipeline::load_mask(Path::new(mask_path()?))?;
            let art = pipeline::load_mask(Path::new(art_path))?;
            let swap = pipeline::build_hybrid(
                read_donor(hybrid.file)?,
                &ring,
                target_key,
                &targets::hybrid_renames(hybrid, target),
                &art,
                &mask,
                &read_cache,
            )?;
            tracing::info!(textures = ?swap.textures, relocated = ?swap.relocated, mips = swap.mips_replaced, inline = swap.inline_replaced, cache = %swap.tfc_name, "hybrid decal built");
            Built {
                tfc_file: format!("{}.tfc", swap.tfc_name),
                tfc: swap.tfc,
                package: swap.package,
            }
        }
        _ => {
            let paintable = donor.paintable.as_ref().ok_or_else(|| {
                AppError::Unsupported(format!(
                    "{} only takes packs with an image and a zone mask",
                    donor.body_name
                ))
            })?;
            let mask = pipeline::load_mask(Path::new(mask_path()?))?;
            let (required, optional) = targets::renames(donor, paintable, target);
            let swap = pipeline::build(
                read_donor(paintable.file)?,
                &ring,
                target_key,
                &required,
                &optional,
                &mask,
            )?;
            tracing::info!(mask = %swap.mask_texture, mips = swap.mips_replaced, "paintable decal built");
            Built {
                tfc_file: format!("{MASK_TFC_CUSTOM}.tfc"),
                tfc: swap.tfc,
                package: swap.package,
            }
        }
    };

    // One custom decal at a time: clear the previous one, then write.
    writer::ensure_game_closed()?;
    writer::restore_owner(app, Owner::Decals, None)?;
    save_active(&None)?;
    let tracked: Vec<String> = writer::entries_for(Owner::Decals)?
        .into_iter()
        .map(|e| e.rel_path)
        .collect();
    for name in LEGACY_LEFTOVERS {
        let p = install.cooked_dir.join(name);
        if p.is_file() && !tracked.iter().any(|t| t.ends_with(name)) {
            let _ = std::fs::remove_file(&p);
        }
    }

    writer::install_file(
        app,
        Owner::Decals,
        &pack.id,
        &install.cooked_dir.join(&built.tfc_file),
        Placement::NewFile,
        &built.tfc,
    )?;
    if let Err(err) = writer::install_file(
        app,
        Owner::Decals,
        &pack.id,
        &install.mods_dir.join(target.file_name()),
        Placement::ModsOverride,
        &built.package,
    ) {
        let _ = writer::restore_owner(app, Owner::Decals, None);
        return Err(err);
    }
    save_active(&Some(ActiveState {
        pack_id: pack.id.clone(),
        display_name: pack.display_name.clone(),
        body_id: pack.body_id,
        target: target.key.to_string(),
        build: fingerprint::current(&install)
            .map(|f| f.id())
            .unwrap_or_default(),
        applied_at: Utc::now(),
        preview_path: library::art_preview(&pack),
    }))?;
    tracing::info!(pack = %pack.display_name, target = target.key, ms = started.elapsed().as_millis() as u64, "custom decal applied");
    status(app)
}

/// Request that would re-create the active decal (used by presets).
pub fn active_request() -> AppResult<Option<DecalApplyRequest>> {
    Ok(load_active()?.map(|a| DecalApplyRequest {
        pack_id: a.pack_id,
        target: Some(a.target),
    }))
}

pub fn remove(app: &AppHandle) -> AppResult<DecalStatus> {
    writer::restore_owner(app, Owner::Decals, None)?;
    save_active(&None)?;
    status(app)
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
        .filter(|a| a.entry.owner == Owner::Decals)
        .all(|a| a.state == EntryState::Intact);
    if intact && active.build == build {
        return Ok(ReapplyOutcome::NothingToDo);
    }
    // Files built from an older donor must not stay next to a new build.
    let removed = remove(app);
    if !config::current(app).auto_reapply_after_update {
        removed?;
        return Ok(ReapplyOutcome::NeedsUser(format!(
            "custom decal \"{}\" was removed after a game update — apply it again",
            active.display_name
        )));
    }
    let req = DecalApplyRequest {
        pack_id: active.pack_id.clone(),
        target: Some(active.target.clone()),
    };
    match apply(app, &req) {
        Ok(_) => Ok(ReapplyOutcome::Reapplied),
        Err(err) => Ok(ReapplyOutcome::NeedsUser(format!(
            "custom decal could not be re-applied: {err}"
        ))),
    }
}
