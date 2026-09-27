//! Coherent random loadouts.
//!
//! The player equips their "owned" items in game; a random preset picks a
//! different "wanted" item per slot. Decals are chosen among those that fit
//! the body that will be *displayed* (the random body, or the owned body if
//! the body slot isn't randomised), or universal decals.

use std::collections::BTreeMap;

use crate::catalog::{CatalogItem, CatalogSnapshot, Slot};
use crate::swap::SwapRequest;

/// Small deterministic PRNG (SplitMix64) — no extra dependency, testable.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn from_entropy() -> Self {
        Self(uuid::Uuid::new_v4().as_u128() as u64)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            return None;
        }
        let i = (self.next_u64() % items.len() as u64) as usize;
        items.get(i)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomLoadout {
    pub swaps: Vec<SwapRequest>,
    /// Requested slots skipped because no owned item is known for them.
    pub skipped: Vec<Slot>,
}

pub fn random_loadout(
    snap: &CatalogSnapshot,
    slots: &[Slot],
    owned: &BTreeMap<Slot, u32>,
    rng: &mut Rng,
) -> RandomLoadout {
    // Body first: the decal choice depends on it.
    let mut ordered: Vec<Slot> = slots.to_vec();
    ordered.sort();
    ordered.dedup();

    let mut swaps = Vec::new();
    let mut skipped = Vec::new();
    let mut displayed_body = owned.get(&Slot::Body).copied();

    for slot in ordered {
        let Some(&owned_id) = owned.get(&slot) else {
            skipped.push(slot);
            continue;
        };
        let owned_package = snap.get(owned_id).map(|i| i.package.to_ascii_lowercase());
        let candidates: Vec<&CatalogItem> = snap
            .by_slot(slot)
            .filter(|i| {
                !i.locked
                    && i.id != owned_id
                    && Some(i.package.to_ascii_lowercase()) != owned_package
            })
            .filter(|i| slot != Slot::Decal || i.body_id.is_none() || i.body_id == displayed_body)
            .collect();
        let Some(pick) = rng.pick(&candidates) else {
            skipped.push(slot);
            continue;
        };
        if slot == Slot::Body {
            displayed_body = Some(pick.id);
        }
        swaps.push(SwapRequest {
            slot,
            owned_id,
            wanted_id: pick.id,
            paint: None,
        });
    }
    RandomLoadout { swaps, skipped }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SlotCount;

    fn item(id: u32, slot: Slot, package: &str, body_id: Option<u32>) -> CatalogItem {
        CatalogItem {
            id,
            slot,
            asset: package.into(),
            package: format!("{package}_SF.upk"),
            thumbnail_package: None,
            label_fr: package.into(),
            label_en: package.into(),
            body_id,
            shared_package: false,
            locked: false,
        }
    }

    fn snap() -> CatalogSnapshot {
        CatalogSnapshot {
            items: vec![
                item(1, Slot::Body, "Body_Octane", None),
                item(2, Slot::Body, "Body_Dominus", None),
                item(3, Slot::Body, "Body_Fennec", None),
                item(10, Slot::Decal, "Skin_Octane_A", Some(1)),
                item(11, Slot::Decal, "Skin_Dominus_A", Some(2)),
                item(12, Slot::Decal, "Skin_Fennec_A", Some(3)),
                item(13, Slot::Decal, "Skin_Universal", None),
                item(14, Slot::Decal, "Skin_Octane_B", Some(1)),
                item(20, Slot::Wheels, "Wheel_A", None),
                item(21, Slot::Wheels, "Wheel_B", None),
            ],
            counts: Vec::<SlotCount>::new(),
            unresolved: 0,
            build: String::new(),
            from_game: false,
            generated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn decals_always_fit_the_displayed_body() {
        let s = snap();
        let owned = BTreeMap::from([(Slot::Body, 1), (Slot::Decal, 10), (Slot::Wheels, 20)]);
        for seed in 0..200 {
            let out = random_loadout(
                &s,
                &[Slot::Decal, Slot::Body, Slot::Wheels],
                &owned,
                &mut Rng::new(seed),
            );
            let body = out
                .swaps
                .iter()
                .find(|r| r.slot == Slot::Body)
                .unwrap()
                .wanted_id;
            let decal = out
                .swaps
                .iter()
                .find(|r| r.slot == Slot::Decal)
                .map(|r| s.get(r.wanted_id).unwrap());
            if let Some(d) = decal {
                assert!(
                    d.body_id.is_none() || d.body_id == Some(body),
                    "seed {seed}"
                );
            }
            assert_ne!(body, 1, "never the owned item itself");
        }
    }

    #[test]
    fn unknown_owned_slots_are_skipped() {
        let s = snap();
        let owned = BTreeMap::from([(Slot::Wheels, 20)]);
        let out = random_loadout(&s, &[Slot::Body, Slot::Wheels], &owned, &mut Rng::new(7));
        assert_eq!(out.skipped, vec![Slot::Body]);
        assert_eq!(out.swaps.len(), 1);
        assert_eq!(out.swaps[0].wanted_id, 21);
    }

    #[test]
    fn body_not_randomised_uses_owned_body_for_decals() {
        let s = snap();
        let owned = BTreeMap::from([(Slot::Body, 2), (Slot::Decal, 11)]);
        for seed in 0..50 {
            let out = random_loadout(&s, &[Slot::Decal], &owned, &mut Rng::new(seed));
            let d = s.get(out.swaps[0].wanted_id).unwrap();
            assert!(d.body_id.is_none() || d.body_id == Some(2));
        }
    }
}
