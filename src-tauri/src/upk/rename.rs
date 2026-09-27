//! Package renaming — lets a cooked cosmetic package be dropped into another
//! item's slot.
//!
//! RL refuses to load a package whose internal name doesn't match its file
//! name, so the swap engine patches the FName entries the package uses to
//! identify itself. The file size never changes:
//!
//! - **Pad in place** when the target fits the source entry's slot.
//! - **Re-point** otherwise: write the target into another, long-enough
//!   entry that nothing in the header uses, then redirect the name-index
//!   fields of the import/export tables from the source to that entry.
//!
//! Re-pointing only touches parsed fields (`tables::name_ref_offsets`). An
//! earlier version scanned raw bytes for the index and hit the middle of
//! other fields (index 255 = `FF 00 00 00` matches the tail of a negative
//! class index followed by a zero super index), which crashed the game with
//! "Bad export index".

use std::collections::HashSet;

use crate::upk::crypto;
use crate::upk::error::{UpkError, UpkResult};
use crate::upk::keys::KeyRing;
use crate::upk::names::{self, NameEntry, NameTable};
use crate::upk::reader::read_i32;
use crate::upk::summary::PackageSummary;
use crate::upk::tables;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rename {
    pub source: String,
    pub target: String,
}

impl Rename {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
        }
    }
}

fn resolve<'t>(table: &'t NameTable, name: &str) -> UpkResult<&'t NameEntry> {
    // Exact match first; rules derived from file names may differ in case
    // from the name table, so fall back to an ASCII case-insensitive match.
    let entry = table
        .find(name)
        .or_else(|| table.find_ci(name))
        .ok_or_else(|| UpkError::NameNotFound(name.to_string()))?;
    if entry.wide {
        return Err(UpkError::Format(format!("{name} is a UTF-16 name")));
    }
    Ok(entry)
}

/// Renames one entry in place, NUL-padding. Used where re-pointing would be
/// unsafe (names other packages reference, e.g. texture caches).
pub fn pad(plain: &mut [u8], table: &NameTable, rule: &Rename) -> UpkResult<()> {
    let source = resolve(table, &rule.source)?;
    if source.slot_len < rule.target.len() + 1 {
        return Err(UpkError::TargetTooLong {
            from: rule.source.clone(),
            to: rule.target.clone(),
        });
    }
    names::write_slot(plain, source, &rule.target);
    Ok(())
}

/// Applies every rule to a decrypted header region.
///
/// - `table` describes the region *before* any rename (indices and slots
///   never move);
/// - `ref_offsets` are the name-index fields of the import/export tables;
/// - `body_uses(index)` says whether the package body may reference a name —
///   such entries are only recycled when nothing else fits.
pub fn apply_all(
    plain: &mut [u8],
    table: &NameTable,
    ref_offsets: &[usize],
    rules: &[Rename],
    body_uses: impl Fn(usize) -> bool,
) -> UpkResult<()> {
    let sources = rules
        .iter()
        .map(|r| resolve(table, &r.source).map(|e| e.index))
        .collect::<UpkResult<Vec<_>>>()?;
    let referenced: HashSet<usize> = ref_offsets
        .iter()
        .filter_map(|&o| read_i32(plain, o))
        .filter_map(|i| usize::try_from(i).ok())
        .collect();
    // Never recycle a rule's source (another rule may still need it) nor a
    // name the header uses.
    let mut reserved: HashSet<usize> = sources
        .iter()
        .copied()
        .chain(referenced.iter().copied())
        .collect();

    for (rule, &source_idx) in rules.iter().zip(&sources) {
        let source = &table.entries[source_idx];
        let needed = rule.target.len() + 1;
        if source.slot_len >= needed {
            names::write_slot(plain, source, &rule.target);
            continue;
        }
        let fits = |e: &&NameEntry| !e.wide && e.slot_len >= needed && !reserved.contains(&e.index);
        // Prefer entries after the source (asset-internal names), then before.
        let order = || {
            table.entries[source_idx + 1..]
                .iter()
                .chain(&table.entries[..source_idx])
        };
        let candidate = order()
            .filter(fits)
            .find(|e| !body_uses(e.index))
            .or_else(|| order().find(fits))
            .ok_or_else(|| UpkError::NoFit {
                target: rule.target.clone(),
                needed,
                largest: table
                    .entries
                    .iter()
                    .filter(|e| !reserved.contains(&e.index))
                    .map(|e| e.slot_len)
                    .max()
                    .unwrap_or(0),
            })?;
        if body_uses(candidate.index) {
            tracing::warn!(name = %candidate.name, "recycled name slot may be used by the package body");
        }
        names::write_slot(plain, candidate, &rule.target);
        reserved.insert(candidate.index);
        let (from, to) = (
            (source_idx as i32).to_le_bytes(),
            (candidate.index as i32).to_le_bytes(),
        );
        for &o in ref_offsets {
            if plain.get(o..o + 4) == Some(&from[..]) {
                plain[o..o + 4].copy_from_slice(&to);
            }
        }
    }
    Ok(())
}

