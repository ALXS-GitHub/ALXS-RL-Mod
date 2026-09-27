//! Product → package resolution against the user's `CookedPCConsole`.
//!
//! The product database names assets (`Explosion_Basic_Green`) while the game
//! ships packages (`explosion_basic_SF.upk`). Measured on a Season 24 install
//! (Sept 2026): 6 305 / 6 583 products of exposed slots resolve (95.8 %);
//! the rest are simply not on disk (event/mystery items, streamed content).
//!
//! Strategy: try `<asset>_SF.upk` then `<asset>.upk` (case-insensitive), then
//! strip one known variant suffix at a time (tier, colour, rank, `_SE`,
//! trailing digits…) and retry, at most 4 times. Thumbnail companions
//! (`*_T_SF.upk`) are never taken as the main package.

use std::collections::HashMap;

/// Case-insensitive index of the package files in `CookedPCConsole`.
#[derive(Debug, Default)]
pub struct PackageIndex {
    by_lower: HashMap<String, String>,
}

impl PackageIndex {
    pub fn from_names<I: IntoIterator<Item = String>>(names: I) -> Self {
        let by_lower = names
            .into_iter()
            .filter(|n| n.to_ascii_lowercase().ends_with(".upk"))
            .map(|n| (n.to_ascii_lowercase(), n))
            .collect();
        Self { by_lower }
    }

    pub fn scan(dir: &std::path::Path) -> std::io::Result<Self> {
        let names = std::fs::read_dir(dir)?
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().into_owned());
        Ok(Self::from_names(names))
    }

    pub fn get(&self, file_name: &str) -> Option<&str> {
        self.by_lower
            .get(&file_name.to_ascii_lowercase())
            .map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.by_lower.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_lower.is_empty()
    }
}

/// Trailing `_segment`s that denote a variant of the same package.
const VARIANT_TOKENS: &[&str] = &[
    "tier",
    "online",
    "pa",
    "se",
    "lte",
    "free",
    "sr",
    "anodized",
    "multichrome",
    "premium",
    "infinite",
    "uc",
    "nr",
    "t1",
    "t2",
    "t3",
    "green",
    "orange",
    "pink",
    "purple",
    "red",
    "blue",
    "yellow",
    "white",
    "black",
    "grey",
    "gray",
    "lime",
    "crimson",
    "cobalt",
    "saffron",
    "bronze",
    "silver",
    "gold",
    "platinum",
    "diamond",
    "champion",
    "grandchampion",
    "supersonic",
    "legend",
];

