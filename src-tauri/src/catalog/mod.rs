//! Local item catalog: the game's own product list (read from its
//! localisation archive, so every new season appears automatically),
//! resolved against the packages actually present in the user's install.
//! Nothing is bundled and there is no network, ever.
//!
//! The snapshot is cached in memory and rebuilt when the game build
//! changes (or on `catalog_refresh`).

pub mod commands;
pub mod game_db;
pub mod model;
pub mod resolver;
pub mod source;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::game::{fingerprint, install};

pub use model::{CatalogItem, CatalogSnapshot, Slot, SlotCount};

static CACHE: RwLock<Option<Arc<CatalogSnapshot>>> = RwLock::new(None);

pub fn init(app: &AppHandle) -> AppResult<()> {
    // Warm the cache in the background so the Items page opens instantly.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(err) = rebuild(&app) {
            tracing::warn!(%err, "catalog warm-up skipped");
        }
    });
    Ok(())
}

/// Current snapshot; rebuilt transparently if the game build changed, or
/// if it was built without keys and keys are now available (imported or
/// dropped in since).
pub fn snapshot(app: &AppHandle) -> AppResult<Arc<CatalogSnapshot>> {
    let install = install::current_install(app)?;
    let build = fingerprint::current(&install)
        .map(|f| f.id())
        .unwrap_or_default();
    if let Ok(guard) = CACHE.read() {
        if let Some(snap) = guard.as_ref() {
            let stale = !snap.from_game && !crate::upk::keys::ring(app).is_empty();
            if snap.build == build && !stale {
                return Ok(Arc::clone(snap));
            }
        }
    }
    rebuild(app)
}

/// Forces a rescan of the install.
pub fn rebuild(app: &AppHandle) -> AppResult<Arc<CatalogSnapshot>> {
    let install = install::current_install(app)?;
    let build = fingerprint::current(&install)
        .map(|f| f.id())
        .unwrap_or_default();
    let ring = crate::upk::keys::ring(app);
    let game = game_db::read(&install.cooked_dir, &ring);
    let from_game = game.is_some();
    if !from_game {
        tracing::warn!("game localisation unreadable (keys.txt missing?) — catalog empty");
    }
    let products = source::from_game(game.unwrap_or_default());
    let index = resolver::PackageIndex::scan(&install.cooked_dir).map_err(AppError::Io)?;
    let mut snap = build_snapshot(&products, &index, build);
    snap.from_game = from_game;
    mark_locked(&mut snap, &install.cooked_dir, &ring);
    let snap = Arc::new(snap);
    tracing::info!(
        items = snap.items.len(),
        unresolved = snap.unresolved,
        from_game,
        "catalog built"
    );
    if let Ok(mut guard) = CACHE.write() {
        *guard = Some(Arc::clone(&snap));
    }
    Ok(snap)
}

/// Flags items whose package no key can open (checked once per package).
fn mark_locked(snap: &mut CatalogSnapshot, cooked: &std::path::Path, keys: &crate::upk::KeyRing) {
    let mut verdicts: HashMap<String, bool> = HashMap::new();
    for item in &mut snap.items {
        let key = item.package.to_ascii_lowercase();
        let locked = *verdicts
            .entry(key)
            .or_insert_with(|| !package_opens(&cooked.join(&item.package), keys));
        item.locked = locked;
    }
    let locked = snap.items.iter().filter(|i| i.locked).count();
    if locked > 0 {
        tracing::info!(locked, "catalog items without a matching AES key");
    }
}

/// Reads only the package header (summary + tables) and tries the key ring.
fn package_opens(path: &std::path::Path, keys: &crate::upk::KeyRing) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    // TotalHeaderSize sits at bytes 8..12; read it first so the summary is
    // parsed on a buffer that really holds the whole header. The encrypted
    // region is rounded up to the AES block size and can end a few bytes
    // past it, hence the margin.
    let mut fixed = [0u8; 12];
    if file.read_exact(&mut fixed).is_err() {
        return false;
    }
    let total = u32::from_le_bytes([fixed[8], fixed[9], fixed[10], fixed[11]]) as usize;
    if total == 0 || total > 64 * 1024 * 1024 {
        return false;
    }
    let mut buf = fixed.to_vec();
    if file
        .take((total + 4096 - fixed.len()) as u64)
        .read_to_end(&mut buf)
        .is_err()
    {
        return false;
    }
    crate::upk::can_decrypt(&buf, keys)
}

