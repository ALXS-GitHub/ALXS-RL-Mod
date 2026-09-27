//! Which car bodies take custom decals, which stock packages donate the
//! material, and which owned decal slots receive them.
//!
//! Two donor kinds per body:
//! - **paintable** (mask-only AlphaConsole packs): a Force-pattern decal on
//!   `Body_Paintable_Mat` (Octane: GaleFire, validated 2026-05).
//! - **hybrid** (art + zone mask): a decal on the stock
//!   `Body_Paintable_Diffuse_Mat` without forced colours, so the colour
//!   pickers stay active (Octane: ClassyLady, validated 2026-09-27). Donors
//!   were found with a scan of every `skin_<codename>_*_SF.upk` that runs the
//!   real pipeline on it; Venom, Road Hog and Esper have none.
//!
//! - **universal** (AlphaConsole universal packs: `1_Diffuse_Skin` art +
//!   `TrimSheet` logo sheet): the auto-universal esports decals, whose art
//!   follows every body; AlphaConsole authored these packs for Team BDS
//!   (2025). They force their colours: the art shows as is.
//!
//! Target slots are stock decals of the body that every player owns; for
//! universal decals, one of the esports 2025 universal decals (Spacestation
//! Gaming's package name does not fit the donor's name table).

use serde::Serialize;

use crate::upk::Rename;

pub struct DonorSpec {
    pub body_id: i32,
    pub body_name: &'static str,
    /// Donor for mask-only packs, if this body has one.
    pub paintable: Option<Donor>,
    /// Donor for hybrid packs (art + zone mask), if this body has one.
    pub hybrid: Option<Donor>,
    /// Donor for universal packs (body -1 only).
    pub universal: Option<Donor>,
    pub targets: &'static [TargetSpec],
}

pub struct Donor {
    /// Stock package the swap starts from.
    pub file: &'static str,
    /// Package base name inside the donor (as spelled in its name table).
    pub package: &'static str,
    /// Pattern word used in the donor's texture/material names.
    pub pattern: &'static str,
}

pub struct TargetSpec {
    pub key: &'static str,
    pub label: &'static str,
    /// Package base of the owned decal (`Skin_Octane_Stars` → `Skin_Octane_Stars_SF.upk`).
    pub package: &'static str,
    pub pattern: &'static str,
}

const fn target(
    key: &'static str,
    label: &'static str,
    package: &'static str,
    pattern: &'static str,
) -> TargetSpec {
    TargetSpec {
        key,
        label,
        package,
        pattern,
    }
}

const OCTANE_TARGETS: &[TargetSpec] = &[
    target("octane-stars", "Stars", "Skin_Octane_Stars", "Stars"),
    target("octane-flames", "Flames", "Skin_Octane_Flames", "Flames"),
];

const DOMINUS_TARGETS: &[TargetSpec] = &[
    target(
        "dominus-flames",
        "Flames",
        "Skin_MuscleCar_Flames",
        "Flames",
    ),
    target(
        "dominus-stripes",
        "Stripes",
        "Skin_MuscleCar_Stripes",
        "Stripes",
    ),
];

const FENNEC_TARGETS: &[TargetSpec] = &[
    target("fennec-flames", "Flames", "skin_grain_flames", "flames"),
    target("fennec-stripes", "Stripes", "skin_grain_stripes", "stripes"),
];

const UNIVERSAL_TARGETS: &[TargetSpec] = &[
    target(
        "universal-bds",
        "Team BDS (2025)",
        "skin_autobds_00",
        "autobds",
    ),
    target("universal-nrg", "NRG (2025)", "skin_autonrg_00", "autonrg"),
    target("universal-tsm", "TSM (2025)", "skin_autotsm_00", "autotsm"),
    target(
        "universal-g2",
        "Gen.G (2025)",
        "skin_autogeng_00",
        "autogeng",
    ),
    target(
        "universal-furia",
        "Furia (2025)",
        "skin_autofuria_00",
        "autofuria",
    ),
    target(
        "universal-falcons",
        "Team Falcons (2025)",
        "skin_autofalcon_00",
        "autofalcon",
    ),
    target(
        "universal-vitality",
        "Team Vitality (2025)",
        "skin_autovitality_00",
        "autovitality",
    ),
    target(
        "universal-dignitas",
        "Dignitas (2025)",
        "skin_autodignitas_00",
        "autodignitas",
    ),
    target(
        "universal-luminosity",
        "Luminosity (2025)",
        "skin_autoluminosity_00",
        "autoluminosity",
    ),
    target(
        "universal-limitless",
        "Limitless (2025)",
        "skin_autolimitless_00",
        "autolimitless",
    ),
    target(
        "universal-gentlemates",
        "Gentle Mates (2025)",
        "skin_autogentlemates_00",
        "autogentlemates",
    ),
];

pub const DONORS: &[DonorSpec] = &[
    DonorSpec {
        body_id: 23,
        body_name: "Octane",
        paintable: Some(Donor {
            file: "skin_octane_galefire_SF.upk",
            package: "skin_octane_galefire",
            pattern: "GaleFire",
        }),
        hybrid: Some(Donor {
            file: "skin_octane_classylady_SF.upk",
            package: "skin_octane_classylady",
            pattern: "ClassyLady",
        }),
        universal: None,
        targets: OCTANE_TARGETS,
    },
    DonorSpec {
        body_id: 403,
        body_name: "Dominus",
        paintable: None,
        hybrid: Some(Donor {
            file: "skin_musclecar_copystripes_SF.upk",
            package: "skin_musclecar_copystripes",
            pattern: "copystripes",
        }),
        universal: None,
        targets: DOMINUS_TARGETS,
    },
    DonorSpec {
        body_id: 4284,
        body_name: "Fennec",
        paintable: None,
        hybrid: Some(Donor {
            file: "skin_grain_bugbite_SF.upk",
            package: "skin_grain_bugbite",
            pattern: "BugBite",
        }),
        universal: None,
        targets: FENNEC_TARGETS,
    },
    DonorSpec {
        body_id: -1,
        body_name: "Universal",
        paintable: None,
        hybrid: None,
        universal: Some(Donor {
            file: "skin_autobds_00_SF.upk",
            package: "skin_autobds_00",
            pattern: "autobds",
        }),
        targets: UNIVERSAL_TARGETS,
    },
];

