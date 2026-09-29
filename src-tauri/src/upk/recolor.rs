//! Recolouring an item: every colour the package carries as data moves to
//! the target. Two modes ([`ColorMode`]):
//! - **tint**: the target's hue; saturation and brightness scaled by the
//!   target's (a pure colour such as `#ff0000` keeps them, so HDR glows and
//!   fades behave as before). Greys stay grey.
//! - **exact**: the target's hue and saturation (in linear colour, as the
//!   engine renders), brightness scaled by the target's. Material colours
//!   that are grey move too. Meant to go with neutral colour textures
//!   (`swap::texture_color`), so the item really shows the target.
//!
//! Colours that are data (and can therefore change without touching the
//! game's compiled shaders):
//! - particle colour modules (`ParticleModuleColor`, `…ColorOverLife`,
//!   `…ColorScaleOverLife`): `RawDistributionVector` lookup tables
//!   (`[min, max, r, g, b, r, g, b…]`), and the distribution objects they
//!   reference (`DistributionVector*`: `Constant`, `Min`, `Max`);
//! - material instances: `VectorParameterValues` (`LinearColor`).
//!
//! Colours hard-coded in a material's expressions are compiled into the
//! game's shaders and cannot change. Edits are float for float: the package
//! keeps its size.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::upk::chunks::StreamPatch;
use crate::upk::package::{Body, Package};
use crate::upk::props::{self, Prop};
use crate::upk::reader::{read_i32, read_u32};

/// Below this chroma / value ratio a colour counts as grey and is kept.
const GREY: f32 = 0.04;

/// How a custom colour is applied (see the module doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorMode {
    /// The item shows the target colour (colour textures made neutral).
    Exact,
    /// Only the data colours move (swaps saved before the modes existed).
    #[default]
    Tint,
}

/// A recolour target: hue in degrees, saturation and value in 0..=1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
}

impl Target {
    /// `#rrggbb` (the `#` is optional).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let raw = hex.trim().trim_start_matches('#');
        if raw.len() != 6 || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let n = u32::from_str_radix(raw, 16).ok()?;
        let rgb = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map(|c| c as f32 / 255.0);
        Some(Self::from_rgb(rgb))
    }

    /// The same colour in linear light (what material colours hold).
    pub fn linear(self) -> Self {
        let hex = self.to_hex();
        let n = u32::from_str_radix(&hex[1..], 16).unwrap_or(0);
        let rgb =
            [(n >> 16) & 255, (n >> 8) & 255, n & 255].map(|c| srgb_to_linear(c as f32 / 255.0));
        Self::from_rgb(rgb)
    }

    /// HSV of an RGB triple in 0..=1.
    pub fn from_rgb(rgb: [f32; 3]) -> Self {
        let max = rgb.iter().copied().fold(0.0, f32::max);
        let min = rgb.iter().copied().fold(1.0, f32::min);
        let d = max - min;
        let hue = if d == 0.0 {
            0.0
        } else if max == rgb[0] {
            60.0 * ((rgb[1] - rgb[2]) / d).rem_euclid(6.0)
        } else if max == rgb[1] {
            60.0 * ((rgb[2] - rgb[0]) / d + 2.0)
        } else {
            60.0 * ((rgb[0] - rgb[1]) / d + 4.0)
        };
        Self {
            hue,
            saturation: if max == 0.0 { 0.0 } else { d / max },
            value: max,
        }
    }

    /// `#rrggbb` of this colour.
    pub fn to_hex(self) -> String {
        let [r, g, b] = colorize(
            [1.0, 0.0, 0.0],
            Self {
                saturation: 1.0,
                value: 1.0,
                ..self
            },
        );
        let (s, v) = (self.saturation.clamp(0.0, 1.0), self.value.clamp(0.0, 1.0));
        let channel = |c: f32| ((v * (1.0 - s + s * c)) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}", channel(r), channel(g), channel(b))
    }

    /// A pure hue: the shape of earlier "tint" swaps.
    pub fn hue(hue: f32) -> Self {
        Self {
            hue,
            saturation: 1.0,
            value: 1.0,
        }
    }
}

