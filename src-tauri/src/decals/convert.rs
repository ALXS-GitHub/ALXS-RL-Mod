//! AlphaConsole packs → hybrid packs.
//!
//! AlphaConsole's body mask (`Skin` PNG) marks, per pixel:
//! - red at alpha 0 → primary colour,
//! - red at alpha 255 → accent colour,
//! - dark red `#2B0000` → the art (`Diffuse` PNG) as is,
//! - blue → windows (the art shows there too).
//!
//! The hybrid material wants a zone mask (R primary, G accent, black = art)
//! and the art itself. Under painted zones the material multiplies the
//! colour by the desaturated art: AlphaConsole art is usually black there,
//! which would turn the zones black, so it is set to white.

use std::path::Path;

use image::{imageops, Rgba, RgbaImage};
use serde_json::Value;

use crate::base::{AppError, AppResult};

/// File names written next to the pack's `Template.json`.
pub const ART_FILE: &str = "hybrid_art.png";
pub const ZONES_FILE: &str = "hybrid_zones.png";

const PRIMARY: Rgba<u8> = Rgba([255, 0, 0, 255]);
const ACCENT: Rgba<u8> = Rgba([0, 255, 0, 255]);
const ART: Rgba<u8> = Rgba([0, 0, 0, 255]);

/// Zone of an AlphaConsole mask pixel.
fn zone(p: Rgba<u8>) -> Rgba<u8> {
    let [r, _, _, a] = p.0;
    match (r >= 128, a >= 128) {
        (true, false) => PRIMARY,
        (true, true) => ACCENT,
        _ => ART,
    }
}

/// Converts an AlphaConsole `Diffuse` + `Skin` pair to hybrid art + zones
/// (both at the art's size; the mask is resampled nearest-neighbour so its
/// zones keep hard edges).
pub fn from_alphaconsole(diffuse: &RgbaImage, skin: &RgbaImage) -> (RgbaImage, RgbaImage) {
    let (w, h) = diffuse.dimensions();
    let skin = if skin.dimensions() == (w, h) {
        skin.clone()
    } else {
        imageops::resize(skin, w, h, imageops::FilterType::Nearest)
    };
    let zones = RgbaImage::from_fn(w, h, |x, y| zone(*skin.get_pixel(x, y)));
    let art = RgbaImage::from_fn(w, h, |x, y| {
        if *zones.get_pixel(x, y) == ART {
            let [r, g, b, _] = diffuse.get_pixel(x, y).0;
            // Alpha is the material's highlight mask: none.
            Rgba([r, g, b, 0])
        } else {
            Rgba([255, 255, 255, 0])
        }
    });
    (art, zones)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackConversion {
    Converted,
    AlreadyHybrid,
    /// Not a `Diffuse` + `Skin` body pack (universal decals, chassis only…).
    NotApplicable,
}

fn role<'a>(body: &'a serde_json::Map<String, Value>, name: &str) -> Option<&'a str> {
    body.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .and_then(|(_, v)| v.as_str())
}

/// Converts the pack whose `Template.json` is in `dir`, in place: writes
/// [`ART_FILE`] + [`ZONES_FILE`] next to it and adds them to `Body` as
/// `1_Diffuse_Skin` / `2_Diffuse_Skin_Mask`. The original files and roles
/// are kept.
pub fn convert_pack_dir(dir: &Path) -> AppResult<PackConversion> {
    let template = crate::decals::library::manifest_in(dir)
        .ok_or_else(|| AppError::NotFound(format!("no Template.json in {}", dir.display())))?;
    let raw = std::fs::read_to_string(&template)?;
    let mut doc: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| AppError::InvalidInput(format!("{}: {e}", template.display())))?;
    let Some(body) = doc
        .as_object_mut()
        .and_then(|m| m.values_mut().next())
        .and_then(Value::as_object_mut)
        .and_then(|entry| entry.get_mut("Body"))
        .and_then(Value::as_object_mut)
    else {
        return Ok(PackConversion::NotApplicable);
    };
    if role(body, "2_Diffuse_Skin_Mask").is_some() {
        return Ok(PackConversion::AlreadyHybrid);
    }
    let (Some(diffuse), Some(skin)) = (role(body, "Diffuse"), role(body, "Skin")) else {
        return Ok(PackConversion::NotApplicable);
    };
    let diffuse = image::open(dir.join(diffuse))?.to_rgba8();
    let skin = image::open(dir.join(skin))?.to_rgba8();
    let (art, zones) = from_alphaconsole(&diffuse, &skin);
    art.save(dir.join(ART_FILE))?;
    zones.save(dir.join(ZONES_FILE))?;
    body.insert("1_Diffuse_Skin".into(), Value::String(ART_FILE.into()));
    body.insert(
        "2_Diffuse_Skin_Mask".into(),
        Value::String(ZONES_FILE.into()),
    );
    let out = serde_json::to_string_pretty(&doc).map_err(|e| AppError::Internal(e.to_string()))?;
    std::fs::write(&template, out)?;
    Ok(PackConversion::Converted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_alphaconsole_markers_to_zones_and_art() {
        let skin = RgbaImage::from_fn(4, 1, |x, _| match x {
            0 => Rgba([255, 0, 0, 0]),   // primary
            1 => Rgba([255, 0, 0, 255]), // accent
            2 => Rgba([43, 0, 0, 255]),  // #2B0000: art
            _ => Rgba([0, 0, 255, 255]), // windows: art
        });
        let diffuse = RgbaImage::from_pixel(4, 1, Rgba([10, 20, 30, 255]));
        let (art, zones) = from_alphaconsole(&diffuse, &skin);
        assert_eq!(zones.get_pixel(0, 0), &PRIMARY);
        assert_eq!(zones.get_pixel(1, 0), &ACCENT);
        assert_eq!(zones.get_pixel(2, 0), &ART);
        assert_eq!(zones.get_pixel(3, 0), &ART);
        // Painted zones get white art (the colour shows as picked).
        assert_eq!(art.get_pixel(0, 0).0, [255, 255, 255, 0]);
        assert_eq!(art.get_pixel(1, 0).0, [255, 255, 255, 0]);
        // Elsewhere the art keeps its colours.
        assert_eq!(art.get_pixel(2, 0).0, [10, 20, 30, 0]);
    }

    #[test]
    fn converts_a_pack_folder_once() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        RgbaImage::from_pixel(4, 4, Rgba([9, 9, 9, 255]))
            .save(d.join("body.png"))
            .unwrap();
        RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 0]))
            .save(d.join("skin.png"))
            .unwrap();
        std::fs::write(
            d.join("Template.json"),
            r#"{"Pack (Octane)":{"BodyID":23,"Body":{"Diffuse":"body.png","Skin":"skin.png"}}}"#,
        )
        .unwrap();
        assert_eq!(convert_pack_dir(d).unwrap(), PackConversion::Converted);
        assert!(d.join(ART_FILE).is_file() && d.join(ZONES_FILE).is_file());
        let t = std::fs::read_to_string(d.join("Template.json")).unwrap();
        assert!(t.contains("2_Diffuse_Skin_Mask") && t.contains("\"Skin\""));
        assert_eq!(convert_pack_dir(d).unwrap(), PackConversion::AlreadyHybrid);
    }

    #[test]
    fn mask_is_resampled_to_the_art_size() {
        let skin = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 0]));
        let diffuse = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
        let (art, zones) = from_alphaconsole(&diffuse, &skin);
        assert_eq!(zones.dimensions(), (8, 8));
        assert_eq!(art.dimensions(), (8, 8));
        assert!(zones.pixels().all(|p| *p == PRIMARY));
    }
}
