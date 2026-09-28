//! Scans AlphaConsole-style decal libraries.
//!
//! Packs live in the app's own library (`%LOCALAPPDATA%\ALXS-RL-Mod\decals\
//! library`, filled by the AlphaConsole import) plus any folder the user
//! adds. Layout (the one AlphaConsole and RL-Designer already use):
//! `<root>/<Pack>/<Body>/Template.json` + PNG files. `Template.json` is one
//! object whose single key is the display name:
//! `{ "Name (Octane)": { "Group", "BodyID", "SkinID", "Body": {role: file}, "Chassis": {…} } }`.
//! The user's files are never moved; small previews are cached under
//! `%LOCALAPPDATA%\ALXS-RL-Mod\decals\previews\` so the UI can show them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::base::{fsx, paths};
use crate::decals::targets;

#[derive(Debug, Clone, Deserialize)]
struct TemplateEntry {
    #[serde(rename = "Group", default)]
    group: String,
    #[serde(rename = "BodyID", default)]
    body_id: i32,
    #[serde(rename = "SkinID", default)]
    skin_id: i32,
    #[serde(rename = "Body", default)]
    body: HashMap<String, String>,
    #[serde(rename = "Chassis", default)]
    chassis: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecalTexture {
    pub role: String,
    pub file_name: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecalPack {
    /// Stable id derived from the pack's path inside its library root, so a
    /// pack keeps its id when imported from AlphaConsole into the app.
    pub id: String,
    /// Ids earlier versions derived from the full Template.json path (still
    /// referenced by presets and the active decal).
    #[serde(skip)]
    pub legacy_ids: Vec<String>,
    pub library_root: String,
    pub pack_name: String,
    pub body_folder: String,
    pub display_name: String,
    pub group: String,
    /// RL body product id (`-1` = universal).
    pub body_id: i32,
    pub body_name: Option<String>,
    pub skin_id: i32,
    pub body: Vec<DecalTexture>,
    pub chassis: Vec<DecalTexture>,
    pub template_path: String,
    /// Cached PNG previews (inside the app data folder).
    pub preview_path: Option<String>,
    pub mask_preview_path: Option<String>,
    /// Colour-zone mask PNG (required by the paintable pipeline).
    pub mask_path: Option<String>,
    pub diffuse_path: Option<String>,
    /// Body supported by the pipeline and the pack ships a mask.
    pub supported: bool,
    /// Pack authored for the full-colour material: `1_Diffuse_Skin` (art)
    /// + `2_Diffuse_Skin_Mask` (R primary, G accent, black = art colours).
    pub hybrid: bool,
    /// Universal pack (`BodyID` -1): `1_Diffuse_Skin` art + `TrimSheet`.
    pub universal: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRoot {
    pub path: String,
    pub exists: bool,
    pub pack_count: usize,
    pub is_default: bool,
}

const MASK_ROLES: &[&str] = &["Skin", "Skin_M", "Mask", "ColorMask"];
const DIFFUSE_ROLES: &[&str] = &["Diffuse", "1_Diffuse_Skin"];
const HYBRID_ART_ROLE: &str = "1_Diffuse_Skin";
const HYBRID_ZONES_ROLE: &str = "2_Diffuse_Skin_Mask";
const PREVIEW_MAX: u32 = 320;

/// The app's own pack library.
pub fn app_root() -> Option<PathBuf> {
    paths::data_subdir("decals/library").ok()
}

/// `%APPDATA%\bakkesmod\bakkesmod\data\acplugin\DecalTextures`.
pub fn alphaconsole_root() -> Option<PathBuf> {
    paths::roaming_dir().map(|r| {
        r.join("bakkesmod")
            .join("bakkesmod")
            .join("data")
            .join("acplugin")
            .join("DecalTextures")
    })
}

/// The app's library (the default root) followed by the configured folders.
pub fn roots(configured: &[PathBuf]) -> Vec<(PathBuf, bool)> {
    let mut out: Vec<(PathBuf, bool)> = app_root().map(|p| vec![(p, true)]).unwrap_or_default();
    for p in configured {
        if !out.iter().any(|(r, _)| r == p) {
            out.push((p.clone(), false));
        }
    }
    out
}

fn hash_id(key: &str) -> String {
    fsx::sha256_bytes(key.to_lowercase().as_bytes())[..16].to_string()
}

/// `<pack>/<body>` of a pack folder, relative to its library root.
fn relative_key(pack_name: &str, body_folder: &str) -> String {
    format!("{pack_name}/{body_folder}")
}

fn pack_id(pack_name: &str, body_folder: &str) -> String {
    hash_id(&format!("rel:{}", relative_key(pack_name, body_folder)))
}

/// Ids of the old scheme (hash of the full Template.json path), here and at
/// the same place in the AlphaConsole folder the pack was imported from.
fn legacy_ids(template: &Path, pack_name: &str, body_folder: &str) -> Vec<String> {
    let mut ids = vec![hash_id(&template.to_string_lossy())];
    if let Some(ac) = alphaconsole_root() {
        let t = ac.join(pack_name).join(body_folder).join("Template.json");
        ids.push(hash_id(&t.to_string_lossy()));
    }
    ids
}

fn parse_template(raw: &str) -> Option<(String, TemplateEntry)> {
    let map: HashMap<String, TemplateEntry> = serde_json::from_str(raw).ok()?;
    let mut entries: Vec<(String, TemplateEntry)> = map.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.into_iter().next()
}

fn collect(map: &HashMap<String, String>, dir: &Path) -> Vec<DecalTexture> {
    let mut out: Vec<DecalTexture> = map
        .iter()
        .map(|(role, file)| {
            let path = dir.join(file);
            DecalTexture {
                role: role.clone(),
                file_name: file.clone(),
                exists: path.is_file(),
                path: path.display().to_string(),
            }
        })
        .collect();
    out.sort_by(|a, b| a.role.cmp(&b.role));
    out
}

fn pick<'a>(textures: &'a [DecalTexture], roles: &[&str]) -> Option<&'a DecalTexture> {
    roles.iter().find_map(|r| {
        textures
            .iter()
            .find(|t| t.exists && t.role.eq_ignore_ascii_case(r))
    })
}

fn modified(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Downscaled PNG copy of `source`, regenerated when the source is newer.
/// `opaque` drops the alpha channel (hybrid art stores its highlight mask
/// there, often fully transparent).
pub(crate) fn preview(source: &Path, name: &str, opaque: bool) -> Option<String> {
    let dir = paths::data_subdir("decals/previews").ok()?;
    let out = dir.join(format!("{name}.png"));
    if out.is_file() && modified(&out) >= modified(source) {
        return Some(out.display().to_string());
    }
    let mut img = image::open(source).ok()?.to_rgba8();
    if opaque {
        img.pixels_mut().for_each(|p| p.0[3] = 255);
    }
    let (w, h) = img.dimensions();
    let scale = PREVIEW_MAX as f32 / w.max(h).max(1) as f32;
    let small = if scale < 1.0 {
        image::imageops::resize(
            &img,
            ((w as f32 * scale).round() as u32).max(1),
            ((h as f32 * scale).round() as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let mut bytes = Vec::new();
    small
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .ok()?;
    fsx::write_atomic(&out, &bytes).ok()?;
    Some(out.display().to_string())
}

/// The AlphaConsole manifest of a variant folder: `Template.json`, or a
/// `<Car> Template.json` as some packs (RL-Designer's Fennec variants…)
/// name it.
pub fn manifest_in(dir: &Path) -> Option<PathBuf> {
    let exact = dir.join("Template.json");
    if exact.is_file() {
        return Some(exact);
    }
    let mut named: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .map(|n| n.to_string_lossy().to_ascii_lowercase())
                    .is_some_and(|n| n.ends_with("template.json"))
        })
        .collect();
    named.sort();
    named.into_iter().next()
}

fn scan_pack(root: &Path, pack_dir: &Path, with_previews: bool, out: &mut Vec<DecalPack>) {
    let Some(pack_name) = pack_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
    else {
        return;
    };
    let Ok(bodies) = std::fs::read_dir(pack_dir) else {
        return;
    };
    for entry in bodies.flatten() {
        let body_dir = entry.path();
        let Some(template) = manifest_in(&body_dir) else {
            continue;
        };
        let Some((display_name, t)) = std::fs::read_to_string(&template)
            .ok()
            .and_then(|raw| parse_template(&raw))
        else {
            tracing::warn!(template = %template.display(), "Template.json unreadable, pack skipped");
            continue;
        };
        let body = collect(&t.body, &body_dir);
        let chassis = collect(&t.chassis, &body_dir);
        let body_folder = entry.file_name().to_string_lossy().into_owned();
        let id = pack_id(&pack_name, &body_folder);
        let legacy = legacy_ids(&template, &pack_name, &body_folder);
        let hybrid_pair = pick(&body, &[HYBRID_ART_ROLE]).zip(pick(&body, &[HYBRID_ZONES_ROLE]));
        let hybrid = hybrid_pair.is_some()
            && targets::donor_for(t.body_id).is_some_and(|d| d.hybrid.is_some());
        let universal = t.body_id == -1 && pick(&body, &[HYBRID_ART_ROLE]).is_some();
        let (mask, diffuse) = match hybrid_pair {
            Some((art, zones)) => (Some(zones.path.clone()), Some(art.path.clone())),
            None => (
                pick(&body, MASK_ROLES).map(|x| x.path.clone()),
                pick(&body, DIFFUSE_ROLES)
                    .or_else(|| body.iter().find(|x| x.exists))
                    .map(|x| x.path.clone()),
            ),
        };
        let (preview_path, mask_preview_path) = if with_previews {
            (
                diffuse
                    .as_deref()
                    .and_then(|p| art_preview_of(Path::new(p), &id, hybrid || universal)),
                mask.as_deref()
                    .and_then(|p| preview(Path::new(p), &format!("{id}-mask"), false)),
            )
        } else {
            (None, None)
        };
        let supported = targets::donor_for(t.body_id).is_some_and(|d| {
            (universal && d.universal.is_some())
                || (mask.is_some() && (hybrid || d.paintable.is_some()))
        });
        out.push(DecalPack {
            id,
            legacy_ids: legacy,
            library_root: root.display().to_string(),
            pack_name: pack_name.clone(),
            body_folder: body_folder.clone(),
            display_name,
            group: t.group,
            body_id: t.body_id,
            body_name: targets::body_name(t.body_id).map(str::to_string),
            skin_id: t.skin_id,
            body,
            chassis,
            template_path: template.display().to_string(),
            preview_path,
            mask_preview_path,
            mask_path: mask,
            diffuse_path: diffuse,
            supported,
            hybrid,
            universal,
        });
    }
}

/// Every pack of every root, supported ones first.
pub fn scan(roots: &[(PathBuf, bool)], with_previews: bool) -> Vec<DecalPack> {
    let mut packs = Vec::new();
    for (root, _) in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                scan_pack(root, &entry.path(), with_previews, &mut packs);
            }
        }
    }
    // The same pack in two roots (imported, and still in an added folder):
    // the first root, the app's library, wins.
    let mut seen = std::collections::HashSet::new();
    packs.retain(|p| seen.insert(p.id.clone()));
    packs.sort_by(|a, b| {
        b.supported
            .cmp(&a.supported)
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    packs
}

pub fn root_infos(roots: &[(PathBuf, bool)]) -> Vec<LibraryRoot> {
    roots
        .iter()
        .map(|(p, is_default)| {
            let pack_count = std::fs::read_dir(p)
                .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).count())
                .unwrap_or(0);
            LibraryRoot {
                path: p.display().to_string(),
                exists: p.is_dir(),
                pack_count,
                is_default: *is_default,
            }
        })
        .collect()
}

