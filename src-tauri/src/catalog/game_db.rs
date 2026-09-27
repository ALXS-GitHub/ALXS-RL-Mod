//! Product list read from the game itself, so the catalog follows every
//! season without an app update.
//!
//! Rocket League ships its localisation as `CookedPCConsole/Coalesced_<LANG>.bin`:
//! an AES-256-ECB encrypted UE3 "coalesced" archive (one of the keys of the
//! package key ring opens it). Inside, `TAGame\Localization\<LANG>\Products.<ext>`
//! has one section per product — named after the product's asset — with its
//! display `Label`. That gives the full, current product list with real
//! English and French names. The slot is inferred from the asset prefix
//! (`wheel_`, `skin_`, `hat_`…); bundles (`pack_*`) from their name.

use std::path::Path;

use crate::catalog::model::Slot;
use crate::upk::{crypto, KeyRing};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameProduct {
    pub asset: String,
    pub label_en: String,
    pub label_fr: Option<String>,
}

/// One `[section]` of a coalesced file: `(name, key/value pairs)`.
type Section = (String, Vec<(String, String)>);
/// One file of the archive: `(relative path, sections)`.
type CoalescedFile = (String, Vec<Section>);

const MAX_FILES: i32 = 4096;
const MAX_ENTRIES: i32 = 1_000_000;

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn i32(&mut self) -> Option<i32> {
        let bytes = self.buf.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(i32::from_le_bytes(bytes.try_into().ok()?))
    }

    /// UE3 FString: positive length = Latin-1 with NUL, negative = UTF-16 with NUL.
    fn fstring(&mut self) -> Option<String> {
        let len = self.i32()?;
        if len == 0 {
            return Some(String::new());
        }
        if len > 0 {
            let n = usize::try_from(len).ok()?;
            let bytes = self.buf.get(self.pos..self.pos + n)?;
            self.pos += n;
            let trimmed = bytes.strip_suffix(&[0]).unwrap_or(bytes);
            Some(trimmed.iter().map(|&b| char::from(b)).collect())
        } else {
            let chars = usize::try_from(len.checked_neg()?).ok()?;
            let bytes = self.buf.get(self.pos..self.pos + chars * 2)?;
            self.pos += chars * 2;
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let trimmed = units.strip_suffix(&[0]).unwrap_or(&units);
            Some(String::from_utf16_lossy(trimmed))
        }
    }

    fn count(&mut self, max: i32) -> Option<usize> {
        let n = self.i32()?;
        (0..=max).contains(&n).then_some(n as usize)
    }
}

/// Parses a decrypted coalesced archive. `None` if the layout does not hold.
pub fn parse_coalesced(plain: &[u8]) -> Option<Vec<CoalescedFile>> {
    let mut c = Cursor { buf: plain, pos: 0 };
    let files = c.count(MAX_FILES)?;
    let mut out = Vec::with_capacity(files);
    for _ in 0..files {
        let name = c.fstring()?;
        let sections = c.count(MAX_ENTRIES)?;
        let mut secs = Vec::with_capacity(sections.min(65_536));
        for _ in 0..sections {
            let section = c.fstring()?;
            let entries = c.count(MAX_ENTRIES)?;
            let mut kv = Vec::with_capacity(entries.min(64));
            for _ in 0..entries {
                kv.push((c.fstring()?, c.fstring()?));
            }
            secs.push((section, kv));
        }
        out.push((name, secs));
    }
    Some(out)
}

/// Finds the key that opens the archive and returns the parsed content.
/// The first block is enough to reject wrong keys cheaply.
fn open_coalesced(bytes: &[u8], ring: &KeyRing) -> Option<Vec<CoalescedFile>> {
    let usable = bytes.len() - bytes.len() % 16;
    let head = bytes.get(..32.min(usable))?;
    for key in ring.keys() {
        let probe = crypto::ecb_decrypt(head, key);
        let files = i32::from_le_bytes(probe.get(..4)?.try_into().ok()?);
        let name_len = i32::from_le_bytes(probe.get(4..8)?.try_into().ok()?);
        if !(1..=MAX_FILES).contains(&files) || name_len == 0 || name_len.unsigned_abs() > 1024 {
            continue;
        }
        let plain = crypto::ecb_decrypt(&bytes[..usable], key);
        if let Some(parsed) = parse_coalesced(&plain) {
            return Some(parsed);
        }
    }
    None
}