pub fn donor_for(body_id: i32) -> Option<&'static DonorSpec> {
    DONORS.iter().find(|d| d.body_id == body_id)
}

pub fn body_name(body_id: i32) -> Option<&'static str> {
    donor_for(body_id).map(|d| d.body_name)
}

pub fn find_target(donor: &DonorSpec, key: Option<&str>) -> Option<&'static TargetSpec> {
    match key {
        Some(k) => donor.targets.iter().find(|t| t.key == k),
        None => donor.targets.first(),
    }
}

impl TargetSpec {
    pub fn file_name(&self) -> String {
        format!("{}_SF.upk", self.package)
    }
}

/// Renames turning the paintable donor into the target slot. The first two
/// (package names) are mandatory; the others are best-effort (a donor may
/// not carry a thumbnail entry, for instance). All are length-preserving.
pub fn renames(spec: &DonorSpec, donor: &Donor, target: &TargetSpec) -> (Vec<Rename>, Vec<Rename>) {
    let (package, dp, tp) = (donor.package, donor.pattern, target.pattern);
    let body = spec.body_name;
    let required = vec![
        Rename::new(format!("{package}_SF"), format!("{}_SF", target.package)),
        Rename::new(package, target.package),
    ];
    let optional = vec![
        Rename::new(
            format!("Skin_{body}_{dp}_RGB"),
            format!("Skin_{body}_{tp}_RGB"),
        ),
        Rename::new(format!("{body}_{dp}_MIC"), format!("{body}_{tp}_MIC")),
        Rename::new(
            format!("skin_{}_{dp}_TThumbnail", body.to_lowercase()),
            format!("skin_{}_{tp}_TThumbnail", body.to_lowercase()),
        ),
        Rename::new(
            format!("skin_{}_{dp}_T", body.to_lowercase()),
            format!("skin_{}_{tp}_T", body.to_lowercase()),
        ),
    ];
    (required, optional)
}

/// Renames turning a hybrid or universal donor into the target slot:
/// package names only (the game finds the decal through them). Longer
/// targets are re-pointed ([`crate::upk::rename`]).
pub fn hybrid_renames(donor: &Donor, target: &TargetSpec) -> Vec<Rename> {
    vec![
        Rename::new(
            format!("{}_SF", donor.package),
            format!("{}_SF", target.package),
        ),
        Rename::new(donor.package, target.package),
    ]
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSlot {
    pub key: String,
    pub label: String,
    pub body_id: i32,
    pub body_name: String,
    pub file_name: String,
}

pub fn all_slots() -> Vec<TargetSlot> {
    DONORS
        .iter()
        .flat_map(|d| {
            d.targets.iter().map(move |t| TargetSlot {
                key: t.key.into(),
                label: t.label.into(),
                body_id: d.body_id,
                body_name: d.body_name.into(),
                file_name: t.file_name(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paintable_renames_are_length_preserving() {
        for spec in DONORS {
            for target in spec.targets {
                let mut rules = Vec::new();
                if let Some(p) = &spec.paintable {
                    let (req, opt) = renames(spec, p, target);
                    rules.extend(req);
                    rules.extend(opt);
                }
                assert!(
                    spec.paintable.is_some() || spec.hybrid.is_some() || spec.universal.is_some(),
                    "{} has no donor",
                    spec.body_name
                );
                for r in rules {
                    assert!(
                        r.target.len() <= r.source.len(),
                        "{} -> {}",
                        r.source,
                        r.target
                    );
                }
            }
        }
    }

    #[test]
    fn target_keys_are_unique() {
        let slots = all_slots();
        for (i, s) in slots.iter().enumerate() {
            assert!(slots[i + 1..].iter().all(|o| o.key != s.key), "{}", s.key);
        }
    }

    #[test]
    fn octane_mapping_matches_validated_names() {
        let d = donor_for(23).unwrap();
        let t = find_target(d, None).unwrap();
        assert_eq!(t.file_name(), "Skin_Octane_Stars_SF.upk");
        let (req, opt) = renames(d, d.paintable.as_ref().unwrap(), t);
        assert_eq!(
            req[0],
            Rename::new("skin_octane_galefire_SF", "Skin_Octane_Stars_SF")
        );
        assert_eq!(
            opt[0],
            Rename::new("Skin_Octane_GaleFire_RGB", "Skin_Octane_Stars_RGB")
        );
        assert_eq!(
            opt[2],
            Rename::new(
                "skin_octane_GaleFire_TThumbnail",
                "skin_octane_Stars_TThumbnail"
            )
        );
        assert_eq!(
            hybrid_renames(d.hybrid.as_ref().unwrap(), t)[0],
            Rename::new("skin_octane_classylady_SF", "Skin_Octane_Stars_SF")
        );
        assert!(find_target(d, Some("nope")).is_none());
        assert_eq!(body_name(-1), Some("Universal"));
        assert_eq!(body_name(4284), Some("Fennec"));
    }
}
