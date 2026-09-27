//! The product list, built from the game's own localisation (`game_db`):
//! every product with its English and French label, so the catalog follows
//! each season without an app update. Nothing is bundled with the app.
//!
//! Products are identified by a stable id derived from their asset name
//! ([`game_db::synthetic_id`]); Psyonix's numeric product ids are not in
//! the local game files.

use crate::catalog::game_db::{slot_from_asset, synthetic_id, GameProduct};
use crate::catalog::model::Slot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub id: u32,
    pub slot: Slot,
    pub asset: String,
    pub label_fr: String,
    /// Real English label (from the game); `None` → humanised asset name.
    pub label_en: Option<String>,
}

/// Products of an exposed slot, one per asset (first occurrence wins).
pub fn from_game(game: Vec<GameProduct>) -> Vec<Product> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for g in game {
        let Some(slot) = slot_from_asset(&g.asset) else {
            continue;
        };
        if !seen.insert(g.asset.to_ascii_lowercase()) {
            continue;
        }
        // Some products ship without a display name: keep them, the
        // catalog falls back to the humanised asset name.
        let label_en = Some(g.label_en).filter(|l| !l.is_empty());
        let label_fr = g
            .label_fr
            .filter(|l| !l.is_empty())
            .or_else(|| label_en.clone())
            .unwrap_or_else(|| humanize(&g.asset));
        out.push(Product {
            id: synthetic_id(&g.asset),
            slot,
            label_fr,
            label_en,
            asset: g.asset,
        });
    }
    out
}

/// `WHEEL_AlphaRim` → `Alpha Rim`; `skin_octane_flames` → `Octane Flames`.
pub fn humanize(asset: &str) -> String {
    const PREFIXES: [&str; 12] = [
        "wheel_",
        "skin_",
        "body_",
        "boost_",
        "hat_",
        "antenna_",
        "explosion_",
        "ss_",
        "paintfinish_",
        "flag_",
        "goalexplosion_",
        "topper_",
    ];
    let lower = asset.to_ascii_lowercase();
    let rest = PREFIXES
        .iter()
        .find(|p| lower.starts_with(*p))
        .map(|p| &asset[p.len()..])
        .unwrap_or(asset);
    let mut words: Vec<String> = Vec::new();
    for part in rest.split('_').filter(|p| !p.is_empty()) {
        // Split camelCase: "AlphaRim" -> "Alpha Rim".
        let mut word = String::new();
        let chars: Vec<char> = part.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            let boundary = i > 0 && c.is_uppercase() && chars[i - 1].is_lowercase();
            if boundary && !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            word.push(*c);
        }
        if !word.is_empty() {
            words.push(word);
        }
    }
    words
        .into_iter()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(asset: &str, en: &str, fr: Option<&str>) -> GameProduct {
        GameProduct {
            asset: asset.into(),
            label_en: en.into(),
            label_fr: fr.map(str::to_string),
        }
    }

    #[test]
    fn builds_products_from_game_labels() {
        let products = from_game(vec![
            game("antenna_8ball", "8-Ball", Some("Boule n°8")),
            game("wheel_zigzag", "Zig Zag", None),
            game("PlayerBanner_New", "Banner", None),
            game("boost_alphadevreward", "", Some("")),
            game("WHEEL_ZigZag", "Duplicate", None),
        ]);
        assert_eq!(
            products.len(),
            3,
            "banners are not exposed, duplicates dropped"
        );
        assert_eq!(products[0].label_fr, "Boule n°8");
        assert_eq!(products[0].label_en.as_deref(), Some("8-Ball"));
        assert_eq!(products[0].id, synthetic_id("antenna_8ball"));
        assert_eq!(products[1].slot, Slot::Wheels);
        assert_eq!(
            products[1].label_fr, "Zig Zag",
            "English label when no French one"
        );
        assert_eq!(
            products[2].label_fr, "Alphadevreward",
            "asset name when unnamed"
        );
        assert_eq!(products[2].label_en, None);
    }

    #[test]
    fn humanizes_asset_names() {
        assert_eq!(humanize("WHEEL_AlphaRim"), "Alpha Rim");
        assert_eq!(humanize("skin_octane_flames"), "Octane Flames");
        assert_eq!(humanize("Antenna_8Ball"), "8Ball");
    }
}
