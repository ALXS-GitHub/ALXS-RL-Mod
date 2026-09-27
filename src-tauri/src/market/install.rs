//! Turns a downloaded pack into library packs.
//!
//! Sources lay packs out in many ways (flat zip, nested folders, one JSON
//! per variant, `Template.json` or `<Name>.json`…). Every AlphaConsole
//! manifest found is normalised to the libraries' layout —
//! `<root>/<Pack>/<Variant>/Template.json` + the files it references — then
//! decal variants are optionally converted to real-colour (hybrid) packs.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde_json::{Map, Value};

use crate::base::error::{AppError, AppResult};
use crate::base::paths;
use crate::decals::convert::{self, PackConversion};
use crate::market::model::{InstallReport, MarketItem, MarketKind};

const MAX_ENTRIES: usize = 2_000;
const MAX_UNPACKED_BYTES: u64 = 512 * 1024 * 1024;

fn library_root(kind: MarketKind) -> AppResult<PathBuf> {
    match kind {
        MarketKind::Decal => crate::decals::library::app_root(),
        MarketKind::Ball => crate::ball::library::app_root(),
    }
    .ok_or_else(|| AppError::Internal("no app data folder".into()))
}

/// Folder name for a pack title: printable, no path characters, bounded.
pub fn folder_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_()'&.,!+".contains(c) {
                c
            } else {
                ' '
            }
        })
        .collect();
    let name = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(['.', ' '])
        .chars()
        .take(60)
        .collect::<String>();
    if name.is_empty() {
        "Pack".into()
    } else {
        name
    }
}

/// Flags items whose pack folder already exists in the library.
pub fn mark_installed(items: &mut [MarketItem]) {
    for item in items {
        item.installed = library_root(item.kind)
            .map(|root| root.join(folder_name(&item.title)).is_dir())
            .unwrap_or(false);
    }
}

/// A fresh, empty staging folder for one download.
pub fn new_staging() -> AppResult<PathBuf> {
    let dir = paths::data_dir()?
        .join("market_staging")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Extracts a zip into `dest`, refusing paths that escape it.
pub fn extract_zip(bytes: &[u8], dest: &Path) -> AppResult<()> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    if archive.len() > MAX_ENTRIES {
        return Err(AppError::InvalidInput("archive has too many files".into()));
    }
    let mut unpacked = 0u64;
    for i in 0..archive.len() {
        let mut item = archive.by_index(i)?;
        if !item.is_file() {
            continue;
        }
        let Some(rel) = item.enclosed_name() else {
            continue;
        };
        unpacked += item.size();
        if unpacked > MAX_UNPACKED_BYTES {
            return Err(AppError::InvalidInput(
                "archive too large once unpacked".into(),
            ));
        }
        let out = dest.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut data = Vec::with_capacity(item.size() as usize);
        item.read_to_end(&mut data)?;
        std::fs::write(out, data)?;
    }
    Ok(())
}

/// A file referenced by a manifest, relative to the manifest's folder.
fn referenced_file(dir: &Path, value: &str) -> Option<PathBuf> {
    let rel = Path::new(value.trim());
    if value.trim().is_empty() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    let path = dir.join(rel);
    path.is_file().then_some(path)
}

