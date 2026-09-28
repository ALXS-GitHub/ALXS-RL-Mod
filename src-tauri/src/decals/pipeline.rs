//! Paintable custom decal — the pipeline validated in game (commit 74da7c1 of
//! the previous app): the pack's colour-zone **mask** replaces the donor's
//! `_RGB` texture, so the game paints the pack's shapes with the player's
//! primary / accent colours.
//!
//! 1. Open the donor package (stock multi-colour Force decal) and locate its
//!    `_RGB` mask texture and the TFC-stored mips of that texture.
//! 2. Encode the user's mask PNG to the mask's pixel format at each TFC mip
//!    size; lay the (chunked-zlib) mips out in a small new texture cache.
//! 3. Point each mip record (elem count, size on disk, offset) at the new
//!    cache — body blocks recompressed in place, file size unchanged.
//! 4. Rename the donor's package names to the owned target slot, and the
//!    `Textures7` cache name to `MyDecal02` so the mask is read from our file.
//!
//! The pack's diffuse PNG is not used here: literal colours need the other
//! stock material, see [`build_hybrid`].

use std::collections::hash_map::{Entry, HashMap};

use image::RgbaImage;

use crate::base::{AppError, AppResult};
use crate::upk::chunks::{self, StreamPatch};
use crate::upk::keys::AesKey;
use crate::upk::rename;
use crate::upk::texture::{self, PixelFormat, Texture2D};
use crate::upk::{KeyRing, Package, Rename, UpkError};

/// Cache name the donor's mask is bound to, and our same-length replacement.
pub const MASK_TFC_SOURCE: &str = "Textures7";
pub const MASK_TFC_CUSTOM: &str = "MyDecal02";

#[derive(Debug)]
pub struct MaskSwap {
    /// Modified package, to be written to `mods/<target>_SF.upk`.
    pub package: Vec<u8>,
    /// New texture cache, to be written to `CookedPCConsole/MyDecal02.tfc`.
    pub tfc: Vec<u8>,
    pub mask_texture: String,
    pub mips_replaced: usize,
}

fn is_mask(t: &Texture2D) -> bool {
    t.name.to_ascii_lowercase().ends_with("_rgb")
}

/// Pure transformation: donor bytes + mask image → new package + cache.
/// `target_key`: the AES key of the stock package being replaced (see
/// [`reencrypt_for_target`]).
pub fn build(
    donor: Vec<u8>,
    ring: &KeyRing,
    target_key: Option<AesKey>,
    required: &[Rename],
    optional: &[Rename],
    mask: &RgbaImage,
) -> AppResult<MaskSwap> {
    let mut pkg = Package::open(donor, ring)?;
    let body = pkg.body()?;
    let textures = pkg.textures(&body);

    let target = textures
        .iter()
        .find(|t| is_mask(t) && t.mips.iter().any(|m| m.in_tfc() && !m.is_empty()))
        .ok_or_else(|| AppError::Unsupported("donor has no texture-cache mask (_RGB)".into()))?;
    if !matches!(target.format, PixelFormat::Dxt1 | PixelFormat::Dxt5) {
        return Err(AppError::Unsupported(format!(
            "mask format {} not supported",
            target.format_name
        )));
    }
    // Every other texture bound to the hijacked cache would lose its data.
    if let Some(other) = textures.iter().find(|t| {
        t.export_index != target.export_index
            && t.tfc_name.as_deref() == Some(MASK_TFC_SOURCE)
            && t.mips.iter().any(|m| m.in_tfc())
    }) {
        return Err(AppError::Unsupported(format!(
            "{} shares the mask texture cache",
            other.name
        )));
    }

    // 2. Encode + lay out the new cache.
    let tfc_mips: Vec<&texture::Mip> = target
        .mips
        .iter()
        .filter(|m| m.in_tfc() && !m.is_empty())
        .collect();
    let dims: Vec<(u32, u32)> = tfc_mips.iter().map(|m| (m.width, m.height)).collect();
    let encoded = texture::encode_chain(mask, &dims, target.format)?;
    let mut tfc = Vec::new();
    let mut patch = StreamPatch::new();
    for (mip, data) in tfc_mips.iter().zip(&encoded) {
        let stored = if mip.compressed() {
            chunks::wrap_chunked(data)?
        } else {
            data.clone()
        };
        let offset = tfc.len() as u64;
        patch.put(mip.elem_field(), (data.len() as u32).to_le_bytes());
        patch.put(mip.size_field(), (stored.len() as u32).to_le_bytes());
        patch.put(mip.offset_field(), offset.to_le_bytes());
        tfc.extend(stored);
    }

    // 4. Renames on the header (indices never move; table parsed once).
    let table = pkg.names.clone();
    for rule in required {
        rename::pad(&mut pkg.header.plain, &table, rule)?;
    }
    for rule in optional {
        match rename::pad(&mut pkg.header.plain, &table, rule) {
            Ok(()) | Err(UpkError::NameNotFound(_)) => {}
            Err(e) => return Err(e.into()),
        }
    }
    rename::pad(
        &mut pkg.header.plain,
        &table,
        &Rename::new(MASK_TFC_SOURCE, MASK_TFC_CUSTOM),
    )
    .map_err(|e| AppError::Unsupported(format!("cache hijack: {e}")))?;
    pkg.refresh_names();
    reencrypt_for_target(&mut pkg, target_key);

    // 3. Header + patched body blocks, same size as the donor.
    let mut out = pkg.header_bytes()?;
    patch.apply(&mut out, &body.map)?;
    pkg.seal(&mut out)?;
    debug_assert_eq!(out.len(), pkg.bytes.len());

    Ok(MaskSwap {
        package: out,
        tfc,
        mask_texture: target.name.clone(),
        mips_replaced: tfc_mips.len(),
    })
}