/// `rgb` with the target's hue, its saturation scaled by the target's and
/// its value (max, HDR included) scaled by the target's.
pub fn colorize(rgb: [f32; 3], target: Target) -> [f32; 3] {
    let max = rgb.iter().copied().fold(f32::MIN, f32::max);
    let min = rgb.iter().copied().fold(f32::MAX, f32::min);
    // Black and non-finite values are left as they are.
    if !max.is_finite() || max <= 0.0 || (max - min) / max < GREY {
        return rgb;
    }
    let value = max * target.value.clamp(0.0, 1.0);
    let saturation = (max - min) / max * target.saturation.clamp(0.0, 1.0);
    hsv_to_rgb(target.hue, saturation, value)
}

/// sRGB → linear (IEC 61966-2-1).
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear → sRGB.
pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// `rgb` with the target's hue and saturation, its value (max, HDR
/// included) scaled by the target's. Greys move too when `greys` is set;
/// black and non-finite values never do.
pub fn colorize_exact(rgb: [f32; 3], target: Target, greys: bool) -> [f32; 3] {
    let max = rgb.iter().copied().fold(f32::MIN, f32::max);
    let min = rgb.iter().copied().fold(f32::MAX, f32::min);
    if !max.is_finite() || max <= 0.0 || (!greys && (max - min) / max < GREY) {
        return rgb;
    }
    hsv_to_rgb(
        target.hue,
        target.saturation.clamp(0.0, 1.0),
        max * target.value.clamp(0.0, 1.0),
    )
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let chroma = value * saturation;
    let min = value - chroma;
    let h = hue.rem_euclid(360.0) / 60.0;
    let x = chroma * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    [r + min, g + min, b + min]
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RecolorStats {
    /// Colours moved to the new hue.
    pub colors: usize,
    /// Exports that carried at least one colour.
    pub exports: usize,
}

fn f32_at(buf: &[u8], pos: usize) -> Option<f32> {
    read_u32(buf, pos).map(f32::from_bits)
}

struct Recolor<'a> {
    data: &'a [u8],
    target: Target,
    mode: ColorMode,
    patch: StreamPatch,
    stats: RecolorStats,
}

impl Recolor<'_> {
    /// Recolours the RGB triple at `pos`; true when it changed. `material`:
    /// a material colour (greys move too in the exact mode).
    fn triple(&mut self, pos: usize, material: bool) -> bool {
        let (Some(r), Some(g), Some(b)) = (
            f32_at(self.data, pos),
            f32_at(self.data, pos + 4),
            f32_at(self.data, pos + 8),
        ) else {
            return false;
        };
        let out = match self.mode {
            ColorMode::Tint => colorize([r, g, b], self.target),
            ColorMode::Exact => colorize_exact([r, g, b], self.target, material),
        };
        if out == [r, g, b] {
            return false;
        }
        let mut bytes = Vec::with_capacity(12);
        for v in out {
            bytes.extend(v.to_le_bytes());
        }
        self.patch.put(pos, bytes);
        self.stats.colors += 1;
        true
    }

    /// `RawDistributionVector`: recolours its baked lookup table and
    /// returns the distribution object it references (if any).
    fn raw_distribution(&mut self, prop: &Prop, pkg: &Package) -> (bool, Option<i32>) {
        let Ok((fields, _)) = props::walk(self.data, prop.value_pos, &pkg.names) else {
            return (false, None);
        };
        let object = props::find(&fields, "Distribution")
            .and_then(|p| p.as_i32(self.data))
            .filter(|&o| o != 0);
        let mut changed = false;
        if let Some(table) = props::find(&fields, "LookupTable") {
            let n = read_i32(self.data, table.value_pos).unwrap_or(0).max(0) as usize;
            // [min, max] then RGB triples.
            if n >= 5 && (n - 2) % 3 == 0 {
                for k in 0..(n - 2) / 3 {
                    changed |= self.triple(table.value_pos + 4 + (2 + k * 3) * 4, false);
                }
            }
        }
        (changed, object)
    }

    /// Properties of a `DistributionVector*` object: `Constant`, `Min`, `Max`.
    fn distribution(&mut self, fields: &[Prop]) -> bool {
        let mut changed = false;
        for p in fields {
            if p.inner.as_deref() == Some("Vector")
                && matches!(p.name.as_str(), "Constant" | "Min" | "Max")
                && p.size == 12
            {
                changed |= self.triple(p.value_pos, false);
            }
        }
        changed
    }

    /// `VectorParameterValues` of a material instance.
    fn material(&mut self, fields: &[Prop], pkg: &Package) -> bool {
        let Some(array) = props::find(fields, "VectorParameterValues") else {
            return false;
        };
        let count = read_i32(self.data, array.value_pos).unwrap_or(0).max(0);
        let mut at = array.value_pos + 4;
        let mut changed = false;
        for _ in 0..count {
            let Ok((param, next)) = props::walk(self.data, at, &pkg.names) else {
                break;
            };
            at = next;
            if let Some(value) = props::find(&param, "ParameterValue")
                .filter(|p| p.inner.as_deref() == Some("LinearColor") && p.size == 16)
            {
                changed |= self.triple(value.value_pos, true);
            }
        }
        changed
    }
}

