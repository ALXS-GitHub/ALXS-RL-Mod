//! Recolouring an item (experimental): every colour the package carries as
//! data is moved to one hue, keeping its brightness (HDR intensity) and
//! saturation, so glows and fades behave as before. Greys stay grey.
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

use crate::upk::chunks::StreamPatch;
use crate::upk::package::{Body, Package};
use crate::upk::props::{self, Prop};
use crate::upk::reader::{read_i32, read_u32};

/// Below this chroma / value ratio a colour counts as grey and is kept.
const GREY: f32 = 0.04;

/// `rgb` moved to `hue` (degrees), same max (value) and min (so the same
/// saturation and HDR intensity).
pub fn colorize(rgb: [f32; 3], hue: f32) -> [f32; 3] {
    let max = rgb.iter().copied().fold(f32::MIN, f32::max);
    let min = rgb.iter().copied().fold(f32::MAX, f32::min);
    // Black and non-finite values are left as they are.
    if !max.is_finite() || max <= 0.0 || (max - min) / max < GREY {
        return rgb;
    }
    let chroma = max - min;
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
    hue: f32,
    patch: StreamPatch,
    stats: RecolorStats,
}

impl Recolor<'_> {
    /// Recolours the RGB triple at `pos`; true when it changed.
    fn triple(&mut self, pos: usize) -> bool {
        let (Some(r), Some(g), Some(b)) = (
            f32_at(self.data, pos),
            f32_at(self.data, pos + 4),
            f32_at(self.data, pos + 8),
        ) else {
            return false;
        };
        let out = colorize([r, g, b], self.hue);
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
                    changed |= self.triple(table.value_pos + 4 + (2 + k * 3) * 4);
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
                changed |= self.triple(p.value_pos);
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
                changed |= self.triple(value.value_pos);
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

/// Builds the edits that move every data colour of `pkg` to `hue` (degrees).
pub fn recolor_patch(pkg: &Package, body: &Body, hue: f32) -> (StreamPatch, RecolorStats) {
    let mut r = Recolor {
        data: &body.data,
        hue,
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

    #[test]
    fn colorize_keeps_value_and_saturation() {
        // Alpha Boost gold → red: same max and min.
        assert!(close(colorize([1.5, 0.8, 0.2], 0.0), [1.5, 0.2, 0.2]));
        assert!(close(
            colorize([2.5, 0.4, 0.125], 240.0),
            [0.125, 0.125, 2.5]
        ));
        assert!(close(colorize([1.0, 0.0, 0.0], 120.0), [0.0, 1.0, 0.0]));
        // Hue 60: yellow.
        assert!(close(colorize([2.0, 0.0, 0.0], 60.0), [2.0, 2.0, 0.0]));
    }

    #[test]
    fn colorize_leaves_greys_and_black() {
        assert_eq!(colorize([0.5, 0.5, 0.5], 0.0), [0.5, 0.5, 0.5]);
        assert_eq!(colorize([1.0, 1.0, 0.99], 0.0), [1.0, 1.0, 0.99]);
        assert_eq!(colorize([0.0, 0.0, 0.0], 0.0), [0.0, 0.0, 0.0]);
    }
}
