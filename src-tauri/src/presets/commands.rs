//! Presets IPC commands.

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::catalog::{self, Slot};
use crate::presets::bridge;
use crate::presets::model::{PartResult, Preset, PresetApplyReport, PresetPart};
use crate::presets::{random, share, store};
use crate::swap;

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

fn part(part: PresetPart, result: AppResult<()>) -> PartResult {
    match result {
        Ok(()) => PartResult {
            part,
            ok: true,
            applied: 1,
            errors: Vec::new(),
        },
        Err(e) => PartResult {
            part,
            ok: false,
            applied: 0,
            errors: vec![e.to_string()],
        },
    }
}

#[tauri::command]
pub fn presets_list() -> AppResult<Vec<Preset>> {
    store::list()
}

#[tauri::command]
pub fn presets_save(preset: Preset) -> AppResult<Preset> {
    store::save(preset)
}

#[tauri::command]
pub fn presets_delete(id: String) -> AppResult<()> {
    store::delete(&id)
}

/// Replaces the whole loadout: everything the app changed is restored
/// first (swaps, and the map when the preset has none; palette and decal
/// are set or restored below), then only the preset's content is applied.
/// Refused while the game runs: items can't change then, and a half-applied
/// preset would keep the previous swaps. After the clean slate, a failing
/// part never stops the others.
#[tauri::command]
pub async fn presets_apply(app: AppHandle, id: String) -> AppResult<PresetApplyReport> {
    let preset = store::get(&id)?;
    crate::game::writer::ensure_game_closed()?;

    // 1. Clean slate: no swap survives, whatever the preset contains.
    let clean_app = app.clone();
    blocking(move || swap::engine::restore_all(&clean_app).map(|_| ())).await?;
    if preset.map_id.is_none() && crate::maps::active_session().is_some() {
        crate::maps::session::deactivate(&app).await?;
    }

    let mut parts = Vec::new();

    // 2. The preset's swaps.
    let requests = preset.swaps.clone();
    let swap_app = app.clone();
    let swaps = blocking(move || {
        let mut result = PartResult {
            part: PresetPart::Swaps,
            ok: true,
            applied: 0,
            errors: Vec::new(),
        };
        for req in &requests {
            match swap::engine::apply(&swap_app, req) {
                Ok(_) => result.applied += 1,
                Err(e) => {
                    result.ok = false;
                    result
                        .errors
                        .push(format!("{:?} {}: {e}", req.slot, req.wanted_id));
                }
            }
        }
        Ok(result)
    })
    .await?;
    parts.push(swaps);

    parts.push(part(
        PresetPart::Palette,
        bridge::apply_palette(&app, preset.palette.as_ref()).await,
    ));
    parts.push(part(
        PresetPart::Decal,
        bridge::apply_decal(&app, preset.decal.as_ref()).await,
    ));
    if let Some(map_id) = &preset.map_id {
        parts.push(part(PresetPart::Map, bridge::apply_map(&app, map_id).await));
    }

    let ok = parts.iter().all(|p| p.ok);
    tracing::info!(preset = %preset.name, ok, "preset applied");
    if let Err(err) = crate::presets::active::record(&preset) {
        tracing::warn!(%err, "could not remember the applied preset");
    }
    Ok(PresetApplyReport {
        preset_id: preset.id,
        ok,
        parts,
    })
}

/// Last applied preset, if anything is still modified.
#[tauri::command]
pub async fn presets_active(
    app: AppHandle,
) -> AppResult<Option<crate::presets::active::ActivePreset>> {
    blocking(move || crate::presets::active::current(&app)).await
}

/// Saves the current state (swaps, palette, decal, map) as a new preset.
#[tauri::command]
pub fn presets_capture(app: AppHandle, name: String) -> AppResult<Preset> {
    let mut preset = Preset::empty(store::clean_name(&name)?);
    fill_from_current(&app, &mut preset)?;
    store::save(preset)
}

/// Replaces a preset's content (swaps, palette, decal, map) with the
/// current setup; its name and id are kept. It becomes the active preset.
#[tauri::command]
pub fn presets_update_from_current(app: AppHandle, id: String) -> AppResult<Preset> {
    let mut preset = store::get(&id)?;
    fill_from_current(&app, &mut preset)?;
    let preset = store::save(preset)?;
    if let Err(err) = crate::presets::active::record(&preset) {
        tracing::warn!(%err, "could not remember the updated preset");
    }
    Ok(preset)
}

fn fill_from_current(app: &AppHandle, preset: &mut Preset) -> AppResult<()> {
    preset.swaps = swap::active_requests()?;
    // Optional parts: a failure in another slice must not block the capture.
    preset.palette = bridge::current_palette(app).unwrap_or_else(|e| {
        tracing::warn!(%e, "capture: palette unavailable");
        None
    });
    preset.decal = bridge::current_decal(app).unwrap_or_else(|e| {
        tracing::warn!(%e, "capture: decal unavailable");
        None
    });
    preset.map_id = bridge::current_map(app).unwrap_or_else(|e| {
        tracing::warn!(%e, "capture: map unavailable");
        None
    });
    Ok(())
}

#[tauri::command]
pub fn presets_export_code(id: String) -> AppResult<String> {
    let preset = store::get(&id)?;
    share::encode_wire(&share::to_wire(&preset)?)
}

/// Decodes a share code and stores it as a new preset.
#[tauri::command]
pub fn presets_import_code(code: String) -> AppResult<Preset> {
    let preset = share::from_wire(share::decode_wire(&code)?);
    store::save(preset)
}

/// Unsaved random preset for the given slots (owned items = last used per slot).
#[tauri::command]
pub async fn presets_random(app: AppHandle, slots: Vec<Slot>) -> AppResult<Preset> {
    blocking(move || {
        let snap = catalog::snapshot(&app)?;
        let owned = swap::owned_defaults()?;
        let loadout =
            random::random_loadout(&snap, &slots, &owned, &mut random::Rng::from_entropy());
        if loadout.swaps.is_empty() {
            return Err(AppError::InvalidInput(
                "no owned item known for these slots yet — apply one swap per slot first".into(),
            ));
        }
        let mut preset =
            Preset::empty(format!("Random · {}", chrono::Local::now().format("%H:%M")));
        preset.swaps = loadout.swaps;
        Ok(preset)
    })
    .await
}