/// Where an export's property block starts: after the `NetIndex` (4 bytes)
/// for most objects, after 8 bytes for some (distribution objects).
fn property_block(data: &[u8], pos: usize, pkg: &Package) -> Option<Vec<Prop>> {
    [4usize, 8].into_iter().find_map(|skip| {
        props::walk(data, pos + skip, &pkg.names)
            .ok()
            .map(|(p, _)| p)
    })
}

/// Builds the edits that move every data colour of `pkg` to `target`.
/// In the exact mode, `target` is taken as sRGB and converted to linear.
pub fn recolor_patch(
    pkg: &Package,
    body: &Body,
    target: Target,
    mode: ColorMode,
) -> (StreamPatch, RecolorStats) {
    let mut r = Recolor {
        data: &body.data,
        target: match mode {
            ColorMode::Tint => target,
            ColorMode::Exact => target.linear(),
        },
        mode,
        patch: StreamPatch::new(),
        stats: RecolorStats::default(),
    };
    // 1. Particle colour modules, collecting the distributions they use
    //    (other distributions — sizes, velocities — are not colours).
    let mut colour_distributions: HashSet<i32> = HashSet::new();
    for e in pkg.exports.iter().filter(|e| e.serial_size > 0) {
        let class = pkg.class_of(e);
        let is_colour_module = class.starts_with("ParticleModuleColor");
        let is_material = class == "MaterialInstanceConstant";
        if !is_colour_module && !is_material {
            continue;
        }
        let Some(pos) = body.export_pos(e) else {
            continue;
        };
        let Some(fields) = property_block(&body.data, pos, pkg) else {
            continue;
        };
        let changed = if is_material {
            r.material(&fields, pkg)
        } else {
            let mut changed = false;
            for p in fields
                .iter()
                .filter(|p| p.inner.as_deref() == Some("RawDistributionVector"))
            {
                let (c, object) = r.raw_distribution(p, pkg);
                changed |= c;
                colour_distributions.extend(object);
            }
            changed
        };
        r.stats.exports += usize::from(changed);
    }
    // 2. The distribution objects behind those modules.
    for e in pkg.exports.iter().filter(|e| e.serial_size > 0) {
        let object = i32::try_from(e.index + 1).unwrap_or(0);
        if !colour_distributions.contains(&object)
            || !pkg.class_of(e).starts_with("DistributionVector")
        {
            continue;
        }
        let Some(pos) = body.export_pos(e) else {
            continue;
        };
        let Some(fields) = property_block(&body.data, pos, pkg) else {
            continue;
        };
        let changed = r.distribution(&fields);
        r.stats.exports += usize::from(changed);
    }
    (r.patch, r.stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
    }

    fn hue(h: f32) -> Target {
        Target::hue(h)
    }

    #[test]
    fn colorize_keeps_value_and_saturation_for_pure_colours() {
        // Alpha Boost gold -> red: same max and min.
        assert!(close(colorize([1.5, 0.8, 0.2], hue(0.0)), [1.5, 0.2, 0.2]));
        assert!(close(
            colorize([2.5, 0.4, 0.125], hue(240.0)),
            [0.125, 0.125, 2.5]
        ));
        assert!(close(
            colorize([1.0, 0.0, 0.0], hue(120.0)),
            [0.0, 1.0, 0.0]
        ));
        // Hue 60: yellow.
        assert!(close(colorize([2.0, 0.0, 0.0], hue(60.0)), [2.0, 2.0, 0.0]));
    }

    #[test]
    fn colorize_scales_by_the_target() {
        let red = Target::from_hex("#ff0000").unwrap();
        assert!(close(colorize([1.5, 0.8, 0.2], red), [1.5, 0.2, 0.2]));
        // Half-bright red halves the value.
        let dark = Target::from_hex("#800000").unwrap();
        assert!(close(
            colorize([2.0, 0.0, 0.0], dark),
            [2.0 * 128.0 / 255.0, 0.0, 0.0]
        ));
        // Pastel red (s ~ 0.5) roughly halves the saturation.
        let pastel = Target::from_hex("#ff8080").unwrap();
        let out = colorize([2.0, 0.0, 0.0], pastel);
        assert!((out[0] - 2.0).abs() < 1e-4 && (out[1] - out[2]).abs() < 1e-4);
        assert!((out[1] - 2.0 * (128.0 / 255.0)).abs() < 1e-3);
    }

    #[test]
    fn writes_targets_as_hex() {
        for hex in ["#ff0000", "#e3a23a", "#2b1245", "#808080", "#000000"] {
            let t = Target::from_hex(hex).unwrap();
            assert_eq!(t.to_hex(), hex);
        }
        assert_eq!(Target::hue(120.0).to_hex(), "#00ff00");
    }

    #[test]
    fn parses_targets() {
        let t = Target::from_hex("e3a23b").unwrap();
        assert!((t.hue - 37.0).abs() < 1.0);
        assert!(Target::from_hex("#12345").is_none());
        assert!(Target::from_hex("#gggggg").is_none());
    }

    #[test]
    fn exact_takes_the_target_saturation_and_moves_material_greys() {
        let red = Target::from_hex("#ff0000").unwrap();
        // Gold (low saturation) -> fully saturated red, same brightness.
        let out = colorize_exact([0.75, 0.67, 0.48], red, false);
        assert!(close(out, [0.75, 0.0, 0.0]), "{out:?}");
        // A grey material colour moves; a grey particle colour does not.
        assert!(close(
            colorize_exact([0.5, 0.5, 0.5], red, true),
            [0.5, 0.0, 0.0]
        ));
        assert!(close(
            colorize_exact([0.5, 0.5, 0.5], red, false),
            [0.5, 0.5, 0.5]
        ));
        // Black target: black.
        let black = Target::from_hex("#000000").unwrap();
        assert!(close(
            colorize_exact([0.75, 0.67, 0.48], black, true),
            [0.0; 3]
        ));
    }

    #[test]
    fn linear_target_matches_srgb_decoding() {
        let pale = Target::from_hex("#80c0ff").unwrap().linear();
        let rgb = hsv_to_rgb(pale.hue, pale.saturation, pale.value);
        let expected = [0x80, 0xc0, 0xff].map(|c| srgb_to_linear(c as f32 / 255.0));
        assert!(close(rgb, expected), "{rgb:?} vs {expected:?}");
        assert!((linear_to_srgb(srgb_to_linear(0.5)) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn colorize_leaves_greys_and_black() {
        assert_eq!(colorize([0.5, 0.5, 0.5], hue(0.0)), [0.5, 0.5, 0.5]);
        assert_eq!(colorize([1.0, 1.0, 0.99], hue(0.0)), [1.0, 1.0, 0.99]);
        assert_eq!(colorize([0.0, 0.0, 0.0], hue(0.0)), [0.0, 0.0, 0.0]);
    }
}
