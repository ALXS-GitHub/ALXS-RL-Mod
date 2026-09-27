//! Ball texture packs.
//!
//! Layout (AlphaConsole's `BallTextures`): `<root>/<Pack>/<Ball>/Template.json`
//! and a PNG; the template is one object whose single key is the display
//! name: `{ "Itzy (Default)": { "Group": "Itzy", "Params": { "Diffuse": "diffuse.png" } } }`.
//! `<Ball>` names the ball the pack was made for; only `Default` (the
//! standard ball) is supported.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::base::{fsx, paths};

#[derive(Debug, Clone, Deserialize)]
struct TemplateEntry {
    #[serde(rename = "Group", default)]
    group: String,
    #[serde(rename = "Params", default)]
    params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BallPack {
    /// Hash of `<pack>/<ball>` (stable across library folders).
    pub id: String,
    pub pack_name: String,
    /// Ball the pack was made for (`Default`, …).
    pub ball: String,
    pub display_name: String,
    pub group: String,
    pub image_path: Option<String>,
    pub preview_path: Option<String>,
    pub template_path: String,
    /// Made for the standard ball, with an existing image.
    pub supported: bool,
}

/// The app's ball pack library.
pub fn app_root() -> Option<PathBuf> {
    paths::data_subdir("ball/library").ok()
}

/// `%APPDATA%\bakkesmod\bakkesmod\data\acplugin\BallTextures`.
pub fn alphaconsole_root() -> Option<PathBuf> {
    paths::roaming_dir().map(|r| {
        r.join("bakkesmod")
            .join("bakkesmod")
            .join("data")
            .join("acplugin")
            .join("BallTextures")
    })
}

fn pack_id(pack: &str, ball: &str) -> String {
    fsx::sha256_bytes(format!("ball:{pack}/{ball}").to_lowercase().as_bytes())[..16].to_string()
}

fn parse_template(raw: &str) -> Option<(String, TemplateEntry)> {
    let map: HashMap<String, TemplateEntry> =
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok()?;
    let mut entries: Vec<(String, TemplateEntry)> = map.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.into_iter().next()
}

fn image_of(dir: &Path, params: &HashMap<String, String>) -> Option<PathBuf> {
    params
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Diffuse"))
        .map(|(_, f)| dir.join(f))
        .filter(|p| p.is_file())
}

/// Every pack of `root`, supported ones first.
pub fn scan_root(root: &Path, with_previews: bool) -> Vec<BallPack> {
    let mut out = Vec::new();
    let Ok(packs) = std::fs::read_dir(root) else {
        return out;
    };
    for pack in packs.flatten().filter(|e| e.path().is_dir()) {
        let pack_name = pack.file_name().to_string_lossy().into_owned();
        let Ok(balls) = std::fs::read_dir(pack.path()) else {
            continue;
        };
        for ball in balls.flatten() {
            let dir = ball.path();
            let template = dir.join("Template.json");
            let Some((display_name, t)) = std::fs::read_to_string(&template)
                .ok()
                .and_then(|raw| parse_template(&raw))
            else {
                continue;
            };
            let ball_name = ball.file_name().to_string_lossy().into_owned();
            let id = pack_id(&pack_name, &ball_name);
            let image = image_of(&dir, &t.params);
            let preview_path = if with_previews {
                image
                    .as_deref()
                    .and_then(|p| crate::decals::library::preview(p, &format!("ball-{id}"), true))
            } else {
                None
            };
            out.push(BallPack {
                supported: image.is_some() && ball_name.eq_ignore_ascii_case("Default"),
                id,
                pack_name: pack_name.clone(),
                ball: ball_name,
                display_name,
                group: t.group,
                image_path: image.map(|p| p.display().to_string()),
                preview_path,
                template_path: template.display().to_string(),
            });
        }
    }
    out.sort_by(|a, b| {
        b.supported
            .cmp(&a.supported)
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    out
}

pub fn scan(with_previews: bool) -> Vec<BallPack> {
    app_root()
        .map(|r| scan_root(&r, with_previews))
        .unwrap_or_default()
}

pub fn find(id: &str) -> Option<BallPack> {
    scan(false).into_iter().find(|p| p.id == id)
}

/// Cached preview of a pack's image (generated if needed).
pub fn preview_of(pack: &BallPack) -> Option<String> {
    pack.preview_path.clone().or_else(|| {
        pack.image_path.as_deref().and_then(|p| {
            crate::decals::library::preview(Path::new(p), &format!("ball-{}", pack.id), true)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, content: &[u8]) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn scans_ball_packs_and_supports_the_default_ball_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            &root.join("Itzy/Default/Template.json"),
            br#"{"Itzy (Default)":{"Group":"Itzy","Params":{"Diffuse":"diffuse.png"}}}"#,
        );
        write(&root.join("Itzy/Default/diffuse.png"), b"x");
        write(
            &root.join("Itzy/Zomahx/Template.json"),
            br#"{"Itzy (Zomahx)":{"Group":"Itzy","Params":{"Diffuse":"diffuse.png"}}}"#,
        );
        write(&root.join("Itzy/Zomahx/diffuse.png"), b"x");
        write(
            &root.join("Broken/Default/Template.json"),
            br#"{"Broken":{"Params":{"Diffuse":"missing.png"}}}"#,
        );
        let packs = scan_root(root, false);
        assert_eq!(packs.len(), 3);
        assert_eq!(packs[0].display_name, "Itzy (Default)");
        assert!(packs[0].supported);
        assert!(!packs.iter().find(|p| p.ball == "Zomahx").unwrap().supported);
        assert!(
            !packs
                .iter()
                .find(|p| p.pack_name == "Broken")
                .unwrap()
                .supported
        );
        assert_ne!(packs[0].id, pack_id("Itzy", "Zomahx"));
    }
}