/// `(asset, label)` pairs of the `Products` file of one language. The label
/// may be empty: recent products sometimes ship without a display name and
/// must still be listed (the catalog falls back to the asset name).
fn product_labels(files: &[CoalescedFile]) -> Vec<(String, String)> {
    let Some((_, sections)) = files.iter().find(|(name, _)| {
        let lower = name.to_ascii_lowercase().replace('/', "\\");
        lower.contains("\\tagame\\localization\\")
            && lower
                .rsplit('\\')
                .next()
                .is_some_and(|f| f.starts_with("products."))
    }) else {
        return Vec::new();
    };
    sections
        .iter()
        // Product sections are bare asset names; other objects are `Name Class`.
        .filter(|(name, _)| !name.contains(' ') && !name.is_empty())
        .map(|(name, kv)| {
            let label = kv
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("Label"))
                .map_or("", |(_, v)| v.trim());
            (name.clone(), label.to_string())
        })
        .collect()
}

fn read_language(cooked: &Path, lang: &str, ring: &KeyRing) -> Option<Vec<(String, String)>> {
    let path = cooked.join(format!("Coalesced_{lang}.bin"));
    let bytes = std::fs::read(&path).ok()?;
    let files = open_coalesced(&bytes, ring)?;
    let labels = product_labels(&files);
    (!labels.is_empty()).then_some(labels)
}

/// Current product list of the installed game (`None` if the localisation
/// archive cannot be opened with the available keys).
pub fn read(cooked: &Path, ring: &KeyRing) -> Option<Vec<GameProduct>> {
    let english = read_language(cooked, "INT", ring)?;
    let french: std::collections::HashMap<String, String> = read_language(cooked, "FRA", ring)
        .unwrap_or_default()
        .into_iter()
        .map(|(asset, label)| (asset.to_ascii_lowercase(), label))
        .collect();
    Some(
        english
            .into_iter()
            .map(|(asset, label_en)| GameProduct {
                label_fr: french.get(&asset.to_ascii_lowercase()).cloned(),
                asset,
                label_en,
            })
            .collect(),
    )
}

/// Slot from the asset naming convention (measured on the bundled product
/// list: every prefix below maps to a single slot in > 90 % of cases).
pub fn slot_from_asset(asset: &str) -> Option<Slot> {
    let prefix = asset
        .split('_')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    Some(match prefix.as_str() {
        "body" | "blackmarkettest" => Slot::Body,
        "skin" | "skins" => Slot::Decal,
        "wheel" => Slot::Wheels,
        "boost" => Slot::Boost,
        "hat" | "crown" => Slot::Topper,
        "antenna" | "at" | "flag" | "countryflag" | "streamerflag" | "tourneyflag" | "pennant" => {
            Slot::Antenna
        }
        "explosion" => Slot::GoalExplosion,
        "ss" => Slot::Trail,
        "paintfinish" => Slot::PaintFinish,
        "pack" => return pack_slot(asset),
        _ => return None,
    })
}

/// Item bundles (`pack_nflflags`, `Pack_CountryFlags`, `pack_rlcs2025`…):
/// one product whose variants are picked in game.
fn pack_slot(asset: &str) -> Option<Slot> {
    let name = asset.to_ascii_lowercase();
    Some(if name.contains("explosion") {
        Slot::GoalExplosion
    } else if name.contains("hat") || name.contains("helmet") {
        Slot::Topper
    } else if name.contains("flag") || name == "pack_nba" || name == "pack_bb" {
        Slot::Antenna
    } else {
        Slot::Decal
    })
}