/// The game decrypts a package with the key registered for its *name*: a
/// donor renamed to the target slot must be encrypted with the target's key
/// (skin_octane_classylady and Skin_Octane_Stars use different keys; the
/// game then rejects the file as "implausible compressed-chunk count").
fn reencrypt_for_target(pkg: &mut Package, target_key: Option<AesKey>) {
    if let (Some(key), Some(_)) = (target_key, pkg.header.key) {
        pkg.header.key = Some(key);
    }
}

/// Loads a mask PNG (any size; resized per mip).
pub fn load_mask(path: &std::path::Path) -> AppResult<RgbaImage> {
    Ok(image::open(path)?.to_rgba8())
}

// ── Texture swaps: hybrid decals, universal decals, the ball ────────────

/// Texture parameters of `Body_Paintable_Diffuse_Mat`, the stock material
/// behind full-colour decals (Germany, Baroque…) and some paintable ones
/// (ClassyLady…). Its own parameter descriptions: `1_Diffuse_Skin` is the
/// art ("A is Highlight mask"); `2_Diffuse_Skin_Mask` is "R Team, G Accent,
/// B Paint, A Animated". Zones get the chosen colour over the desaturated
/// art; where the mask is black the art shows in its own colours.
/// Universal decals use the same material with `TrimSheet` (logo sheet).
pub const ART_PARAM: &str = "1_Diffuse_Skin";
pub const ZONES_PARAM: &str = "2_Diffuse_Skin_Mask";
pub const TRIM_PARAM: &str = "TrimSheet";

/// Which texture(s) of the donor an image replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Select {
    /// Every local texture bound to this MIC texture parameter.
    Param(String),
    /// The texture export with this name (case-insensitive).
    Name(String),
}

impl std::fmt::Display for Select {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Select::Param(p) => write!(f, "parameter {p}"),
            Select::Name(n) => write!(f, "texture {n}"),
        }
    }
}

pub struct SwapJob<'a> {
    pub select: Select,
    pub image: &'a RgbaImage,
}

pub struct SwapSpec<'a> {
    /// Package renames (donor → target slot); longer targets are re-pointed.
    pub renames: &'a [Rename],
    /// AES key of the stock package being replaced (see [`reencrypt_for_target`]).
    pub target_key: Option<AesKey>,
    /// Prefix of our texture cache name (`AlxsDecal` → `AlxsDecal.tfc`).
    pub cache_prefix: &'a str,
    pub jobs: Vec<SwapJob<'a>>,
}