fn is_variant_token(seg: &str) -> bool {
    let s = seg.to_ascii_lowercase();
    VARIANT_TOKENS.contains(&s.as_str())
        || (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        || s.strip_prefix("tier")
            .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        || s.strip_prefix("number")
            .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Removes trailing ASCII digits when at least 3 letters remain
/// (`wheel_treble1` → `wheel_treble`, `skin_jpeggy2` → `skin_jpeggy`).
fn strip_glued_digits(seg: &str) -> Option<&str> {
    let trimmed = seg.trim_end_matches(|c: char| c.is_ascii_digit());
    (trimmed.len() != seg.len()
        && trimmed.len() > 2
        && trimmed.ends_with(|c: char| c.is_ascii_alphabetic()))
    .then_some(trimmed)
}

/// One stripping step, or `None` when nothing more can be stripped.
pub fn strip_variant(asset: &str) -> Option<String> {
    let parts: Vec<&str> = asset.split('_').collect();
    let last = *parts.last()?;
    if parts.len() >= 2 && is_variant_token(last) {
        let mut rest = &parts[..parts.len() - 1];
        // "tier_01": drop the dangling "tier" too.
        if rest.last().is_some_and(|s| s.eq_ignore_ascii_case("tier")) {
            rest = &rest[..rest.len() - 1];
        }
        // Keep at least `<Kind>_<Name>`: collapsing to a bare kind prefix
        // ("Skin", "Boost") would match an unrelated package.
        if rest.len() < 2 {
            return None;
        }
        return Some(rest.join("_"));
    }
    let glued = strip_glued_digits(last)?;
    let mut owned: Vec<&str> = parts[..parts.len() - 1].to_vec();
    owned.push(glued);
    Some(owned.join("_"))
}

/// Package file for `asset`, if any.
pub fn resolve(index: &PackageIndex, asset: &str) -> Option<String> {
    let mut current = asset.to_string();
    for _ in 0..5 {
        for candidate in [format!("{current}_SF.upk"), format!("{current}.upk")] {
            if candidate.to_ascii_lowercase().ends_with("_t_sf.upk") {
                continue;
            }
            if let Some(real) = index.get(&candidate) {
                return Some(real.to_string());
            }
        }
        current = strip_variant(&current)?;
    }
    None
}

/// `WHEEL_AlphaRim_SF.upk` → `WHEEL_AlphaRim_T_SF.upk` when it exists.
pub fn thumbnail_for(index: &PackageIndex, package: &str) -> Option<String> {
    let base = package_base(package);
    index.get(&format!("{base}_T_SF.upk")).map(str::to_string)
}

/// File name without `.upk` and without the `_SF` suffix.
pub fn package_base(package: &str) -> &str {
    let stem = package_stem(package);
    let lower = stem.to_ascii_lowercase();
    if lower.ends_with("_sf") {
        &stem[..stem.len() - 3]
    } else {
        stem
    }
}

/// File name without `.upk`.
pub fn package_stem(package: &str) -> &str {
    let lower = package.to_ascii_lowercase();
    if lower.ends_with(".upk") {
        &package[..package.len() - 4]
    } else {
        package
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> PackageIndex {
        PackageIndex::from_names(
            [
                "WHEEL_AlphaRim_SF.upk",
                "WHEEL_AlphaRim_T_SF.upk",
                "explosion_basic_SF.upk",
                "ss_tier_01_SF.upk",
                "body_unicorn_SF.upk",
                "wheel_treble_SF.upk",
                "Skin_Octane_Flames_SF.upk",
                "Only_T_SF.upk",
                "readme.txt",
            ]
            .map(String::from),
        )
    }

    #[test]
    fn exact_and_case_insensitive() {
        let i = index();
        assert_eq!(
            resolve(&i, "WHEEL_AlphaRim").as_deref(),
            Some("WHEEL_AlphaRim_SF.upk")
        );
        assert_eq!(
            resolve(&i, "wheel_alpharim").as_deref(),
            Some("WHEEL_AlphaRim_SF.upk")
        );
        assert_eq!(
            resolve(&i, "skin_octane_flames").as_deref(),
            Some("Skin_Octane_Flames_SF.upk")
        );
    }

    #[test]
    fn strips_known_variants() {
        let i = index();
        assert_eq!(
            resolve(&i, "Explosion_Basic_Green").as_deref(),
            Some("explosion_basic_SF.upk")
        );
        assert_eq!(
            resolve(&i, "SS_Tier_01_Bronze").as_deref(),
            Some("ss_tier_01_SF.upk")
        );
        assert_eq!(
            resolve(&i, "body_unicorn_tier2").as_deref(),
            Some("body_unicorn_SF.upk")
        );
        assert_eq!(
            resolve(&i, "wheel_treble1").as_deref(),
            Some("wheel_treble_SF.upk")
        );
    }

    #[test]
    fn never_strips_meaningful_segments() {
        // "Dragon" is not a variant token: must not fall back to a wrong package.
        let i = PackageIndex::from_names(["Skin_Octane_SF.upk".to_string()]);
        assert_eq!(resolve(&i, "Skin_Octane_Dragon"), None);
    }

    #[test]
    fn thumbnails_are_not_main_packages() {
        let i = index();
        assert_eq!(resolve(&i, "Only"), None);
        assert_eq!(
            thumbnail_for(&i, "WHEEL_AlphaRim_SF.upk").as_deref(),
            Some("WHEEL_AlphaRim_T_SF.upk")
        );
        assert_eq!(package_base("WHEEL_AlphaRim_SF.upk"), "WHEEL_AlphaRim");
    }

    #[test]
    fn strip_variant_steps() {
        assert_eq!(
            strip_variant("hat_pixel_02_SR").as_deref(),
            Some("hat_pixel_02")
        );
        assert_eq!(strip_variant("hat_pixel_02").as_deref(), Some("hat_pixel"));
        assert_eq!(
            strip_variant("Explosion_Techy_Tier_01").as_deref(),
            Some("Explosion_Techy")
        );
        assert_eq!(strip_variant("Octane"), None);
    }
}