/// Whether `index` appears as an FName (`index`, `number` = 0) anywhere in
/// the decompressed body. Over-approximates: a false hit only makes a slot
/// less preferred.
fn body_name_uses(body: &[u8]) -> impl Fn(usize) -> bool + '_ {
    move |index| {
        let Ok(i) = i32::try_from(index) else {
            return false;
        };
        let mut pattern = [0u8; 8];
        pattern[..4].copy_from_slice(&i.to_le_bytes());
        body.windows(8).any(|w| w == pattern)
    }
}

/// Renames entries of a package and returns the patched file (same size).
pub fn rename_package(bytes: &[u8], rules: &[Rename], keys: &KeyRing) -> UpkResult<Vec<u8>> {
    let summary = PackageSummary::parse(bytes)?;
    let mut header = crypto::open_header(bytes, &summary, keys)?;
    let table = NameTable::parse(&header.plain, summary.name_count);
    if table.entries.len() != summary.name_count {
        return Err(UpkError::Format(format!(
            "name table: parsed {} of {}",
            table.entries.len(),
            summary.name_count
        )));
    }
    let refs = tables::name_ref_offsets(&header, &summary)?;
    let body = crate::upk::chunks::ChunkMap::read_from(bytes, summary.total_header_size)
        .and_then(|map| crate::upk::chunks::decompress_all(bytes, &map))
        .unwrap_or_default();
    apply_all(
        &mut header.plain,
        &table,
        &refs,
        rules,
        body_name_uses(&body),
    )?;
    let mut out = bytes.to_vec();
    header.splice_into(&mut out)?;
    Ok(out)
}

/// [`rename_package`] on an opened package: renames in its decrypted header
/// (`body` = its decompressed body, to avoid recycling names it uses).
pub fn rename_in_header(
    pkg: &mut crate::upk::Package,
    body: &[u8],
    rules: &[Rename],
) -> UpkResult<()> {
    let table = pkg.names.clone();
    let refs = tables::name_ref_offsets(&pkg.header, &pkg.summary)?;
    apply_all(
        &mut pkg.header.plain,
        &table,
        &refs,
        rules,
        body_name_uses(body),
    )
}