#[derive(Debug)]
pub struct TextureSwap {
    /// Modified package (same size as the donor).
    pub package: Vec<u8>,
    /// New texture cache, to be written to `CookedPCConsole/<tfc_name>.tfc`.
    pub tfc: Vec<u8>,
    pub tfc_name: String,
    /// Textures replaced by the images.
    pub textures: Vec<String>,
    /// Other textures of the hijacked cache, copied into ours unchanged.
    pub relocated: Vec<String>,
    pub mips_replaced: usize,
    /// The small mips stored in the package were replaced too (they stay
    /// stock when the patched blocks no longer fit their budget).
    pub inline_replaced: bool,
}

/// Same-length name for our texture cache (`Textures3` + `AlxsDecal` →
/// `AlxsDecal`, `Textures2` + `AlxsBall` → `AlxsBall0`).
pub fn custom_cache_name(prefix: &str, source_len: usize) -> String {
    let base = format!("{prefix}0123456789");
    base[..source_len.clamp(4, base.len())].to_string()
}

fn encodable(format: PixelFormat) -> bool {
    matches!(
        format,
        PixelFormat::Dxt1 | PixelFormat::Dxt5 | PixelFormat::A8R8G8B8
    )
}

/// Reads `len` bytes at `offset` of a stock texture cache (`<name>.tfc`).
pub type CacheReader<'a> = &'a dyn Fn(&str, u64, usize) -> AppResult<Vec<u8>>;

