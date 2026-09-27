//! Item thumbnails extracted from the user's own install.
//!
//! RL ships a small companion package per item (`<Asset>_T_SF.upk`) holding
//! the icon texture. We decode its best readable mip (inline mips first —
//! no `.tfc` read needed), downscale to ≤ 256 px and cache a PNG under
//! `%LOCALAPPDATA%\ALXS-RL-Mod\catalog\thumbs\` (inside the asset-protocol
//! scope, so the UI can display it directly).
//! Misses are cached too (`.none` marker holding the build id and the key
//! count) so a missing thumbnail costs one attempt per game build — and is
//! retried when new keys are added.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use image::RgbaImage;

use crate::base::{fsx, paths, AppResult};
use crate::upk::chunks;
use crate::upk::error::{UpkError, UpkResult};
use crate::upk::keys::KeyRing;
use crate::upk::package::Package;
use crate::upk::texture::{self, Mip, Texture2D};

const THUMB_MAX: u32 = 256;

/// Case-insensitive file index of `CookedPCConsole` (built once per folder).
static INDEX: Mutex<Option<(PathBuf, HashMap<String, String>)>> = Mutex::new(None);

fn resolve_file(cooked: &Path, file_name: &str) -> Option<PathBuf> {
    let mut guard = INDEX.lock().ok()?;
    let fresh = !matches!(&*guard, Some((dir, _)) if dir == cooked);
    if fresh {
        let mut map = HashMap::new();
        for entry in std::fs::read_dir(cooked).ok()?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            map.insert(name.to_ascii_lowercase(), name);
        }
        *guard = Some((cooked.to_path_buf(), map));
    }
    let (_, map) = guard.as_ref()?;
    map.get(&file_name.to_ascii_lowercase())
        .map(|real| cooked.join(real))
}

/// Candidate thumbnail packages for a catalog package name
/// (`Wheel_Vortex_SF`, `Wheel_Vortex_SF.upk` or `Wheel_Vortex` all work).
pub fn candidates(package: &str) -> Vec<String> {
    let stem = package
        .trim()
        .trim_end_matches(".upk")
        .trim_end_matches(".UPK");
    let base = stem
        .strip_suffix("_SF")
        .or_else(|| stem.strip_suffix("_sf"))
        .unwrap_or(stem);
    vec![
        format!("{base}_T_SF.upk"),
        format!("{base}_T.upk"),
        format!("{stem}.upk"),
    ]
}

fn cache_dir() -> AppResult<PathBuf> {
    paths::data_subdir("catalog/thumbs")
}

fn cache_key(package: &str) -> String {
    fsx::sanitize_file_name(&package.trim().trim_end_matches(".upk").to_ascii_lowercase())
}

/// Reads one mip's raw pixels (inline or from the `.tfc`).
pub fn read_mip(
    cooked: &Path,
    body: &[u8],
    tfc_name: Option<&str>,
    mip: &Mip,
) -> UpkResult<Vec<u8>> {
    let raw = if let Some(pos) = mip.data_pos {
        body.get(pos..pos + mip.size_on_disk as usize)
            .ok_or(UpkError::Truncated("inline mip"))?
            .to_vec()
    } else {
        let tfc = tfc_name
            .ok_or_else(|| UpkError::Texture("mip stored in an unnamed texture cache".into()))?;
        let path = resolve_file(cooked, &format!("{tfc}.tfc"))
            .ok_or_else(|| UpkError::Texture(format!("{tfc}.tfc not found")))?;
        let mut f = std::fs::File::open(path)?;
        f.seek(SeekFrom::Start(mip.offset_in_file))?;
        let mut buf = vec![0u8; mip.size_on_disk as usize];
        f.read_exact(&mut buf)?;
        buf
    };
    if mip.compressed() {
        chunks::unwrap_chunked(&raw)
    } else {
        Ok(raw)
    }
}

fn best_image(cooked: &Path, pkg: &Package) -> UpkResult<Option<RgbaImage>> {
    let body = pkg.body()?;
    let mut textures: Vec<Texture2D> = pkg.textures(&body);
    // Prefer textures that look like icons.
    textures.sort_by_key(|t| {
        let n = t.name.to_ascii_lowercase();
        (
            !(n.contains("thumb") || n.ends_with("_t")),
            std::cmp::Reverse(t.width),
        )
    });
    for tex in &textures {
        let mut mips: Vec<&Mip> = tex.mips.iter().filter(|m| !m.is_empty()).collect();
        // Inline mips first (cheap), then the smallest TFC mip ≥ THUMB_MAX.
        mips.sort_by_key(|m| (m.data_pos.is_none(), m.width.abs_diff(THUMB_MAX)));
        for mip in mips {
            let Ok(data) = read_mip(cooked, &body.data, tex.tfc_name.as_deref(), mip) else {
                continue;
            };
            if let Ok(img) = texture::decode(tex.format, mip.width, mip.height, &data) {
                if img.width() >= 16 {
                    return Ok(Some(img));
                }
            }
        }
    }
    Ok(None)
}

fn downscale(img: RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let longest = w.max(h);
    if longest <= THUMB_MAX {
        return img;
    }
    let scale = THUMB_MAX as f32 / longest as f32;
    let (nw, nh) = (
        ((w as f32) * scale).round().max(1.0) as u32,
        ((h as f32) * scale).round().max(1.0) as u32,
    );
    image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
}

/// Returns the cached PNG path, extracting it if needed. `None` when the
/// item has no readable thumbnail.
pub fn thumbnail(
    cooked: &Path,
    ring: &KeyRing,
    package: &str,
    build: &str,
) -> AppResult<Option<PathBuf>> {
    let dir = cache_dir()?;
    let key = cache_key(package);
    let png = dir.join(format!("{key}.png"));
    if png.is_file() {
        return Ok(Some(png));
    }
    let miss = dir.join(format!("{key}.none"));
    // A miss is only valid for the same build *and* the same key ring.
    let miss_tag = format!("{build}|{}", ring.len());
    if std::fs::read_to_string(&miss).is_ok_and(|b| b == miss_tag) {
        return Ok(None);
    }
    for candidate in candidates(package) {
        let Some(path) = resolve_file(cooked, &candidate) else {
            continue;
        };
        let bytes = std::fs::read(&path)?;
        let pkg = match Package::open(bytes, ring) {
            Ok(p) => p,
            Err(UpkError::KeysMissing) => return Err(UpkError::KeysMissing.into()),
            Err(err) => {
                tracing::debug!(%candidate, %err, "thumbnail package unreadable");
                continue;
            }
        };
        if let Some(img) = best_image(cooked, &pkg)? {
            let mut out = Vec::new();
            downscale(img)
                .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
                .map_err(crate::base::AppError::from)?;
            fsx::write_atomic(&png, &out)?;
            return Ok(Some(png));
        }
    }
    fsx::write_atomic(&miss, miss_tag.as_bytes())?;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_names() {
        assert_eq!(
            candidates("Wheel_Vortex_SF.upk"),
            vec![
                "Wheel_Vortex_T_SF.upk",
                "Wheel_Vortex_T.upk",
                "Wheel_Vortex_SF.upk"
            ]
        );
        assert_eq!(candidates("Antenna_8Ball")[0], "Antenna_8Ball_T_SF.upk");
    }

    #[test]
    fn downscale_keeps_aspect() {
        let img = RgbaImage::new(1024, 512);
        let out = downscale(img);
        assert_eq!(out.dimensions(), (256, 128));
    }
}
