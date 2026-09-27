//! Builds `RL-Designer/decals/alxs-rl-mod/`: RL-Designer's decal and ball
//! packs in the app's format (manifest renamed `Template.json`, decals
//! converted to real-colour hybrid packs) plus the `index.json` read by the
//! marketplace (`market::rl_designer`).
//!
//! `cargo run --example rl_designer_export -- <RL-Designer>/decals`

use std::path::{Path, PathBuf};

use alxs_rl_mod_lib::decals::convert::{self, PackConversion};
use alxs_rl_mod_lib::decals::targets;
use alxs_rl_mod_lib::market::model::MarketKind;
use alxs_rl_mod_lib::market::rl_designer::{kind_dir, Index, IndexPack, IndexVariant};
use serde_json::Value;

/// Creator templates (not wearable) and packs kept out of the marketplace.
const SKIPPED_PACKS: &[&str] = &["Templates", "Jordan"];
const PREVIEW_FILE: &str = "preview.png";
const PREVIEW_SIZE: u32 = 256;

fn read_template(dir: &Path) -> Value {
    let raw = std::fs::read_to_string(dir.join("Template.json")).expect("template");
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).expect("template json")
}

fn entry_mut(doc: &mut Value) -> Option<&mut serde_json::Map<String, Value>> {
    doc.as_object_mut()?.values_mut().next()?.as_object_mut()
}

fn role<'a>(map: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    map.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .and_then(|(_, v)| v.as_str())
}

/// Converted packs of bodies with a hybrid donor never use the AlphaConsole
/// `Diffuse` + `Skin` pair again: drop them to halve the download. Other
/// bodies keep them (a paintable donor would need the `Skin` mask).
fn drop_originals(dir: &Path) {
    let mut doc = read_template(dir);
    let Some(entry) = entry_mut(&mut doc) else {
        return;
    };
    let body_id = entry.get("BodyID").and_then(Value::as_i64).unwrap_or(0) as i32;
    if !targets::donor_for(body_id).is_some_and(|d| d.hybrid.is_some()) {
        return;
    }
    let Some(body) = entry.get_mut("Body").and_then(Value::as_object_mut) else {
        return;
    };
    let mut removed = Vec::new();
    body.retain(|k, v| {
        let original = k.eq_ignore_ascii_case("Diffuse") || k.eq_ignore_ascii_case("Skin");
        if original {
            if let Some(f) = v.as_str() {
                removed.push(f.to_string());
            }
        }
        !original
    });
    let still_used = serde_json::to_string(&doc).expect("json");
    for f in removed {
        if !still_used.contains(&format!("\"{f}\"")) {
            let _ = std::fs::remove_file(dir.join(&f));
        }
    }
    std::fs::write(
        dir.join("Template.json"),
        serde_json::to_string_pretty(&doc).expect("json"),
    )
    .expect("write template");
}

/// Small opaque thumbnail for the marketplace grid.
fn write_preview(dir: &Path, kind: MarketKind) -> bool {
    let mut doc = read_template(dir);
    let Some(entry) = entry_mut(&mut doc) else {
        return false;
    };
    let source = match kind {
        MarketKind::Decal => entry
            .get("Body")
            .and_then(Value::as_object)
            .and_then(|b| role(b, "1_Diffuse_Skin").or_else(|| role(b, "Diffuse"))),
        MarketKind::Ball => entry
            .get("Params")
            .and_then(Value::as_object)
            .and_then(|p| role(p, "Diffuse")),
    };
    let Some(img) = source.and_then(|f| image::open(dir.join(f)).ok()) else {
        return false;
    };
    let mut thumb = img
        .resize(
            PREVIEW_SIZE,
            PREVIEW_SIZE,
            image::imageops::FilterType::Triangle,
        )
        .to_rgba8();
    for p in thumb.pixels_mut() {
        p.0[3] = 255;
    }
    thumb.save(dir.join(PREVIEW_FILE)).is_ok()
}

/// `(pack name, relative_path, [(variant name, files)])` of an RL-Designer index.
type PackEntry = (String, Option<String>, Vec<(String, Vec<String>)>);

