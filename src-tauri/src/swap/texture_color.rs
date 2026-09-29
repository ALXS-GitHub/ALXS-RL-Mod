//! "Exact colour" for items whose colour is painted into a texture.
//!
//! The Alpha wheels' rim is a gold diffuse texture tinted by material
//! colours (`RimColor`, `CustomColor`): recolouring those colours alone
//! multiplies the target by the gold (red turns orange, black stays golden).
//! Here the item's colour textures become neutral — each pixel keeps its
//! brightness (HSV value) and loses its hue — so the recoloured material
//! colours give the item the target colour, details and shading kept.
//!
//! Colour textures: textures a material instance binds to a `*Diffuse*`
//! parameter, or named `*_D` / `*Diffuse*`. Normal maps, masks and textures
//! that are already neutral are left. When the material has no colour to
//! tint the texture with (Alpha hat), the target is baked into the texture. The new pixels go to our own texture
//! cache (`decals::pipeline::build_swap`): one per swapped package, named
//! after it, so replacing a swap overwrites its cache.

use std::path::Path;

use image::RgbaImage;

use crate::base::AppResult;
use crate::decals::pipeline::{self, Select, SwapJob, SwapSpec};
use crate::upk::package::Body;
use crate::upk::recolor::{self, ColorMode, Target};
use crate::upk::texture::{self, Texture2D};
use crate::upk::{material, props, reader, thumbs, KeyRing, Package};

/// Average saturation below which a texture already counts as neutral.
const NEUTRAL: f32 = 0.12;
/// Preview size (the nearest mip at or under it is decoded).
const PREVIEW_MAX: u32 = 256;

/// The package with neutral colour textures, and the texture cache they
/// now live in (`None` when the item has no colour texture).
pub struct NeutralTextures {
    pub package: Vec<u8>,
    /// (`<name>` of `<name>.tfc`, bytes)
    pub cache: Option<(String, Vec<u8>)>,
    pub textures: Vec<String>,
}

fn is_colour_texture(parameter: &str, texture: &str) -> bool {
    let (p, t) = (parameter.to_ascii_lowercase(), texture.to_ascii_lowercase());
    let named = p.contains("diffuse") || t.ends_with("_d") || t.contains("diffuse");
    named && !p.contains("normal") && !t.ends_with("_n")
}

/// Cache name prefix of a swapped package: stable per package, so the next
/// swap of the same item reuses (overwrites) the same cache file.
pub fn cache_prefix(package: &str) -> String {
    let hash = package
        .to_ascii_lowercase()
        .bytes()
        .fold(0x811c_9dc5u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        });
    format!("Ax{hash:08X}")
}

/// Average HSV saturation of the visible pixels.
fn mean_saturation(img: &RgbaImage) -> f32 {
    let (mut sum, mut n) = (0.0f32, 0u32);
    for p in img.pixels() {
        let max = p[0].max(p[1]).max(p[2]);
        if max < 8 {
            continue;
        }
        let min = p[0].min(p[1]).min(p[2]);
        sum += f32::from(max - min) / f32::from(max);
        n += 1;
    }
    if n == 0 {
        0.0
    } else {
        sum / n as f32
    }
}

/// Each pixel's brightness (HSV value) as a grey; alpha kept.
pub fn neutral(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let v = p[0].max(p[1]).max(p[2]);
        p.0 = [v, v, v, p[3]];
    }
    out
}

/// `img` × `colour` (linear light) / `scale`, back to sRGB; alpha kept.
fn tinted(img: &RgbaImage, colour: [f32; 3], scale: f32) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        for c in 0..3 {
            let lin = recolor::srgb_to_linear(f32::from(p[c]) / 255.0) * colour[c] / scale;
            p[c] = (recolor::linear_to_srgb(lin.clamp(0.0, 1.0)) * 255.0).round() as u8;
        }
    }
    out
}

