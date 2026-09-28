//! Swap execution: build the renamed package from **current stock** bytes and
//! hand it to the write layer.

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::catalog::{self, resolver, CatalogItem, CatalogSnapshot};
use crate::game::writer::{self, Owner, Placement};
use crate::game::{fingerprint, install, ReapplyOutcome};
use crate::swap::rules;
use crate::swap::state::{self, ActiveSwap, SwapEventKind, SwapRequest};

fn lookup<'a>(snap: &'a CatalogSnapshot, id: u32, role: &str) -> AppResult<&'a CatalogItem> {
    snap.get(id)
        .ok_or_else(|| AppError::NotFound(format!("{role} item {id} is not in the catalog")))
}

/// Stock bytes of a package, even when that package is itself currently
/// overwritten by another swap (the write layer serves the verified backup).
fn stock_bytes(install: &install::RlInstall, package: &str) -> AppResult<Vec<u8>> {
    writer::stock_bytes(install, &install.cooked_dir.join(package))
}

/// Validates the request against the catalog.
pub fn validate<'a>(
    snap: &'a CatalogSnapshot,
    req: &SwapRequest,
) -> AppResult<(&'a CatalogItem, &'a CatalogItem)> {
    let owned = lookup(snap, req.owned_id, "owned")?;
    let wanted = lookup(snap, req.wanted_id, "wanted")?;
    if owned.slot != req.slot || wanted.slot != req.slot {
        return Err(AppError::InvalidInput(format!(
            "slot mismatch: request {:?}, owned {:?}, wanted {:?}",
            req.slot, owned.slot, wanted.slot
        )));
    }
    if req.paint.is_some_and(|p| p as usize >= rules::PAINTS.len()) {
        return Err(AppError::InvalidInput(
            "paint must be between 0 and 12".into(),
        ));
    }
    if req.tint.is_some_and(|h| h >= 360) {
        return Err(AppError::InvalidInput(
            "tint must be a hue between 0 and 359".into(),
        ));
    }
    // The same package is fine when recolouring: the item replaces itself.
    if owned.package.eq_ignore_ascii_case(&wanted.package)
        && req.paint.unwrap_or(0) == 0
        && req.tint.is_none()
    {
        return Err(AppError::Conflict(format!(
            "{} and {} live in the same package ({})",
            owned.label_en, wanted.label_en, owned.package
        )));
    }
    Ok((owned, wanted))
}

/// Builds and writes one swap. `id` is reused when re-applying.
pub fn apply_with_id(app: &AppHandle, req: &SwapRequest, id: &str) -> AppResult<ActiveSwap> {
    let snap = catalog::snapshot(app)?;
    let (owned, wanted) = validate(&snap, req)?;
    let install = install::current_install(app)?;
    let index = resolver::PackageIndex::scan(&install.cooked_dir)?;

    let painted = req
        .paint
        .filter(|p| *p > 0)
        .and_then(|p| rules::painted_variant(&index, &wanted.package, p));
    let source_package = painted.clone().unwrap_or_else(|| wanted.package.clone());
    if source_package.eq_ignore_ascii_case(&owned.package) && req.tint.is_none() {
        return Err(AppError::Conflict(format!(
            "no painted variant of {} on disk — the swap would change nothing",
            owned.package
        )));
    }

    let bytes = stock_bytes(&install, &source_package)?;
    let renames = rules::derive_rules(req.slot, &source_package, &owned.package);
    let keys = crate::upk::keys::ring(app);
    if keys.is_empty() {
        return Err(AppError::KeysMissing);
    }
    // The game decrypts the file with the key of the name it loads: the
    // owned item's. Without that key the swapped file would be unreadable.
    let owned_stock = stock_bytes(&install, &owned.package)?;
    if !crate::upk::can_decrypt(&owned_stock, &keys) {
        return Err(AppError::Unsupported(format!(
            "{} is encrypted with a key missing from keys.txt",
            owned.package
        )));
    }
    let target_key = crate::upk::rename::package_key(&owned_stock, &keys);
    let mut out = if source_package.eq_ignore_ascii_case(&owned.package) {
        bytes
    } else {
        crate::upk::rename::rename_package(&bytes, &renames, &keys, target_key)?
    };
    if let Some(hue) = req.tint {
        out = recolor(out, &keys, f32::from(hue))?;
    }

    let target = install.cooked_dir.join(&owned.package);
    writer::install_file(app, Owner::Swap, id, &target, Placement::RootReplace, &out)?;

    Ok(ActiveSwap {
        id: id.to_string(),
        request: req.clone(),
        owned_label: owned.label_fr.clone(),
        wanted_label: wanted.label_fr.clone(),
        target_package: owned.package.clone(),
        source_package,
        painted: painted.is_some(),
        applied_at: chrono::Utc::now(),
        build: fingerprint::current(&install)
            .map(|f| f.id())
            .unwrap_or_default(),
    })
}

