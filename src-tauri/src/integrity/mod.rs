//! Detects what a game update or a "verify files" pass did to our changes,
//! and rebuilds them from the *current* stock files.
//!
//! - Game update: the build fingerprint differs from the one recorded after
//!   the last successful check → every feature with files in the install is
//!   rebuilt (a `mods/` override stays byte-identical but is now stale).
//! - Verify / partial update: `game::writer::audit` reports files that are
//!   not ours anymore → their owners are rebuilt.

pub mod commands;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::base::error::AppResult;
use crate::base::{config, fsx, paths};
use crate::game::writer::{self, EntryState, Owner};
use crate::game::{fingerprint, install, watcher, ReapplyOutcome};

pub const EVENT_REPORT: &str = "integrity://report";
const STARTUP_DELAY: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityFailure {
    pub feature: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub game_updated: bool,
    pub files_reverted: u32,
    pub reapplied: Vec<String>,
    pub pending: Vec<String>,
    pub failures: Vec<IntegrityFailure>,
    pub checked_at: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct IntegrityState {
    /// Fingerprint after the last check that left everything consistent.
    last_build: Option<String>,
}

fn state_path() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("integrity")?.join("state.json"))
}

/// Feature names as reported to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Swap,
    Palette,
    Decals,
    Ball,
    Maps,
    Stats,
}

impl Feature {
    fn name(self) -> &'static str {
        match self {
            Feature::Swap => "swap",
            Feature::Palette => "palette",
            Feature::Decals => "decals",
            Feature::Ball => "ball",
            Feature::Maps => "maps",
            Feature::Stats => "stats",
        }
    }

    fn from_owner(owner: Owner) -> Feature {
        match owner {
            Owner::Swap => Feature::Swap,
            Owner::Palette => Feature::Palette,
            Owner::Decals => Feature::Decals,
            Owner::Ball => Feature::Ball,
            Owner::Maps => Feature::Maps,
            Owner::Stats => Feature::Stats,
        }
    }

    /// `None` for features without an automatic rebuild (stats ini).
    fn reapply(self, app: &AppHandle) -> Option<AppResult<ReapplyOutcome>> {
        match self {
            Feature::Swap => Some(crate::swap::reapply_after_update(app)),
            Feature::Palette => Some(crate::palette::reapply_after_update(app)),
            Feature::Decals => Some(crate::decals::reapply_after_update(app)),
            Feature::Ball => Some(crate::ball::reapply_after_update(app)),
            Feature::Maps => Some(crate::maps::reapply_after_update(app)),
            Feature::Stats => None,
        }
    }
}

const REBUILDABLE: [Feature; 5] = [
    Feature::Swap,
    Feature::Palette,
    Feature::Decals,
    Feature::Ball,
    Feature::Maps,
];

struct Assessment {
    game_updated: bool,
    files_reverted: u32,
    affected: BTreeSet<Feature>,
    current_build: Option<String>,
}

fn assess(app: &AppHandle) -> AppResult<Assessment> {
    let install = install::current_install(app)?;
    let current_build = fingerprint::current(&install).map(|f| f.id());
    let state: IntegrityState = fsx::read_json_or_default(&state_path()?)?;
    let audits = writer::audit(app)?;

    let game_updated =
        matches!((&state.last_build, &current_build), (Some(last), Some(now)) if last != now);
    let mut affected = BTreeSet::new();
    let mut files_reverted = 0u32;
    for a in &audits {
        if a.state != EntryState::Intact {
            files_reverted += 1;
            affected.insert(Feature::from_owner(a.entry.owner));
        } else if game_updated {
            affected.insert(Feature::from_owner(a.entry.owner));
        }
    }
    Ok(Assessment {
        game_updated,
        files_reverted,
        affected,
        current_build,
    })
}

fn remember_build(build: Option<String>) -> AppResult<()> {
    fsx::write_json(&state_path()?, &IntegrityState { last_build: build })
}

fn empty_report(a: &Assessment) -> IntegrityReport {
    IntegrityReport {
        game_updated: a.game_updated,
        files_reverted: a.files_reverted,
        reapplied: Vec::new(),
        pending: Vec::new(),
        failures: Vec::new(),
        checked_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Read-only: what would need rebuilding.
pub fn check(app: &AppHandle) -> AppResult<IntegrityReport> {
    let a = assess(app)?;
    let mut report = empty_report(&a);
    report.pending = a.affected.iter().map(|f| f.name().to_string()).collect();
    Ok(report)
}

/// Rebuilds `features` (or everything affected when `force_all` is false).
fn run_reapply(app: &AppHandle, force_all: bool) -> AppResult<IntegrityReport> {
    let a = assess(app)?;
    let mut report = empty_report(&a);
    let targets: BTreeSet<Feature> = if force_all {
        REBUILDABLE
            .iter()
            .copied()
            .chain(a.affected.iter().copied())
            .collect()
    } else {
        a.affected.clone()
    };

    if watcher::probe_now().running && !targets.is_empty() {
        // The game locks its packages; retry at next start or manually.
        report.pending = targets.iter().map(|f| f.name().to_string()).collect();
        return Ok(report);
    }

    for feature in targets {
        match feature.reapply(app) {
            None => report.pending.push(feature.name().to_string()),
            Some(Ok(ReapplyOutcome::NothingToDo)) => {}
            Some(Ok(ReapplyOutcome::Reapplied)) => {
                report.reapplied.push(feature.name().to_string())
            }
            Some(Ok(ReapplyOutcome::NeedsUser(why))) => {
                report.pending.push(feature.name().to_string());
                report.failures.push(IntegrityFailure {
                    feature: feature.name().into(),
                    message: why,
                });
            }
            Some(Err(err)) => report.failures.push(IntegrityFailure {
                feature: feature.name().into(),
                message: err.to_string(),
            }),
        }
    }
    // Only move the baseline forward when nothing is left behind, so an
    // interrupted rebuild is detected again next time.
    if report.failures.is_empty() && report.pending.iter().all(|p| p == Feature::Stats.name()) {
        remember_build(a.current_build)?;
    }
    Ok(report)
}

pub fn reapply(app: &AppHandle) -> AppResult<IntegrityReport> {
    run_reapply(app, true)
}

fn startup_check(app: &AppHandle) -> AppResult<IntegrityReport> {
    let state: IntegrityState = fsx::read_json_or_default(&state_path()?)?;
    if state.last_build.is_none() {
        // First run with this data dir: record the baseline, nothing to compare.
        let install = install::current_install(app)?;
        remember_build(fingerprint::current(&install).map(|f| f.id()))?;
    }
    if config::current(app).auto_reapply_after_update {
        run_reapply(app, false)
    } else {
        check(app)
    }
}

pub fn emit(app: &AppHandle, report: &IntegrityReport) {
    if let Err(err) = app.emit(EVENT_REPORT, report) {
        tracing::warn!(%err, "could not emit integrity report");
    }
}

pub fn init(app: &AppHandle) -> AppResult<()> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        let worker = app.clone();
        let result = tauri::async_runtime::spawn_blocking(move || startup_check(&worker)).await;
        match result {
            Ok(Ok(report)) => {
                tracing::info!(
                    updated = report.game_updated,
                    reverted = report.files_reverted,
                    reapplied = ?report.reapplied,
                    pending = ?report.pending,
                    "integrity startup check"
                );
                emit(&app, &report);
            }
            Ok(Err(err)) => tracing::warn!(%err, "integrity startup check skipped"),
            Err(err) => tracing::error!(%err, "integrity task panicked"),
        }
    });
    Ok(())
}