/// The material colour that tints a texture: the most saturated non-black
/// colour of its material instance (`None`: nothing tints it).
fn main_tint(pkg: &Package, body: &Body, mic: &str) -> Option<[f32; 3]> {
    vector_params(pkg, body, mic)
        .into_iter()
        .filter(|c| c.iter().all(|v| v.is_finite()) && c.iter().copied().fold(0.0, f32::max) > 0.0)
        .max_by(|a, b| {
            let sat = |c: &[f32; 3]| {
                let max = c.iter().copied().fold(0.0, f32::max);
                (max - c.iter().copied().fold(f32::MAX, f32::min)) / max
            };
            sat(a).total_cmp(&sat(b))
        })
}

/// Colour textures of the package, with the material instance binding them.
fn colour_textures<'a>(pkg: &Package, textures: &'a [Texture2D]) -> Vec<(&'a Texture2D, String)> {
    let Ok(body) = pkg.body() else {
        return Vec::new();
    };
    let mut out: Vec<(&Texture2D, String)> = Vec::new();
    for p in material::texture_params(pkg, &body)
        .into_iter()
        .filter(|p| p.object > 0 && is_colour_texture(&p.parameter, &p.object_name))
    {
        let Some(t) = textures
            .iter()
            .find(|t| i32::try_from(t.export_index + 1).ok() == Some(p.object))
        else {
            continue;
        };
        if !out.iter().any(|(o, _)| o.export_index == t.export_index) {
            out.push((t, p.material));
        }
    }
    out
}

fn decode(cooked: &Path, body: &[u8], t: &Texture2D, max_width: u32) -> Option<RgbaImage> {
    let mip = t
        .mips
        .iter()
        .filter(|m| !m.is_empty() && m.width <= max_width)
        .max_by_key(|m| m.width)?;
    let raw = thumbs::read_mip(cooked, body, t.tfc_name.as_deref(), mip).ok()?;
    texture::decode(t.format, mip.width, mip.height, &raw).ok()
}

/// Makes the colour textures of `bytes` neutral, or bakes `target` into
/// those no material colour tints (see the module doc).
pub fn neutralize(
    bytes: &[u8],
    ring: &KeyRing,
    cooked: &Path,
    package_name: &str,
    target: Target,
) -> AppResult<NeutralTextures> {
    let baked = recolor::colorize_exact([1.0; 3], target.linear(), true);
    let unchanged = || NeutralTextures {
        package: bytes.to_vec(),
        cache: None,
        textures: Vec::new(),
    };
    let pkg = Package::open(bytes.to_vec(), ring)?;
    let body = pkg.body()?;
    let textures = pkg.textures(&body);
    let mut cache: Option<String> = None;
    let mut images: Vec<(String, RgbaImage)> = Vec::new();
    for (t, mic) in colour_textures(&pkg, &textures) {
        // One texture cache per swap: `build_swap` hijacks a single one.
        let Some(tfc) = t.tfc_name.clone() else {
            continue;
        };
        if cache.as_ref().is_some_and(|c| *c != tfc) {
            tracing::debug!(texture = %t.name, "colour texture in another cache, kept");
            continue;
        }
        let Some(img) = decode(cooked, &body.data, t, u32::MAX) else {
            continue;
        };
        if mean_saturation(&img) < NEUTRAL {
            continue;
        }
        cache = Some(tfc);
        let image = match main_tint(&pkg, &body, &mic) {
            Some(_) => neutral(&img),
            None => tinted(&neutral(&img), baked, 1.0),
        };
        images.push((t.name.clone(), image));
    }
    if images.is_empty() {
        return Ok(unchanged());
    }
    let read = |name: &str, offset: u64, len: usize| {
        crate::game::tfc::read_range(cooked, name, offset, len)
    };
    let swap = pipeline::build_swap(
        bytes.to_vec(),
        ring,
        &SwapSpec {
            renames: &[],
            target_key: None,
            cache_prefix: &cache_prefix(package_name),
            jobs: images
                .iter()
                .map(|(name, image)| SwapJob {
                    select: Select::Name(name.clone()),
                    image,
                })
                .collect(),
        },
        &read,
    )?;
    tracing::info!(
        textures = ?swap.textures,
        relocated = swap.relocated.len(),
        cache_bytes = swap.tfc.len(),
        "colour textures made neutral"
    );
    Ok(NeutralTextures {
        package: swap.package,
        cache: Some((swap.tfc_name, swap.tfc)),
        textures: swap.textures,
    })
}