/// Stable product id derived from the asset name (FNV-1a, high bit set so
/// it never collides with the small numeric ids earlier versions stored).
pub fn synthetic_id(asset: &str) -> u32 {
    // FNV-1a, 32 bit.
    let mut hash: u32 = 0x811c_9dc5;
    for b in asset.to_ascii_lowercase().bytes() {
        hash ^= u32::from(b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash | 0x8000_0000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fstr(out: &mut Vec<u8>, s: &str) {
        out.extend((s.len() as i32 + 1).to_le_bytes());
        out.extend(s.as_bytes());
        out.push(0);
    }

    fn wide(out: &mut Vec<u8>, s: &str) {
        let units: Vec<u16> = s.encode_utf16().collect();
        out.extend((-(units.len() as i32 + 1)).to_le_bytes());
        for u in units {
            out.extend(u.to_le_bytes());
        }
        out.extend([0, 0]);
    }

    fn sample() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(2i32.to_le_bytes());
        // File 1: unrelated.
        wide(&mut b, "..\\..\\Engine\\Localization\\INT\\Core.int");
        b.extend(0i32.to_le_bytes());
        // File 2: products.
        wide(&mut b, "..\\..\\TAGame\\Localization\\INT\\Products.int");
        b.extend(4i32.to_le_bytes());
        fstr(&mut b, "Antenna_8Ball");
        b.extend(1i32.to_le_bytes());
        fstr(&mut b, "Label");
        wide(&mut b, "8-Ball");
        fstr(&mut b, "Body ProductSlot_TA");
        b.extend(1i32.to_le_bytes());
        fstr(&mut b, "Label");
        fstr(&mut b, "Body");
        fstr(&mut b, "boost_alphadevreward");
        b.extend(1i32.to_le_bytes());
        fstr(&mut b, "Label");
        fstr(&mut b, "");
        fstr(&mut b, "wheel_zigzag");
        b.extend(2i32.to_le_bytes());
        fstr(&mut b, "Artist");
        fstr(&mut b, "x");
        fstr(&mut b, "Label");
        wide(&mut b, "Zig Zag");
        b
    }

    #[test]
    fn parses_archive_and_extracts_product_labels() {
        let files = parse_coalesced(&sample()).unwrap();
        assert_eq!(files.len(), 2);
        let labels = product_labels(&files);
        assert_eq!(
            labels,
            vec![
                ("Antenna_8Ball".into(), "8-Ball".into()),
                ("boost_alphadevreward".into(), String::new()),
                ("wheel_zigzag".into(), "Zig Zag".into()),
            ]
        );
    }

    #[test]
    fn rejects_truncated_archives() {
        let full = sample();
        assert!(parse_coalesced(&full[..full.len() - 3]).is_none());
        assert!(parse_coalesced(&[0xff, 0xff, 0xff, 0x7f]).is_none());
    }

    #[test]
    fn finds_the_right_key() {
        let key = [7u8; 32];
        let mut plain = sample();
        plain.resize(plain.len().next_multiple_of(16), 0);
        let encrypted = crypto::ecb_encrypt(&plain, &key);
        let ring = KeyRing::from_keys(vec![[1u8; 32], key]);
        let files = open_coalesced(&encrypted, &ring).unwrap();
        assert_eq!(product_labels(&files).len(), 3);
    }

    #[test]
    fn slots_follow_the_naming_convention() {
        assert_eq!(slot_from_asset("wheel_zigzag_inverted"), Some(Slot::Wheels));
        assert_eq!(slot_from_asset("Skin_Octane_Flames"), Some(Slot::Decal));
        assert_eq!(slot_from_asset("CountryFlag_France"), Some(Slot::Antenna));
        assert_eq!(slot_from_asset("SS_Tier_01"), Some(Slot::Trail));
        assert_eq!(slot_from_asset("PlayerBanner_X"), None);
        assert_eq!(slot_from_asset("pack_thunderdecals"), Some(Slot::Decal));
        assert_eq!(slot_from_asset("Pack_CountryFlags"), Some(Slot::Antenna));
        assert_eq!(slot_from_asset("pack_nba"), Some(Slot::Antenna));
        assert_eq!(slot_from_asset("pack_bb_hats"), Some(Slot::Topper));
        assert_eq!(
            slot_from_asset("pack_esportsexplosions23"),
            Some(Slot::GoalExplosion)
        );
    }

    #[test]
    fn synthetic_ids_are_stable_and_out_of_the_real_range() {
        let a = synthetic_id("wheel_Zigzag");
        assert_eq!(a, synthetic_id("WHEEL_zigzag"));
        assert!(a >= 0x8000_0000);
        assert_ne!(a, synthetic_id("wheel_zigzag_inverted"));
    }
}
