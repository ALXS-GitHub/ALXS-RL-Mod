//! Pure planning helpers: package-name rename rules.
//!
//! Rocket League only loads a cooked package whose internal name matches its
//! file name, so the wanted package is renamed to the owned package's name
//! before being written over it. Rules proven byte-identical to rlpeak's
//! output (see `docs_rl/` and the legacy `upk_renamer` tests):
//! - wheels, boosts and most slots: rename the base name AND the `_SF` name;
//! - decals: rename the base name only — the `_SF` FName is referenced by the
//!   package's own asset lookups and must keep resolving.

use crate::catalog::resolver::{package_base, package_stem};
use crate::catalog::Slot;
use crate::upk::rename::Rename;

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
}