/// Moves every data colour of the package to `hue` (experimental).
fn recolor(bytes: Vec<u8>, keys: &crate::upk::KeyRing, hue: f32) -> AppResult<Vec<u8>> {
    let pkg = crate::upk::Package::open(bytes, keys)?;
    let body = pkg.body()?;
    let (patch, stats) = crate::upk::recolor::recolor_patch(&pkg, &body, hue);
    if stats.colors == 0 {
        return Err(AppError::Unsupported(
            "this item has no colour the app can change".into(),
        ));
    }
    let mut out = pkg.header_bytes()?;
    patch.apply(&mut out, &body.map)?;
    pkg.seal(&mut out)?;
    tracing::info!(
        colors = stats.colors,
        exports = stats.exports,
        hue,
        "item recoloured"
    );
    Ok(out)
}

/// Applies a new swap, replacing any active swap on the same owned package.
pub fn apply(app: &AppHandle, req: &SwapRequest) -> AppResult<ActiveSwap> {
    let id = uuid::Uuid::new_v4().to_string();
    let swap = apply_with_id(app, req, &id)?;
    state::update(|s| {
        // The write layer kept the original backup; the old swap is superseded.
        s.active
            .retain(|a| !a.target_package.eq_ignore_ascii_case(&swap.target_package));
        s.active.push(swap.clone());
        s.owned_defaults.insert(req.slot, req.owned_id);
        s.push_event(&swap, SwapEventKind::Applied, None);
        Ok(())
    })?;
    Ok(swap)
}

pub fn restore(app: &AppHandle, id: &str) -> AppResult<()> {
    let current = state::load()?;
    let swap = current
        .active
        .iter()
        .find(|a| a.id == id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("swap {id}")))?;
    writer::restore_owner(app, Owner::Swap, Some(id))?;
    state::update(|s| {
        s.active.retain(|a| a.id != id);
        s.push_event(&swap, SwapEventKind::Restored, None);
        Ok(())
    })
}

pub fn restore_all(app: &AppHandle) -> AppResult<u32> {
    writer::restore_owner(app, Owner::Swap, None)?;
    state::update(|s| {
        let restored = std::mem::take(&mut s.active);
        for swap in &restored {
            s.push_event(swap, SwapEventKind::Restored, None);
        }
        Ok(restored.len() as u32)
    })
}

/// Rebuilds swaps whose file the game replaced (update or "verify files").
pub fn reapply_after_update(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    let audits = writer::audit(app)?;
    let active = state::load()?.active;
    if active.is_empty() {
        return Ok(ReapplyOutcome::NothingToDo);
    }
    let mut rebuilt = Vec::new();
    let mut dropped = Vec::new();
    for swap in &active {
        let entries: Vec<_> = audits
            .iter()
            .filter(|a| a.entry.owner == Owner::Swap && a.entry.tag == swap.id)
            .collect();
        let intact = !entries.is_empty()
            && entries
                .iter()
                .all(|a| a.state == writer::EntryState::Intact);
        if intact {
            continue;
        }
        let stale: Vec<String> = entries.iter().map(|a| a.entry.rel_path.clone()).collect();
        if !stale.is_empty() {
            writer::forget(Owner::Swap, &stale)?;
        }
        match apply_with_id(app, &swap.request, &swap.id) {
            Ok(fresh) => rebuilt.push(fresh),
            Err(err) => {
                tracing::warn!(swap = %swap.id, %err, "swap could not be re-applied");
                dropped.push((swap.clone(), err.to_string()));
            }
        }
    }
    if rebuilt.is_empty() && dropped.is_empty() {
        return Ok(ReapplyOutcome::NothingToDo);
    }
    let dropped_count = dropped.len();
    state::update(|s| {
        for fresh in &rebuilt {
            if let Some(slot) = s.active.iter_mut().find(|a| a.id == fresh.id) {
                *slot = fresh.clone();
            }
            s.push_event(fresh, SwapEventKind::Reapplied, None);
        }
        for (swap, why) in &dropped {
            s.active.retain(|a| a.id != swap.id);
            s.push_event(swap, SwapEventKind::Dropped, Some(why.clone()));
        }
        Ok(())
    })?;
    if dropped_count > 0 {
        return Ok(ReapplyOutcome::NeedsUser(format!(
            "{dropped_count} swap(s) could not be rebuilt"
        )));
    }
    Ok(ReapplyOutcome::Reapplied)
}