/// Colour parameters (`VectorParameterValues`) of a material instance.
fn vector_params(pkg: &Package, body: &Body, mic: &str) -> Vec<[f32; 3]> {
    let Some(e) = pkg.exports.iter().find(|e| e.object_name == mic) else {
        return Vec::new();
    };
    let Some(pos) = body.export_pos(e) else {
        return Vec::new();
    };
    let body = &body.data[..];
    let Ok((tags, _)) = props::walk(body, pos + 4, &pkg.names) else {
        return Vec::new();
    };
    let Some(array) = props::find(&tags, "VectorParameterValues") else {
        return Vec::new();
    };
    let count = reader::read_i32(body, array.value_pos).unwrap_or(0).max(0);
    let mut at = array.value_pos + 4;
    let mut out = Vec::new();
    for _ in 0..count {
        let Ok((param, next)) = props::walk(body, at, &pkg.names) else {
            break;
        };
        at = next;
        if let Some(v) = props::find(&param, "ParameterValue").filter(|p| p.size == 16) {
            let f = |i: usize| {
                reader::read_u32(body, v.value_pos + 4 * i)
                    .map(f32::from_bits)
                    .unwrap_or(0.0)
            };
            out.push([f(0), f(1), f(2)]);
        }
    }
    out
}

/// Flat preview of the item's main colour texture with `target` applied
/// the way the game combines it (texture × material colour), without
/// lighting. `None` when the item has no colour texture.
pub fn preview(
    bytes: &[u8],
    ring: &KeyRing,
    cooked: &Path,
    target: Target,
    mode: ColorMode,
) -> AppResult<Option<RgbaImage>> {
    let pkg = Package::open(bytes.to_vec(), ring)?;
    let body = pkg.body()?;
    let textures = pkg.textures(&body);
    let Some((t, mic)) = colour_textures(&pkg, &textures).into_iter().next() else {
        return Ok(None);
    };
    let Some(img) = decode(cooked, &body.data, t, PREVIEW_MAX) else {
        return Ok(None);
    };
    // No material colour: shown as is (tint) or baked (exact).
    let tint = main_tint(&pkg, &body, &mic).unwrap_or([1.0; 3]);
    let tint_max = tint.iter().copied().fold(0.0, f32::max).max(1e-3);
    Ok(Some(match mode {
        ColorMode::Exact => tinted(
            &neutral(&img),
            recolor::colorize_exact(tint, target.linear(), true),
            tint_max,
        ),
        ColorMode::Tint => tinted(&img, recolor::colorize(tint, target), tint_max),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_textures_are_diffuse_ones() {
        assert!(is_colour_texture("RimDiffuse", "Alpha_D"));
        assert!(is_colour_texture("Diffuse", "Wheel_Rim"));
        assert!(!is_colour_texture("RimNormal", "Alpha_N"));
        assert!(!is_colour_texture("RimRGB", "Alpha_RGB"));
        assert!(!is_colour_texture("NormalDiffuseMix", "X_D"));
    }

    #[test]
    fn neutral_keeps_brightness_and_alpha() {
        let img = RgbaImage::from_pixel(2, 2, image::Rgba([200, 150, 20, 128]));
        let out = neutral(&img);
        assert_eq!(out.get_pixel(1, 1).0, [200, 200, 200, 128]);
        assert!(mean_saturation(&out) < NEUTRAL);
        assert!(mean_saturation(&img) > 0.8);
    }

    #[test]
    fn cache_prefix_is_stable_per_package() {
        assert_eq!(
            cache_prefix("WHEEL_AlphaRim_SF.upk"),
            cache_prefix("wheel_alpharim_sf.upk")
        );
        assert_ne!(
            cache_prefix("WHEEL_AlphaRim_SF.upk"),
            cache_prefix("WHEEL_Proto_SF.upk")
        );
        assert_eq!(cache_prefix("x").len(), 10);
    }
}