/// Copies every file `entry` references into `dest` (flat) and rewrites
/// the references to the copied names.
fn copy_references(
    entry: &mut Value,
    src: &Path,
    dest: &Path,
    copied: &mut HashMap<PathBuf, String>,
) -> AppResult<()> {
    match entry {
        Value::String(s) => {
            if let Some(file) = referenced_file(src, s) {
                let name = match copied.get(&file) {
                    Some(n) => n.clone(),
                    None => {
                        let base = file
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let mut name = base.clone();
                        let mut k = 2;
                        while copied.values().any(|v| v.eq_ignore_ascii_case(&name)) {
                            name = format!("{k}_{base}");
                            k += 1;
                        }
                        std::fs::copy(&file, dest.join(&name))?;
                        copied.insert(file, name.clone());
                        name
                    }
                };
                *s = name;
            }
        }
        Value::Array(items) => {
            for v in items {
                copy_references(v, src, dest, copied)?;
            }
        }
        Value::Object(map) => {
            for v in map.values_mut() {
                copy_references(v, src, dest, copied)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn get_ci<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    map.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v)
}

/// Folder name of a manifest entry, or `None` if it is not a `kind` pack.
/// `folder` = the manifest's own folder name, used for bodies the app does
/// not name (sources usually name that folder after the car).
fn variant_name(
    kind: MarketKind,
    entry_name: &str,
    entry: &Map<String, Value>,
    folder: Option<&str>,
) -> Option<String> {
    match kind {
        MarketKind::Decal => {
            if get_ci(entry, "Body").is_none() && get_ci(entry, "Chassis").is_none() {
                return None;
            }
            let body = get_ci(entry, "BodyID").and_then(Value::as_i64);
            Some(match body {
                Some(id) => i32::try_from(id)
                    .ok()
                    .and_then(crate::decals::targets::body_name)
                    .map(str::to_string)
                    .or_else(|| folder.map(folder_name))
                    .unwrap_or_else(|| format!("Body {id}")),
                None => folder_name(entry_name),
            })
        }
        MarketKind::Ball => get_ci(entry, "Params").map(|_| folder_name(entry_name)),
    }
}

fn manifests(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .max_depth(8)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("json"))
        })
        .collect();
    out.sort();
    out
}

/// Normalises the packs found under `src` into `<root>/<folder_name(title)>`.
pub fn install_dir(
    src: &Path,
    root: &Path,
    kind: MarketKind,
    title: &str,
    convert_packs: bool,
) -> AppResult<InstallReport> {
    let pack = folder_name(title);
    let dest = root.join(&pack);
    if dest.exists() {
        return Err(AppError::Conflict(format!(
            "{pack} is already in your library"
        )));
    }
    // Build next to the destination, then rename: never a half pack.
    let staging = root.join(format!(".market-{pack}"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)?;

    let mut report = InstallReport {
        pack: pack.clone(),
        ..Default::default()
    };
    let mut variant_dirs = Vec::new();
    for manifest in manifests(src) {
        let raw = std::fs::read_to_string(&manifest)?;
        let Ok(Value::Object(doc)) =
            serde_json::from_str::<Value>(raw.trim_start_matches('\u{feff}'))
        else {
            continue;
        };
        let dir = manifest.parent().unwrap_or(src);
        let folder = (dir != src)
            .then(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()))
            .flatten();
        for (entry_name, entry) in doc {
            let Value::Object(fields) = &entry else {
                continue;
            };
            let Some(base) = variant_name(kind, &entry_name, fields, folder.as_deref()) else {
                continue;
            };
            let mut name = base.clone();
            let mut k = 2;
            while staging.join(&name).exists() {
                name = format!("{base} {k}");
                k += 1;
            }
            let out = staging.join(&name);
            std::fs::create_dir_all(&out)?;
            let mut entry = entry;
            copy_references(&mut entry, dir, &out, &mut HashMap::new())?;
            let mut single = Map::new();
            single.insert(entry_name, entry);
            let json = serde_json::to_string_pretty(&Value::Object(single))
                .map_err(|e| AppError::Internal(e.to_string()))?;
            std::fs::write(out.join("Template.json"), json)?;
            variant_dirs.push(name);
            report.variants += 1;
        }
    }
    if report.variants == 0 {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(AppError::InvalidInput(
            "no AlphaConsole pack found in the download".into(),
        ));
    }
    std::fs::rename(&staging, &dest)?;

    if kind == MarketKind::Decal && convert_packs {
        for name in variant_dirs {
            match convert::convert_pack_dir(&dest.join(&name)) {
                Ok(PackConversion::Converted) => report.converted += 1,
                Ok(_) => {}
                Err(e) => report.failed.push(format!("{name}: {e}")),
            }
        }
    }
    tracing::info!(pack = %report.pack, variants = report.variants, converted = report.converted, "market pack installed");
    Ok(report)
}

