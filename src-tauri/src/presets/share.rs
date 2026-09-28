//! Share codes: `ALXS1:` + base64url(deflate(compact JSON)).
//!
//! Items are referenced by product id (identical on every install), the
//! palette is embedded (a friend may not have it), the map by id only (maps
//! are local files; the receiver keeps whatever map they have).

use std::io::{Read, Write};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};
use crate::catalog::Slot;
use crate::presets::model::Preset;
use crate::swap::SwapRequest;

pub const PREFIX: &str = "ALXS1:";
/// Inflated payload cap — a real code is a few KB; refuse decompression bombs.
const MAX_JSON_BYTES: u64 = 512 * 1024;

/// Wire format, deliberately terse.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShareV1 {
    pub n: String,
    /// `[slot index, owned id, wanted id, paint]`
    #[serde(default)]
    pub s: Vec<(u8, u32, u32, u8)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub m: Option<String>,
}

pub fn encode_wire(wire: &ShareV1) -> AppResult<String> {
    let json = serde_json::to_vec(wire)?;
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    enc.write_all(&json)?;
    Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(enc.finish()?)))
}

pub fn decode_wire(code: &str) -> AppResult<ShareV1> {
    let body = code
        .trim()
        .strip_prefix(PREFIX)
        .ok_or_else(|| AppError::InvalidInput("not an ALXS-RL-Mod share code".into()))?;
    let compressed = URL_SAFE_NO_PAD
        .decode(body.trim().trim_end_matches('='))
        .map_err(|_| AppError::InvalidInput("share code is damaged (base64)".into()))?;
    let mut json = Vec::new();
    DeflateDecoder::new(compressed.as_slice())
        .take(MAX_JSON_BYTES + 1)
        .read_to_end(&mut json)
        .map_err(|_| AppError::InvalidInput("share code is damaged (deflate)".into()))?;
    if json.len() as u64 > MAX_JSON_BYTES {
        return Err(AppError::InvalidInput("share code is too large".into()));
    }
    serde_json::from_slice(&json)
        .map_err(|_| AppError::InvalidInput("share code is damaged (json)".into()))
}

pub fn to_wire(preset: &Preset) -> AppResult<ShareV1> {
    Ok(ShareV1 {
        n: preset.name.clone(),
        s: preset
            .swaps
            .iter()
            .map(|r| {
                (
                    r.slot.index(),
                    r.owned_id,
                    r.wanted_id,
                    r.paint.unwrap_or(0),
                )
            })
            .collect(),
        p: preset
            .palette
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?,
        d: preset
            .decal
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?,
        m: preset.map_id.clone(),
    })
}

/// Builds an unsaved preset. Unknown slots are dropped; a palette or decal
/// that doesn't parse is dropped too (older/newer app version) rather than
/// failing the whole import.
pub fn from_wire(wire: ShareV1) -> Preset {
    let mut preset = Preset::empty(if wire.n.trim().is_empty() {
        "Imported".to_string()
    } else {
        wire.n
    });
    preset.swaps = wire
        .s
        .into_iter()
        .filter_map(|(slot, owned_id, wanted_id, paint)| {
            Slot::from_index(slot).map(|slot| SwapRequest {
                slot,
                owned_id,
                wanted_id,
                paint: (paint > 0 && paint <= crate::swap::paint::MAX_PAINT_ID).then_some(paint),
                color: None,
                tint: None,
            })
        })
        .collect();
    preset.palette = wire.p.and_then(|v| serde_json::from_value(v).ok());
    preset.decal = wire.d.and_then(|v| serde_json::from_value(v).ok());
    preset.map_id = wire.m;
    preset
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ShareV1 {
        ShareV1 {
            n: "Octane ✨ Titanium".into(),
            s: vec![
                (Slot::Wheels.index(), 10, 20, 12),
                (Slot::Decal.index(), 30, 40, 0),
            ],
            p: Some(serde_json::json!({ "id": "x", "colors": ["#ff0000"] })),
            d: None,
            m: Some("map_1".into()),
        }
    }

    #[test]
    fn roundtrip() {
        let code = encode_wire(&sample()).unwrap();
        assert!(code.starts_with(PREFIX));
        assert!(!code.contains('+') && !code.contains('/'));
        assert_eq!(decode_wire(&format!("  {code}\n")).unwrap(), sample());
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_wire("hello").is_err());
        assert!(decode_wire("ALXS1:!!!").is_err());
        assert!(decode_wire("ALXS1:AAAA").is_err());
    }

    #[test]
    fn wire_to_preset_drops_invalid_entries() {
        let mut w = sample();
        w.s.push((250, 1, 2, 0)); // unknown slot
        w.s.push((Slot::Boost.index(), 1, 2, 99)); // invalid paint
        let p = from_wire(w);
        assert_eq!(p.swaps.len(), 3);
        assert_eq!(p.swaps[0].paint, Some(12));
        assert_eq!(p.swaps[2].paint, None);
        assert_eq!(p.map_id.as_deref(), Some("map_1"));
    }
}