/// Pure part of the build (unit-tested).
pub fn build_snapshot(
    products: &[source::Product],
    index: &resolver::PackageIndex,
    build: String,
) -> CatalogSnapshot {
    let mut items = Vec::with_capacity(products.len());
    let mut unresolved = 0u32;
    for p in products {
        match resolver::resolve(index, &p.asset) {
            Some(package) => items.push(CatalogItem {
                id: p.id,
                slot: p.slot,
                thumbnail_package: resolver::thumbnail_for(index, &package),
                package,
                asset: p.asset.clone(),
                label_en: p
                    .label_en
                    .clone()
                    .unwrap_or_else(|| source::humanize(&p.asset)),
                label_fr: if p.label_fr.is_empty() {
                    source::humanize(&p.asset)
                } else {
                    p.label_fr.clone()
                },
                body_id: None,
                shared_package: false,
                locked: false,
            }),
            None => unresolved += 1,
        }
    }

    // Shared packages: several products resolved to the same file.
    let mut per_package: HashMap<String, u32> = HashMap::new();
    for i in &items {
        *per_package
            .entry(i.package.to_ascii_lowercase())
            .or_default() += 1;
    }
    for i in &mut items {
        i.shared_package = per_package
            .get(&i.package.to_ascii_lowercase())
            .copied()
            .unwrap_or(0)
            > 1;
    }

    link_decals_to_bodies(&mut items);

    let mut counts: Vec<SlotCount> = Slot::ALL
        .iter()
        .map(|s| SlotCount {
            slot: *s,
            count: items.iter().filter(|i| i.slot == *s).count() as u32,
        })
        .collect();
    counts.retain(|c| c.count > 0);
    items.sort_by(|a, b| {
        a.slot
            .cmp(&b.slot)
            .then_with(|| a.label_fr.to_lowercase().cmp(&b.label_fr.to_lowercase()))
    });

    CatalogSnapshot {
        items,
        counts,
        unresolved,
        build,
        from_game: false,
        generated_at: chrono::Utc::now(),
    }
}

/// Decal → body: first by label (`"Octane : Flammes"` → body labelled
/// `Octane`), then by asset token (`Skin_Octane_Flames` → `Body_Octane`).
/// Decals matching neither are universal (`body_id = None`).
fn link_decals_to_bodies(items: &mut [CatalogItem]) {
    let mut by_label: HashMap<String, u32> = HashMap::new();
    let mut by_token: HashMap<String, u32> = HashMap::new();
    for b in items.iter().filter(|i| i.slot == Slot::Body) {
        by_label
            .entry(b.label_fr.trim().to_lowercase())
            .or_insert(b.id);
        if let Some((_, token)) = b.asset.split_once('_') {
            by_token.entry(token.to_ascii_lowercase()).or_insert(b.id);
        }
    }
    for d in items.iter_mut().filter(|i| i.slot == Slot::Decal) {
        let from_label = d
            .label_fr
            .split_once(" : ")
            .and_then(|(body, _)| by_label.get(&body.trim().to_lowercase()).copied());
        let from_token = || {
            let parts: Vec<&str> = d.asset.split('_').collect();
            (parts.len() > 2)
                .then(|| by_token.get(&parts[1].to_ascii_lowercase()).copied())
                .flatten()
        };
        d.body_id = from_label.or_else(from_token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use source::Product;

    fn p(id: u32, slot: Slot, asset: &str, fr: &str) -> Product {
        Product {
            id,
            slot,
            asset: asset.into(),
            label_fr: fr.into(),
            label_en: None,
        }
    }

    #[test]
    fn builds_links_and_counts() {
        let index = resolver::PackageIndex::from_names(
            [
                "Body_Octane_SF.upk",
                "Skin_Octane_Flames_SF.upk",
                "skin_takumi_ii_stripes_SF.upk",
                "Body_Takumi_II_SF.upk",
                "skin_heatwave_SF.upk",
                "explosion_basic_SF.upk",
            ]
            .map(String::from),
        );
        let products = vec![
            p(23, Slot::Body, "Body_Octane", "Octane"),
            p(24, Slot::Body, "Body_Takumi_II", "Takumi RX-T"),
            p(500, Slot::Decal, "Skin_Octane_Flames", "Octane : Flammes"),
            p(
                501,
                Slot::Decal,
                "skin_takumi_ii_stripes",
                "Takumi RX-T : Bandes",
            ),
            p(502, Slot::Decal, "skin_heatwave", "Vague de chaleur"),
            p(
                600,
                Slot::GoalExplosion,
                "Explosion_Basic_Green",
                "Basique (vert)",
            ),
            p(
                601,
                Slot::GoalExplosion,
                "Explosion_Basic_Pink",
                "Basique (rose)",
            ),
            p(700, Slot::Wheels, "WHEEL_Missing", "Absente"),
        ];
        let snap = build_snapshot(&products, &index, "b".into());
        assert_eq!(snap.unresolved, 1);
        assert_eq!(snap.get(500).unwrap().body_id, Some(23));
        assert_eq!(snap.get(501).unwrap().body_id, Some(24));
        assert_eq!(snap.get(502).unwrap().body_id, None);
        assert!(snap.get(600).unwrap().shared_package);
        assert!(!snap.get(23).unwrap().shared_package);
        let explosions = snap
            .counts
            .iter()
            .find(|c| c.slot == Slot::GoalExplosion)
            .unwrap();
        assert_eq!(explosions.count, 2);
    }
}