/// Replaces textures of `donor` with images. The replaced textures' texture
/// cache name is hijacked to our own cache; every other texture of the
/// package stored in that cache is copied into ours unchanged (read with
/// `read_cache`), so nothing else of the package changes. The package size
/// never changes.
pub fn build_swap(
    donor: Vec<u8>,
    ring: &KeyRing,
    spec: &SwapSpec<'_>,
    read_cache: CacheReader<'_>,
) -> AppResult<TextureSwap> {
    let mut pkg = Package::open(donor, ring)?;
    let body = pkg.body()?;
    let textures = pkg.textures(&body);
    let params = crate::upk::material::texture_params(&pkg, &body);

    // (texture, job index)
    let mut jobs: Vec<(&Texture2D, usize)> = Vec::new();
    for (i, job) in spec.jobs.iter().enumerate() {
        let found: Vec<&Texture2D> = match &job.select {
            Select::Param(name) => params
                .iter()
                .filter(|p| p.parameter.eq_ignore_ascii_case(name))
                .filter_map(|p| {
                    textures
                        .iter()
                        .find(|t| i32::try_from(t.export_index + 1).ok() == Some(p.object))
                })
                .collect(),
            Select::Name(name) => textures
                .iter()
                .filter(|t| t.name.eq_ignore_ascii_case(name))
                .collect(),
        };
        if found.is_empty() {
            return Err(AppError::Unsupported(format!(
                "donor has no local texture for {}",
                job.select
            )));
        }
        for t in found {
            if !jobs.iter().any(|(j, _)| j.export_index == t.export_index) {
                jobs.push((t, i));
            }
        }
    }
    if let Some((t, _)) = jobs.iter().find(|(t, _)| !encodable(t.format)) {
        return Err(AppError::Unsupported(format!(
            "{}: format {} not supported",
            t.name, t.format_name
        )));
    }

    // One cache for every replaced texture.
    let in_tfc = |t: &Texture2D| t.mips.iter().any(|m| m.in_tfc() && !m.is_empty());
    let mut caches: Vec<&str> = jobs
        .iter()
        .filter(|(t, _)| in_tfc(t))
        .filter_map(|(t, _)| t.tfc_name.as_deref())
        .collect();
    caches.sort_unstable();
    caches.dedup();
    let [source_cache] = caches[..] else {
        return Err(AppError::Unsupported(format!(
            "replaced textures use {} texture caches, expected one",
            caches.len()
        )));
    };
    let tfc_name = custom_cache_name(spec.cache_prefix, source_cache.len());
    // Hijacking the cache name re-points every texture of that cache: one
    // we cannot relocate would read past the end of ours and crash the game.
    let stranded: Vec<&str> = pkg
        .exports
        .iter()
        .filter(|e| pkg.class_of(e) == "Texture2D")
        .filter(|e| !textures.iter().any(|t| t.export_index == e.index))
        .filter(|e| {
            body.export_pos(e).is_some_and(|pos| {
                texture::cache_name_of(&body.data, pos, &pkg.names).as_deref() == Some(source_cache)
            })
        })
        .map(|e| e.object_name.as_str())
        .collect();
    if !stranded.is_empty() {
        return Err(AppError::Unsupported(format!(
            "textures of {source_cache} could not be read: {}",
            stranded.join(", ")
        )));
    }

    // Encode each (image, format, mip chain) once: a mask's Painted variant
    // has the same dimensions as the regular one.
    type ChainKey = (usize, PixelFormat, Vec<(u32, u32)>);
    let mut encoded: HashMap<ChainKey, Vec<Vec<u8>>> = HashMap::new();
    let mut tfc = Vec::new();
    let mut records = StreamPatch::new();
    let mut inline: Vec<(usize, Vec<u8>)> = Vec::new();
    let mut mips_replaced = 0;
    for (t, job) in &jobs {
        let mips: Vec<&texture::Mip> = t.mips.iter().filter(|m| !m.is_empty()).collect();
        let dims: Vec<(u32, u32)> = mips.iter().map(|m| (m.width, m.height)).collect();
        let chain = match encoded.entry((*job, t.format, dims)) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(v) => {
                let dims = v.key().2.clone();
                v.insert(texture::encode_chain(
                    spec.jobs[*job].image,
                    &dims,
                    t.format,
                )?)
            }
        };
        for (mip, data) in mips.iter().zip(chain.iter()) {
            if mip.in_tfc() {
                let stored = if mip.compressed() {
                    chunks::wrap_chunked(data)?
                } else {
                    data.clone()
                };
                let offset = tfc.len() as u64;
                records.put(mip.elem_field(), (data.len() as u32).to_le_bytes());
                records.put(mip.size_field(), (stored.len() as u32).to_le_bytes());
                records.put(mip.offset_field(), offset.to_le_bytes());
                tfc.extend(stored);
                mips_replaced += 1;
            } else if let Some(at) = mip.data_pos {
                if !mip.compressed() && mip.size_on_disk as usize == data.len() {
                    inline.push((at, data.clone()));
                }
            }
        }
    }

    // Relocate the other textures of the hijacked cache: same bytes, new
    // offsets in our cache.
    let mut relocated = Vec::new();
    for t in textures.iter().filter(|t| {
        in_tfc(t)
            && t.tfc_name.as_deref() == Some(source_cache)
            && !jobs.iter().any(|(j, _)| j.export_index == t.export_index)
    }) {
        for mip in t.mips.iter().filter(|m| m.in_tfc() && !m.is_empty()) {
            let bytes = read_cache(source_cache, mip.offset_in_file, mip.size_on_disk as usize)?;
            records.put(mip.offset_field(), (tfc.len() as u64).to_le_bytes());
            tfc.extend(bytes);
        }
        relocated.push(t.name.clone());
    }

    // Package renames (re-pointing when longer), then the cache name
    // (padded in place: the body references it by index).
    if !spec.renames.is_empty() {
        rename::rename_in_header(&mut pkg, &body.data, spec.renames)?;
        pkg.refresh_names();
    }
    let table = pkg.names.clone();
    rename::pad(
        &mut pkg.header.plain,
        &table,
        &Rename::new(source_cache, tfc_name.clone()),
    )
    .map_err(|e| AppError::Unsupported(format!("cache hijack: {e}")))?;
    pkg.refresh_names();
    reencrypt_for_target(&mut pkg, spec.target_key);

    // Body: mip records, plus the inline mips when every block still fits.
    let header = pkg.header_bytes()?;
    let mut full = StreamPatch::new();
    records.copy_into(&mut full);
    for (at, data) in &inline {
        full.put(*at, data.clone());
    }
    let mut out = header.clone();
    let inline_replaced = match full.apply(&mut out, &body.map) {
        Ok(_) => !inline.is_empty(),
        Err(UpkError::BudgetExceeded { .. }) => {
            tracing::warn!("inline mips kept stock: patched blocks exceed their budget");
            out = header;
            records.apply(&mut out, &body.map)?;
            false
        }
        Err(e) => return Err(e.into()),
    };
    pkg.seal(&mut out)?;
    debug_assert_eq!(out.len(), pkg.bytes.len());

    Ok(TextureSwap {
        package: out,
        tfc,
        tfc_name,
        textures: jobs.iter().map(|(t, _)| t.name.clone()).collect(),
        relocated,
        mips_replaced,
        inline_replaced,
    })
}

