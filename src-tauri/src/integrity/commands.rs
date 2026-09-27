//! Integrity IPC commands.

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::integrity::{self, IntegrityReport};

/// Read-only report of what a rebuild would touch.
#[tauri::command]
pub async fn integrity_check(app: AppHandle) -> AppResult<IntegrityReport> {
    tauri::async_runtime::spawn_blocking(move || integrity::check(&app))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

/// Rebuilds every feature from current stock files and emits the report.
#[tauri::command]
pub async fn integrity_reapply(app: AppHandle) -> AppResult<IntegrityReport> {
    let worker = app.clone();
    let report = tauri::async_runtime::spawn_blocking(move || integrity::reapply(&worker))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
    integrity::emit(&app, &report);
    Ok(report)
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanFailure {
    pub feature: String,
    pub message: String,
}

/// Result of "restore the stock game".
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    /// Item swaps that were restored.
    pub swaps: u32,
    pub palette: bool,
    pub decal: bool,
    pub ball: bool,
    pub map: bool,
    /// Files still listed in the write manifest and restored by the final sweep.
    pub leftovers: u32,
    pub failures: Vec<CleanFailure>,
}

/// Puts the game back to stock: every swap, the palette, the custom decal
/// and the active map are removed, then any file still listed in the write
/// manifest is restored. Steps are independent — one failure does not stop
/// the others — and nothing outside what the app wrote is touched.
#[tauri::command]
pub async fn integrity_restore_stock(app: AppHandle) -> AppResult<CleanReport> {
    use crate::game::writer::{self, Owner};

    writer::ensure_game_closed()?;
    let mut failures = Vec::new();
    let mut fail = |feature: &str, err: AppError| {
        tracing::warn!(feature, %err, "restore stock step failed");
        failures.push(CleanFailure {
            feature: feature.to_string(),
            message: err.to_string(),
        });
    };

    let swaps = crate::swap::commands::swap_restore_all(app.clone())
        .await
        .unwrap_or_else(|e| {
            fail("swap", e);
            0
        });
    let palette = crate::palette::commands::palette_restore(app.clone())
        .await
        .map_err(|e| fail("palette", e))
        .is_ok();
    let decal = crate::decals::commands::decals_remove(app.clone())
        .await
        .map_err(|e| fail("decals", e))
        .is_ok();
    let ball = crate::ball::commands::ball_remove(app.clone())
        .await
        .map_err(|e| fail("ball", e))
        .is_ok();
    let map = crate::maps::commands::maps_deactivate(app.clone())
        .await
        .map_err(|e| fail("maps", e))
        .is_ok();

    // Safety net: whatever a feature's own state may have missed.
    let sweep = app.clone();
    let leftovers = tauri::async_runtime::spawn_blocking(move || {
        let mut restored = 0u32;
        let mut errors = Vec::new();
        for owner in [
            Owner::Swap,
            Owner::Palette,
            Owner::Decals,
            Owner::Ball,
            Owner::Maps,
            Owner::Stats,
        ] {
            let pending = writer::entries_for(owner).map(|e| e.len()).unwrap_or(0);
            if pending == 0 {
                continue;
            }
            match writer::restore_owner(&sweep, owner, None) {
                Ok(outcomes) => restored += outcomes.len() as u32,
                Err(err) => errors.push((format!("{owner:?}").to_lowercase(), err)),
            }
        }
        (restored, errors)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    let (leftovers, sweep_errors) = leftovers;
    for (feature, err) in sweep_errors {
        fail(&feature, err);
    }

    tracing::info!(
        swaps,
        palette,
        decal,
        ball,
        map,
        leftovers,
        failures = failures.len(),
        "game restored to stock"
    );
    Ok(CleanReport {
        swaps,
        palette,
        decal,
        ball,
        map,
        leftovers,
        failures,
    })
}