fn art_preview_of(source: &Path, id: &str, hybrid: bool) -> Option<String> {
    let name = if hybrid {
        format!("{id}-art")
    } else {
        id.to_string()
    };
    preview(source, &name, hybrid)
}

/// A pack's texture for a `Body` role (case-insensitive), if the file exists.
pub fn role_path(pack: &DecalPack, role: &str) -> Option<String> {
    pick(&pack.body, &[role]).map(|t| t.path.clone())
}

/// Cached preview of a pack's art (generated if needed); packs returned by
/// [`find`] carry no preview paths.
pub fn art_preview(pack: &DecalPack) -> Option<String> {
    pack.preview_path.clone().or_else(|| {
        pack.diffuse_path
            .as_deref()
            .and_then(|p| art_preview_of(Path::new(p), &pack.id, pack.hybrid || pack.universal))
    })
}

pub fn find(roots: &[(PathBuf, bool)], id: &str) -> Option<DecalPack> {
    scan(roots, false)
        .into_iter()
        .find(|p| p.id == id || p.legacy_ids.iter().any(|l| l == id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, content: &[u8]) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn scans_packs_and_flags_support() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let octane = root.join("Itzy").join("Octane");
        write(
            &octane.join("Template.json"),
            br#"{"Born To Be Itzy (Octane)":{"Group":"Itzy","BodyID":23,"SkinID":0,
                "Body":{"Diffuse":"body.png","Skin":"skin.png"},"Chassis":{"Diffuse":"chassis.png"}}}"#,
        );
        write(&octane.join("body.png"), b"x");
        write(&octane.join("skin.png"), b"x");
        let universal = root.join("Crystals").join("Universal");
        write(
            &universal.join("Template.json"),
            br#"{"Frozen Crystals":{"BodyID":-1,"SkinID":11609,"Body":{"1_Diffuse_Skin":"d.png"}}}"#,
        );
        write(&universal.join("d.png"), b"x");
        std::fs::create_dir_all(root.join("Empty").join("wip")).unwrap();

        let roots = vec![(root.clone(), false)];
        let packs = scan(&roots, false);
        assert_eq!(packs.len(), 2);
        assert!(packs[0].supported);
        assert_eq!(packs[0].body_id, 23);
        assert!(packs[0].mask_path.as_deref().unwrap().ends_with("skin.png"));
        assert!(
            packs[1].supported && packs[1].universal,
            "universal art pack"
        );
        assert!(packs[1].mask_path.is_none() && packs[1].diffuse_path.is_some());
        assert_eq!(root_infos(&roots)[0].pack_count, 3);
        assert_eq!(
            find(&roots, &packs[1].id).unwrap().display_name,
            "Frozen Crystals"
        );
    }

    #[test]
    fn ids_are_stable_and_case_insensitive() {
        assert_eq!(pack_id("Itzy", "Octane"), pack_id("itzy", "OCTANE"));
        assert_ne!(pack_id("Itzy", "Octane"), pack_id("Itzy", "Fennec"));
    }

    #[test]
    fn same_pack_in_another_root_keeps_its_id_and_legacy_ids_resolve() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        for root in [a.path(), b.path()] {
            let dir = root.join("Itzy").join("Octane");
            write(
                &dir.join("Template.json"),
                br#"{"Itzy (Octane)":{"BodyID":23,"Body":{"Skin":"s.png"}}}"#,
            );
            write(&dir.join("s.png"), b"x");
        }
        let roots = vec![
            (a.path().to_path_buf(), true),
            (b.path().to_path_buf(), false),
        ];
        let packs = scan(&roots, false);
        assert_eq!(packs.len(), 1, "duplicates collapse to the first root");
        assert!(packs[0]
            .library_root
            .starts_with(&*a.path().to_string_lossy()));
        // An id of the old scheme (full Template.json path) still finds it.
        let old = hash_id(
            &a.path()
                .join("Itzy")
                .join("Octane")
                .join("Template.json")
                .to_string_lossy(),
        );
        assert_ne!(old, packs[0].id);
        assert_eq!(find(&roots, &old).unwrap().id, packs[0].id);
    }

    #[test]
    fn reads_car_named_manifests() {
        let dir = tempfile::tempdir().unwrap();
        let fennec = dir.path().join("Winter Aespa/Fennec");
        write(
            &fennec.join("Fennec Template.json"),
            br#"{"Winter Aespa (Fennec)":{"BodyID":4284,"Body":{"Diffuse":"d.png","Skin":"s.png"}}}"#,
        );
        assert_eq!(
            manifest_in(&fennec).unwrap().file_name().unwrap(),
            "Fennec Template.json"
        );
        write(&fennec.join("Template.json"), b"{}");
        assert_eq!(
            manifest_in(&fennec).unwrap().file_name().unwrap(),
            "Template.json"
        );
        assert!(manifest_in(dir.path()).is_none());
    }
}