/// Hybrid decal: art + zone mask into a `Body_Paintable_Diffuse_Mat` donor.
pub fn build_hybrid(
    donor: Vec<u8>,
    ring: &KeyRing,
    target_key: Option<AesKey>,
    renames: &[Rename],
    art: &RgbaImage,
    zones: &RgbaImage,
    read_cache: CacheReader<'_>,
) -> AppResult<TextureSwap> {
    build_swap(
        donor,
        ring,
        &SwapSpec {
            renames,
            target_key,
            cache_prefix: "AlxsDecal",
            jobs: vec![
                SwapJob {
                    select: Select::Param(ART_PARAM.into()),
                    image: art,
                },
                SwapJob {
                    select: Select::Param(ZONES_PARAM.into()),
                    image: zones,
                },
            ],
        },
        read_cache,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upk::chunks::{unwrap_chunked, wrap_chunked};
    use crate::upk::crypto::ecb_encrypt;
    use crate::upk::props::tests::{texture_props, NAMES};
    use crate::upk::summary::PACKAGE_MAGIC;
    use crate::upk::texture::{BULK_COMPRESSED_ZLIB, BULK_SEPARATE_FILE};
    use base64::Engine as _;

    const KEY: [u8; 32] = [42u8; 32];
    const EXTRA: &[&str] = &[
        "Core",
        "Class",
        "Texture2D",
        "skin_octane_galefire_SF",
        "skin_octane_galefire",
        "Skin_Octane_GaleFire_RGB",
    ];

    fn names_bytes() -> (Vec<u8>, Vec<&'static str>) {
        let all: Vec<&str> = NAMES.iter().chain(EXTRA).copied().collect();
        (crate::upk::names::tests::build(&all), all)
    }

    fn idx(all: &[&str], n: &str) -> i32 {
        all.iter().position(|x| *x == n).unwrap() as i32
    }

    fn tfc_mip(dim: i32, offset: u64) -> Vec<u8> {
        let elem = dim.max(4) / 4 * (dim.max(4) / 4) * 16;
        let mut b = (BULK_SEPARATE_FILE | BULK_COMPRESSED_ZLIB | 0x10000)
            .to_le_bytes()
            .to_vec();
        b.extend(elem.to_le_bytes());
        b.extend(1234i32.to_le_bytes());
        b.extend(offset.to_le_bytes());
        b.extend(dim.to_le_bytes());
        b.extend(dim.to_le_bytes());
        b
    }

    /// Encrypted donor with one `_RGB` Texture2D (2 TFC mips: 8² and 4²).
    fn fake_donor() -> Vec<u8> {
        let (names, all) = names_bytes();
        let mut import = Vec::new();
        for (n, num) in [("Core", 0i32), ("Class", 0i32)] {
            import.extend(idx(&all, n).to_le_bytes());
            import.extend(num.to_le_bytes());
        }
        import.extend(0i32.to_le_bytes()); // outer
        import.extend(idx(&all, "Texture2D").to_le_bytes());
        import.extend(0i32.to_le_bytes());

        // Body stream: the texture serial.
        let mut serial = (-1i32).to_le_bytes().to_vec();
        serial.extend(texture_props(8));
        serial.extend([0x00, 0x00, 0x01, 0x00]);
        serial.extend([0u8; 8]); // empty SourceArt
        serial.extend(2i32.to_le_bytes());
        serial.extend(tfc_mip(8, 5000));
        serial.extend(tfc_mip(4, 9000));
        let mut stream = serial.clone();
        stream.extend(vec![0u8; 4096]); // slack so the patched block recompresses in budget

        let summary_len = 96usize;
        let export_len = 72usize;
        let region_raw = names.len() + import.len() + export_len;
        let region_len = region_raw.next_multiple_of(16);
        let total_header = summary_len + region_len;

        let mut export = Vec::new();
        export.extend((-1i32).to_le_bytes()); // class = import 0 (Texture2D)
        export.extend(0i32.to_le_bytes()); // super
        export.extend(0i32.to_le_bytes()); // outer
        export.extend(idx(&all, "Skin_Octane_GaleFire_RGB").to_le_bytes());
        export.extend(0i32.to_le_bytes()); // name number
        export.extend(0i32.to_le_bytes()); // archetype
        export.extend(0u64.to_le_bytes()); // flags
        export.extend((serial.len() as i32).to_le_bytes());
        export.extend((total_header as u64).to_le_bytes());
        export.extend(0u32.to_le_bytes()); // export flags
        export.extend(0i32.to_le_bytes()); // net objects
        export.extend([0u8; 20]); // guid + package flags
        assert_eq!(export.len(), export_len);

        let mut region = names.clone();
        region.extend(&import);
        region.extend(&export);
        region.resize(region_len, 0);

        let mut file = Vec::new();
        file.extend(PACKAGE_MAGIC.to_le_bytes());
        file.extend(868u16.to_le_bytes());
        file.extend(32u16.to_le_bytes());
        file.extend((total_header as u32).to_le_bytes());
        file.extend(5i32.to_le_bytes());
        file.extend(b"None\0");
        file.extend(0u32.to_le_bytes());
        for v in [
            all.len(),
            summary_len,
            1,
            summary_len + names.len() + import.len(),
            1,
            summary_len + names.len(),
            summary_len + region_raw,
        ] {
            file.extend((v as u32).to_le_bytes());
        }
        file.resize(summary_len, 0);
        file.extend(ecb_encrypt(&region, &KEY));
        assert_eq!(file.len(), total_header);
        file.extend(wrap_chunked(&stream).unwrap());
        file
    }

    #[test]
    fn custom_cache_name_keeps_the_length() {
        assert_eq!(
            custom_cache_name("AlxsDecal", "Textures3".len()),
            "AlxsDecal"
        );
        assert_eq!(custom_cache_name("AlxsDecal", "Textures".len()), "AlxsDeca");
        assert_eq!(
            custom_cache_name("AlxsDecal", "Textures10".len()),
            "AlxsDecal0"
        );
        assert_eq!(
            custom_cache_name("AlxsBall", "Textures2".len()),
            "AlxsBall0"
        );
    }

    #[test]
    fn mask_swap_end_to_end() {
        let ring = KeyRing::from_text(&base64::prelude::BASE64_STANDARD.encode(KEY), None);
        let donor = fake_donor();
        let size = donor.len();
        let d = crate::decals::targets::donor_for(23).unwrap();
        let t = crate::decals::targets::find_target(d, None).unwrap();
        let (required, optional) =
            crate::decals::targets::renames(d, d.paintable.as_ref().unwrap(), t);
        let mask = RgbaImage::from_pixel(32, 32, image::Rgba([255, 0, 0, 0]));

        let swap = build(donor, &ring, None, &required, &optional, &mask).unwrap();
        assert_eq!(swap.package.len(), size, "package size must not change");
        assert_eq!(swap.mips_replaced, 2);

        let out = Package::open(swap.package.clone(), &ring).unwrap();
        for expected in [
            "Skin_Octane_Stars_SF",
            "Skin_Octane_Stars",
            "Skin_Octane_Stars_RGB",
            MASK_TFC_CUSTOM,
        ] {
            assert!(out.names.find(expected).is_some(), "{expected} missing");
        }
        assert!(out.names.find(MASK_TFC_SOURCE).is_none());

        let body = out.body().unwrap();
        let tex = out.textures(&body);
        assert_eq!(tex.len(), 1);
        let mips = &tex[0].mips;
        assert_eq!(mips[0].offset_in_file, 0);
        assert_eq!(mips[1].offset_in_file, u64::from(mips[0].size_on_disk));
        let first = &swap.tfc[..mips[0].size_on_disk as usize];
        let raw = unwrap_chunked(first).unwrap();
        assert_eq!(raw.len() as u32, mips[0].elem_count);
        let decoded = texture::decode(PixelFormat::Dxt5, 8, 8, &raw).unwrap();
        assert_eq!(decoded.get_pixel(3, 3).0[0], 255);
    }
}
