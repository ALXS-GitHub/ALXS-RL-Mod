//! Imports AlphaConsole decal packs into the app's library.
//!
//! Packs are copied (the AlphaConsole folder is left untouched, it doubles
//! as the backup) and, if asked, converted to hybrid packs so their art
//! shows in its own colours (see [`convert`](super::convert)). A pack whose
//! folder already exists in the app's library is skipped.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::base::{AppError, AppResult};
use crate::decals::convert::{self, PackConversion};
use crate::decals::library;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaConsoleSource {
    pub path: String,
    /// Pack folders found (each holds one `Template.json` per body).
    pub pack_count: usize,
    /// Pack folders not in the app's library yet.
    pub not_imported: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: usize,
    pub already: usize,
    /// Body packs converted to hybrid.
    pub converted: usize,
    /// `<pack>/<body>: reason` for body packs that could not be converted.
    pub failed: Vec<String>,
}

/// Top-level folders of `root` holding at least one `<body>/Template.json`.
pub(crate) fn pack_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            std::fs::read_dir(p)
                .map(|rd| {
                    rd.flatten()
                        .any(|b| library::manifest_in(&b.path()).is_some())
                })
                .unwrap_or(false)
        })
        .collect();
    dirs.sort();
    dirs
}

pub fn source() -> Option<AlphaConsoleSource> {
    source_in(&library::alphaconsole_root()?, &library::app_root()?)
}

/// Packs of `root` (an AlphaConsole folder) and how many are not in `app`.
pub(crate) fn source_in(root: &Path, app: &Path) -> Option<AlphaConsoleSource> {
    if !root.is_dir() {
        return None;
    }
    let packs = pack_dirs(root);
    let not_imported = packs
        .iter()
        .filter(|p| p.file_name().is_some_and(|n| !app.join(n).exists()))
        .count();
    Some(AlphaConsoleSource {
        path: root.display().to_string(),
        pack_count: packs.len(),
        not_imported,
    })
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// Copies every AlphaConsole pack not yet imported into the app's library,
/// converting them to hybrid packs when `convert_packs` is set.
pub fn import(convert_packs: bool) -> AppResult<ImportReport> {
    let root = library::alphaconsole_root()
        .filter(|r| r.is_dir())
        .ok_or_else(|| AppError::NotFound("AlphaConsole decal folder".into()))?;
    let app = library::app_root().ok_or_else(|| AppError::Internal("no app data folder".into()))?;
    import_from(&root, &app, convert_packs)
}

pub(crate) fn import_from(root: &Path, app: &Path, convert_packs: bool) -> AppResult<ImportReport> {
    let mut report = ImportReport::default();
    for pack in pack_dirs(root) {
        let Some(name) = pack.file_name() else {
            continue;
        };
        let dest = app.join(name);
        if dest.exists() {
            report.already += 1;
            continue;
        }
        // Copy next to the destination first: a failed copy never leaves a
        // half pack that would then be skipped as "already imported".
        let staging = app.join(format!(".importing-{}", name.to_string_lossy()));
        let _ = std::fs::remove_dir_all(&staging);
        copy_dir(&pack, &staging)?;
        std::fs::rename(&staging, &dest)?;
        report.imported += 1;
        if !convert_packs {
            continue;
        }
        for body in std::fs::read_dir(&dest)?.flatten() {
            let dir = body.path();
            if library::manifest_in(&dir).is_none() {
                continue;
            }
            match convert::convert_pack_dir(&dir) {
                Ok(PackConversion::Converted) => report.converted += 1,
                Ok(_) => {}
                Err(e) => report.failed.push(format!(
                    "{}/{}: {e}",
                    name.to_string_lossy(),
                    body.file_name().to_string_lossy()
                )),
            }
        }
    }
    tracing::info!(
        imported = report.imported,
        already = report.already,
        converted = report.converted,
        failed = report.failed.len(),
        "AlphaConsole packs imported"
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn ac_pack(root: &Path, name: &str) {
        let dir = root.join(name).join("Octane");
        std::fs::create_dir_all(&dir).unwrap();
        RgbaImage::from_pixel(4, 4, Rgba([9, 9, 9, 255]))
            .save(dir.join("d.png"))
            .unwrap();
        RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 0]))
            .save(dir.join("s.png"))
            .unwrap();
        std::fs::write(
            dir.join("Template.json"),
            r#"{"P (Octane)":{"BodyID":23,"Body":{"Diffuse":"d.png","Skin":"s.png"}}}"#,
        )
        .unwrap();
    }

    #[test]
    fn copies_converts_and_skips_what_is_already_there() {
        let ac = tempfile::tempdir().unwrap();
        let app = tempfile::tempdir().unwrap();
        ac_pack(ac.path(), "Itzy");
        ac_pack(ac.path(), "Aespa");
        std::fs::write(ac.path().join("loose.png"), b"x").unwrap();

        let r = import_from(ac.path(), app.path(), true).unwrap();
        assert_eq!((r.imported, r.already, r.converted), (2, 0, 2));
        assert!(r.failed.is_empty());
        assert!(app
            .path()
            .join("Itzy/Octane")
            .join(convert::ZONES_FILE)
            .is_file());
        // The AlphaConsole originals are untouched.
        assert!(!ac
            .path()
            .join("Itzy/Octane")
            .join(convert::ZONES_FILE)
            .exists());

        let again = import_from(ac.path(), app.path(), true).unwrap();
        assert_eq!((again.imported, again.already), (0, 2));
    }

    #[test]
    fn plain_copy_without_conversion() {
        let ac = tempfile::tempdir().unwrap();
        let app = tempfile::tempdir().unwrap();
        ac_pack(ac.path(), "Itzy");
        let r = import_from(ac.path(), app.path(), false).unwrap();
        assert_eq!((r.imported, r.converted), (1, 0));
        assert!(app.path().join("Itzy/Octane/Template.json").is_file());
        assert!(!app
            .path()
            .join("Itzy/Octane")
            .join(convert::ZONES_FILE)
            .exists());
    }
}
