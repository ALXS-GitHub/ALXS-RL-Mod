//! Pure planning helpers: package-name rename rules and painted variants.
//!
//! Rocket League only loads a cooked package whose internal name matches its
//! file name, so the wanted package is renamed to the owned package's name
//! before being written over it. Rules proven byte-identical to rlpeak's
//! output (see `docs_rl/` and the legacy `upk_renamer` tests):
//! - wheels, boosts and most slots: rename the base name AND the `_SF` name;
//! - decals: rename the base name only — the `_SF` FName is referenced by the
//!   package's own asset lookups and must keep resolving.

use crate::catalog::resolver::{package_base, package_stem, PackageIndex};
use crate::catalog::Slot;
use crate::upk::rename::Rename;

/// Paint ids as used by the game (0 = unpainted).
pub const PAINTS: [&str; 13] = [
    "None",
    "Crimson",
    "Lime",
    "Black",
    "Orange",
    "Sky Blue",
    "Cobalt",
    "Saffron",
    "Grey",
    "Pink",
    "Forest Green",
    "Purple",
    "Titanium White",
];

pub fn derive_rules(slot: Slot, source_package: &str, target_package: &str) -> Vec<Rename> {
    let src_base = package_base(source_package).to_string();
    let tgt_base = package_base(target_package).to_string();
    let mut rules = vec![Rename {
        source: src_base,
        target: tgt_base,
    }];
    if slot != Slot::Decal {
        let src_full = package_stem(source_package).to_string();
        let tgt_full = package_stem(target_package).to_string();
        // Packages shipped without `_SF` have a single package FName.
        if src_full != rules[0].source {
            rules.push(Rename {
                source: src_full,
                target: tgt_full,
            });
        }
    }
    rules.retain(|r| r.source != r.target);
    rules
}

/// File-name slugs the game uses for painted variants of a package.
fn paint_slugs(paint: u8) -> Vec<String> {
    let Some(name) = PAINTS.get(paint as usize).filter(|_| paint > 0) else {
        return Vec::new();
    };
    let mut slugs = vec![name.replace(' ', ""), name.replace(' ', "_")];
    let extra: &[&str] = match paint {
        5 => &["SB"],
        8 => &["Gray"],
        10 => &["FG"],
        12 => &["TW"],
        _ => &[],
    };
    slugs.extend(extra.iter().map(|s| s.to_string()));
    slugs.dedup();
    slugs
}

/// A painted variant package of `package`, if the install ships one.
/// Most items are painted at runtime and have no such file: callers fall
/// back to the unpainted package.
pub fn painted_variant(index: &PackageIndex, package: &str, paint: u8) -> Option<String> {
    let base = package_base(package);
    paint_slugs(paint).into_iter().find_map(|slug| {
        [
            format!("{base}_{slug}_SF.upk"),
            format!("{base}_{slug}.upk"),
        ]
        .into_iter()
        .find_map(|c| index.get(&c).map(str::to_string))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(rules: &[Rename]) -> Vec<(&str, &str)> {
        rules
            .iter()
            .map(|r| (r.source.as_str(), r.target.as_str()))
            .collect()
    }

    #[test]
    fn wheels_rename_base_and_sf() {
        let r = derive_rules(Slot::Wheels, "WHEEL_AlphaRim_SF.upk", "WHEEL_Vortex_SF.upk");
        assert_eq!(
            pairs(&r),
            vec![
                ("WHEEL_AlphaRim", "WHEEL_Vortex"),
                ("WHEEL_AlphaRim_SF", "WHEEL_Vortex_SF")
            ]
        );
    }

    #[test]
    fn decals_rename_base_only() {
        let r = derive_rules(
            Slot::Decal,
            "Skin_bartees_SF.upk",
            "skin_aa_flames_tierall_SF.upk",
        );
        assert_eq!(pairs(&r), vec![("Skin_bartees", "skin_aa_flames_tierall")]);
    }

    #[test]
    fn packages_without_sf_suffix() {
        let r = derive_rules(Slot::Body, "Body_Foo.upk", "Body_Bar.upk");
        assert_eq!(pairs(&r), vec![("Body_Foo", "Body_Bar")]);
    }

    #[test]
    fn painted_variant_lookup() {
        let idx = PackageIndex::from_names([
            "wheel_x_TitaniumWhite_SF.upk".to_string(),
            "wheel_x_SF.upk".to_string(),
        ]);
        assert_eq!(
            painted_variant(&idx, "wheel_x_SF.upk", 12).as_deref(),
            Some("wheel_x_TitaniumWhite_SF.upk")
        );
        assert_eq!(painted_variant(&idx, "wheel_x_SF.upk", 1), None);
        assert_eq!(painted_variant(&idx, "wheel_x_SF.upk", 0), None);
    }
}