/// Whether the package header can be decrypted with the current keys.
pub fn can_decrypt(bytes: &[u8], keys: &KeyRing) -> bool {
    crypto::can_open(bytes, keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upk::names::tests::build;

    fn never(_: usize) -> bool {
        false
    }

    #[test]
    fn pads_in_place_when_target_fits() {
        let mut plain = build(&["WHEEL_AlphaRim", "WHEEL_AlphaRim_SF"]);
        let table = NameTable::parse(&plain, 2);
        apply_all(
            &mut plain,
            &table,
            &[],
            &[Rename::new("WHEEL_AlphaRim", "WHEEL_Vortex")],
            never,
        )
        .unwrap();
        let after = NameTable::parse(&plain, 2);
        assert_eq!(after.get(0), "WHEEL_Vortex");
        assert_eq!(after.get(1), "WHEEL_AlphaRim_SF");
    }

    #[test]
    fn repoints_only_name_fields_to_an_unused_slot() {
        let mut plain = build(&["ABC", "Filler1", "FillerLonger", "WayLongerFiller"]);
        let field = plain.len();
        plain.extend(0i32.to_le_bytes()); // name field → "ABC"
        plain.extend(2i32.to_le_bytes()); // name field → "FillerLonger" (in use)
        plain.extend(0i32.to_le_bytes()); // not a name field
        let table = NameTable::parse(&plain, 4);
        apply_all(
            &mut plain,
            &table,
            &[field, field + 4],
            &[Rename::new("ABC", "NewerName")],
            never,
        )
        .unwrap();
        let after = NameTable::parse(&plain, 4);
        assert_eq!(
            after.get(2),
            "FillerLonger",
            "a name in use is never recycled"
        );
        assert_eq!(after.get(3), "NewerName");
        let at = |o: usize| i32::from_le_bytes(plain[o..o + 4].try_into().unwrap());
        assert_eq!((at(field), at(field + 4), at(field + 8)), (3, 2, 0));
    }

    /// Regression (2026-09-27, WHEEL_AlphaRim_SF → Wheel_SoccerBall_SF): the
    /// source index 255 is `FF 00 00 00`, which a raw scan found straddling
    /// a negative class index and a zero super index.
    #[test]
    fn never_touches_bytes_straddling_fields() {
        let mut names_list = vec!["N".to_string(); 255];
        names_list.push("WHEEL_AlphaRim_SF".into());
        names_list.push("WHEEL_AlphaRim_Thumbnail".into());
        let refs: Vec<&str> = names_list.iter().map(String::as_str).collect();
        let mut plain = build(&refs);
        let export = plain.len();
        plain.extend((-13i32).to_le_bytes()); // class index F3 FF FF FF
        plain.extend(0i32.to_le_bytes()); // super
        plain.extend(0i32.to_le_bytes()); // outer
        plain.extend(255i32.to_le_bytes()); // object name
        let table = NameTable::parse(&plain, 257);
        apply_all(
            &mut plain,
            &table,
            &[export + 12],
            &[Rename::new("WHEEL_AlphaRim_SF", "Wheel_SoccerBall_SF")],
            never,
        )
        .unwrap();
        let at = |o: usize| i32::from_le_bytes(plain[o..o + 4].try_into().unwrap());
        assert_eq!((at(export), at(export + 4), at(export + 12)), (-13, 0, 256));
        assert_eq!(
            NameTable::parse(&plain, 257).get(256),
            "Wheel_SoccerBall_SF"
        );
    }

    /// Regression: with base + `_SF` rules, the first rule must not recycle
    /// the second rule's source, and the second re-point must not catch the
    /// first one's fields.
    #[test]
    fn rules_do_not_step_on_each_other() {
        let mut plain = build(&[
            "WHEEL_AlphaRim",
            "WHEEL_AlphaRim_SF",
            "Spare_Name_Slot_1",
            "Spare_Name_Slot_222",
        ]);
        let fields = plain.len();
        plain.extend(0i32.to_le_bytes());
        plain.extend(1i32.to_le_bytes());
        let table = NameTable::parse(&plain, 4);
        let rules = [
            Rename::new("WHEEL_AlphaRim", "Wheel_SoccerBall"),
            Rename::new("WHEEL_AlphaRim_SF", "Wheel_SoccerBall_SF"),
        ];
        apply_all(&mut plain, &table, &[fields, fields + 4], &rules, never).unwrap();
        let after = NameTable::parse(&plain, 4);
        let at = |o: usize| i32::from_le_bytes(plain[o..o + 4].try_into().unwrap());
        assert_eq!(after.get(at(fields)), "Wheel_SoccerBall");
        assert_eq!(after.get(at(fields + 4)), "Wheel_SoccerBall_SF");
        assert_ne!(at(fields), at(fields + 4));
    }

    #[test]
    fn prefers_slots_the_body_does_not_use() {
        let mut plain = build(&["ABC", "BodyUsedName", "FreeLongName"]);
        let table = NameTable::parse(&plain, 3);
        apply_all(
            &mut plain,
            &table,
            &[],
            &[Rename::new("ABC", "LongerTarget")],
            |i| i == 1,
        )
        .unwrap();
        let after = NameTable::parse(&plain, 3);
        assert_eq!(
            (after.get(1), after.get(2)),
            ("BodyUsedName", "LongerTarget")
        );
    }

    #[test]
    fn source_matching_is_case_insensitive_but_prefers_exact() {
        let mut plain = build(&["wheel_quartz", "WHEEL_Quartz_SF"]);
        let table = NameTable::parse(&plain, 2);
        let rules = [
            Rename::new("Wheel_Quartz_SF", "WHEEL_Vortex_SF"),
            Rename::new("wheel_quartz", "WHEEL_Vortex"),
        ];
        apply_all(&mut plain, &table, &[], &rules, never).unwrap();
        let after = NameTable::parse(&plain, 2);
        assert_eq!(after.get(0), "WHEEL_Vortex");
        assert_eq!(after.get(1), "WHEEL_Vortex_SF");
    }

    #[test]
    fn pad_refuses_longer_target() {
        let mut plain = build(&["Textures7"]);
        let table = NameTable::parse(&plain, 1);
        let err = pad(
            &mut plain,
            &table,
            &Rename::new("Textures7", "MuchLongerName"),
        );
        assert!(matches!(err, Err(UpkError::TargetTooLong { .. })));
    }

    #[test]
    fn missing_source_and_no_fit_are_errors() {
        let mut plain = build(&["ABC", "XYZ"]);
        let table = NameTable::parse(&plain, 2);
        assert!(matches!(
            apply_all(&mut plain, &table, &[], &[Rename::new("Nope", "X")], never),
            Err(UpkError::NameNotFound(_))
        ));
        assert!(matches!(
            apply_all(
                &mut plain,
                &table,
                &[],
                &[Rename::new("ABC", "AVeryLongTargetName")],
                never
            ),
            Err(UpkError::NoFit { .. })
        ));
    }

    #[test]
    fn body_scan_matches_fname_pairs() {
        let body = [0u8, 7, 1, 0, 0, 0, 0, 0, 0, 9];
        let uses = body_name_uses(&body);
        assert!(uses(0x107));
        assert!(!uses(0x108));
    }
}