/// [`install_dir`] into the library of `kind`.
pub fn install(
    src: &Path,
    kind: MarketKind,
    title: &str,
    convert: bool,
) -> AppResult<InstallReport> {
    install_dir(src, &library_root(kind)?, kind, title, convert)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn png(path: &Path, px: [u8; 4]) {
        RgbaImage::from_pixel(4, 4, Rgba(px)).save(path).unwrap();
    }

    #[test]
    fn sanitises_folder_names() {
        assert_eq!(
            folder_name("Souly: Blossom/Decal  v2"),
            "Souly Blossom Decal v2"
        );
        assert_eq!(folder_name("..\\.."), "Pack");
        assert_eq!(folder_name("Chaewon (LeSserafim)"), "Chaewon (LeSserafim)");
    }

    #[test]
    fn normalises_a_flat_alphaconsole_decal_and_converts_it() {
        let src = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let tex = src.path().join("textures");
        std::fs::create_dir_all(&tex).unwrap();
        png(&tex.join("d.png"), [9, 9, 9, 255]);
        png(&tex.join("s.png"), [255, 0, 0, 0]);
        std::fs::write(
            src.path().join("MyDecal.json"),
            r#"{"MyDecal":{"BodyID":23,"Body":{"Diffuse":"textures/d.png","Skin":"textures/s.png"}}}"#,
        )
        .unwrap();
        std::fs::write(src.path().join("notes.json"), r#"["not a pack"]"#).unwrap();

        let r = install_dir(src.path(), root.path(), MarketKind::Decal, "My Decal", true).unwrap();
        assert_eq!(
            (r.pack.as_str(), r.variants, r.converted),
            ("My Decal", 1, 1)
        );
        let variant = root.path().join("My Decal/Octane");
        let template = std::fs::read_to_string(variant.join("Template.json")).unwrap();
        assert!(template.contains("\"Diffuse\": \"d.png\""), "{template}");
        assert!(variant.join("d.png").is_file() && variant.join(convert::ZONES_FILE).is_file());

        // Installing it again is refused, the library is untouched.
        assert!(matches!(
            install_dir(src.path(), root.path(), MarketKind::Decal, "My Decal", true),
            Err(AppError::Conflict(_))
        ));
    }

    #[test]
    fn installs_ball_variants_and_rejects_foreign_archives() {
        let src = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        png(&src.path().join("ball.png"), [1, 2, 3, 255]);
        std::fs::write(
            src.path().join("Ball.json"),
            r#"{"Blossom":{"Group":"","Params":{"Diffuse":"ball.png"}},"Blossom Night":{"Params":{"Diffuse":"ball.png"}}}"#,
        )
        .unwrap();
        let r = install_dir(src.path(), root.path(), MarketKind::Ball, "Blossoms", false).unwrap();
        assert_eq!(r.variants, 2);
        assert!(root
            .path()
            .join("Blossoms/Blossom Night/ball.png")
            .is_file());

        let empty = tempfile::tempdir().unwrap();
        std::fs::write(empty.path().join("x.json"), r#"{"a":{"Params":{}}}"#).unwrap();
        assert!(install_dir(empty.path(), root.path(), MarketKind::Decal, "Nope", true).is_err());
        assert!(!root.path().join("Nope").exists());
    }

    #[test]
    fn extract_refuses_path_traversal() {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("../evil.txt", opts).unwrap();
            std::io::Write::write_all(&mut w, b"x").unwrap();
            w.start_file("ok/a.txt", opts).unwrap();
            std::io::Write::write_all(&mut w, b"y").unwrap();
            w.finish().unwrap();
        }
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        extract_zip(buf.get_ref(), &dest).unwrap();
        assert!(dest.join("ok/a.txt").is_file());
        assert!(!dir.path().join("evil.txt").exists());
    }
}