fn pack_entries(index: &Value) -> Vec<PackEntry> {
    let packs = index
        .get("decals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    packs
        .iter()
        .filter_map(|p| {
            let name = p.get("name")?.as_str()?.to_string();
            let rel = p
                .get("relative_path")
                .and_then(Value::as_str)
                .map(str::to_string);
            let variants = p
                .get("variants")?
                .as_array()?
                .iter()
                .filter_map(|v| {
                    let files = v
                        .get("files")?
                        .as_array()?
                        .iter()
                        .filter_map(|f| f.as_str().map(str::to_string))
                        .collect();
                    Some((v.get("variant_name")?.as_str()?.to_string(), files))
                })
                .collect();
            Some((name, rel, variants))
        })
        .collect()
}

fn export_kind(
    decals_root: &Path,
    out_root: &Path,
    kind: MarketKind,
    git_folder: &str,
    index_file: &str,
) -> Vec<IndexPack> {
    let raw = std::fs::read_to_string(decals_root.join(index_file)).expect("index");
    let index: Value = serde_json::from_str(&raw).expect("index json");
    let mut packs = Vec::new();
    for (name, rel, variants) in pack_entries(&index) {
        if SKIPPED_PACKS.contains(&name.as_str()) {
            continue;
        }
        let mut src_pack = decals_root.join(git_folder);
        if let Some(rel) = &rel {
            src_pack = src_pack.join(rel);
        }
        src_pack = src_pack.join(&name);
        let mut out_variants = Vec::new();
        for (variant, files) in variants {
            let src = src_pack.join(&variant);
            let dest = out_root.join(kind_dir(kind)).join(&name).join(&variant);
            std::fs::create_dir_all(&dest).expect("mkdir");
            for f in &files {
                let from = src.join(f);
                let to_name = if f.to_ascii_lowercase().ends_with(".json")
                    && f.to_ascii_lowercase().contains("template")
                {
                    "Template.json".to_string()
                } else {
                    f.clone()
                };
                if from.is_file() {
                    std::fs::copy(&from, dest.join(&to_name)).expect("copy");
                } else {
                    eprintln!("  missing {}", from.display());
                }
            }
            if kind == MarketKind::Decal {
                match convert::convert_pack_dir(&dest) {
                    Ok(PackConversion::Converted) => {
                        drop_originals(&dest);
                        println!("  converted {name}/{variant}");
                    }
                    Ok(other) => println!("  kept {name}/{variant} ({other:?})"),
                    Err(e) => eprintln!("  FAILED {name}/{variant}: {e}"),
                }
            }
            let has_preview = write_preview(&dest, kind);
            let mut listed: Vec<String> = std::fs::read_dir(&dest)
                .expect("list")
                .flatten()
                .filter(|e| e.path().is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            listed.sort();
            let preview = has_preview.then(|| PREVIEW_FILE.to_string());
            out_variants.push(IndexVariant {
                name: variant,
                files: listed,
                preview,
            });
        }
        packs.push(IndexPack {
            kind,
            group: rel,
            name,
            variants: out_variants,
        });
    }
    packs
}

fn main() {
    let decals_root = PathBuf::from(std::env::args().nth(1).expect("<RL-Designer>/decals"));
    let out_root = decals_root.join("alxs-rl-mod");
    let _ = std::fs::remove_dir_all(&out_root);
    let mut packs = export_kind(
        &decals_root,
        &out_root,
        MarketKind::Decal,
        "decals",
        "index.json",
    );
    packs.extend(export_kind(
        &decals_root,
        &out_root,
        MarketKind::Ball,
        "ball_textures",
        "ball_index.json",
    ));
    let index = Index { version: 1, packs };
    std::fs::write(
        out_root.join("index.json"),
        serde_json::to_string_pretty(&index).expect("json") + "\n",
    )
    .expect("write index");
    std::fs::write(
        out_root.join("README.md"),
        "# ALXS-RL-Mod format\n\nRL-Designer's decal and ball packs, ready for \
         [ALXS-RL-Mod](https://github.com/ALXS-GitHub/ALXS-RL-Mod)'s marketplace: \
         each variant has a `Template.json`, and car decals are converted to real-colour \
         (hybrid) packs (`hybrid_art.png` + `hybrid_zones.png`).\n\nGenerated by \
         `cargo run --example rl_designer_export -- <RL-Designer>/decals` in ALXS-RL-Mod; \
         do not edit by hand.\n",
    )
    .expect("write readme");
    println!(
        "{} packs → {}",
        index.packs.len(),
        out_root.join("index.json").display()
    );
}
