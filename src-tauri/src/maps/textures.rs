//! "Workshop Textures": the 14 stock UDK engine packages custom maps made
//! with the map-making kit reference (materials, skybox…). Without them a
//! map shows default/missing textures.
//!
//! The pack is not shipped: on explicit user action it is downloaded from
//! its bakkesplugins upload (`/maps/96`, `Purpose::Maps`), and only the 14
//! known file names are taken from the zip. They go to the
//! `CookedPCConsole/` root (where every guide and loader puts them; none
//! exists in the stock install) through `game::writer` (`Owner::MapTextures`),
//! so removing them puts the install back exactly as it was. They are not
//! tied to the game build: nothing to rebuild after an update.

use std::io::{Read, Write};
use std::path::Path;

use serde::Serialize;
use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::base::security::{http_client, Purpose};
use crate::game::writer::{self, EntryState, Owner, Placement};
use crate::game::{install, ReapplyOutcome};
use crate::maps::model::DownloadPhase;
use crate::maps::{library, sources};

/// bakkesplugins upload id of the pack ("Workshop Textures").
pub const REMOTE_ID: &str = "96";
const TAG: &str = "workshop-textures";
/// The pack's files (the list every map loader checks).
pub const FILES: [&str; 14] = [
    "EditorLandscapeResources.upk",
    "EditorMaterials.upk",
    "EditorMeshes.upk",
    "EditorResources.upk",
    "EngineDebugMaterials.upk",
    "EngineMaterials.upk",
    "EngineResources.upk",
    "MapTemplateIndex.upk",
    "MapTemplates.upk",
    "NodeBuddies.upk",
    "Engine_MI_Shaders.upk",
    "EngineVolumetrics.upk",
    "EngineBuildings.upk",
    "mods.upk",
];
/// Largest file of the pack is ~31 MB.
const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TexturesStatus {
    /// All 14 files are in `CookedPCConsole/`.
    pub installed: bool,
    pub present: u32,
    pub total: u32,
    /// Installed by this app (removable from it).
    pub managed: bool,
}

/// Canonical file name when `name` is one of the pack's files.
pub(crate) fn pack_file(name: &str) -> Option<&'static str> {
    FILES.iter().copied().find(|f| f.eq_ignore_ascii_case(name))
}

pub fn status(app: &AppHandle) -> AppResult<TexturesStatus> {
    let rl = install::current_install(app)?;
    let present = FILES
        .iter()
        .filter(|f| rl.cooked_dir.join(f).is_file())
        .count() as u32;
    Ok(TexturesStatus {
        installed: present == FILES.len() as u32,
        present,
        total: FILES.len() as u32,
        managed: !writer::entries_for(Owner::MapTextures)?.is_empty(),
    })
}

/// Reads the pack's files out of the downloaded zip (anything else in it is
/// ignored). Fails unless all 14 are there.
fn read_pack(zip_path: &Path) -> AppResult<Vec<(&'static str, Vec<u8>)>> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::InvalidInput(format!("textures archive: {e}")))?;
    let mut out: Vec<(&'static str, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::InvalidInput(format!("textures archive: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry
            .enclosed_name()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        else {
            continue;
        };
        let Some(canonical) = pack_file(&name) else {
            continue;
        };
        if out.iter().any(|(n, _)| *n == canonical) {
            continue;
        }
        if entry.size() > MAX_FILE_BYTES {
            return Err(AppError::InvalidInput(format!("{canonical} is too large")));
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .by_ref()
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        out.push((canonical, bytes));
    }
    if out.len() != FILES.len() {
        let missing: Vec<&str> = FILES
            .iter()
            .copied()
            .filter(|f| !out.iter().any(|(n, _)| n == f))
            .collect();
        return Err(AppError::InvalidInput(format!(
            "the textures download lacks {}",
            missing.join(", ")
        )));
    }
    Ok(out)
}

/// Downloads the pack and installs its 14 files. Progress on
/// `maps://download` (`bakkesPlugins:96`). Refused while the game runs.
pub async fn install(app: &AppHandle) -> AppResult<TexturesStatus> {
    if crate::game::watcher::probe_now().running {
        return Err(AppError::GameRunning);
    }
    let rl = install::current_install(app)?;
    let file = sources::latest_bp_file(REMOTE_ID).await?;
    let staging = library::new_staging()?;
    let zip_path = staging.join("textures.zip");

    let result: AppResult<()> = async {
        let mut resp = http_client(Purpose::Maps)?.get(file.url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Network(format!(
                "download answered HTTP {}",
                resp.status().as_u16()
            )));
        }
        let total = resp.content_length().or(file.size);
        let mut out = std::fs::File::create(&zip_path)?;
        let (mut downloaded, mut last) = (0u64, 0u64);
        sources::emit_progress(app, REMOTE_ID, DownloadPhase::Download, 0, total);
        while let Some(chunk) = resp.chunk().await? {
            downloaded += chunk.len() as u64;
            if downloaded > sources::MAX_DOWNLOAD_BYTES {
                return Err(AppError::InvalidInput("download too large".into()));
            }
            out.write_all(&chunk)?;
            if downloaded - last >= 512 * 1024 {
                last = downloaded;
                sources::emit_progress(app, REMOTE_ID, DownloadPhase::Download, downloaded, total);
            }
        }
        out.flush()?;
        drop(out);

        sources::emit_progress(app, REMOTE_ID, DownloadPhase::Extract, downloaded, total);
        let (app2, zip2, cooked) = (app.clone(), zip_path.clone(), rl.cooked_dir.clone());
        tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
            let pack = read_pack(&zip2)?;
            for (name, bytes) in &pack {
                if let Err(err) = writer::install_file(
                    &app2,
                    Owner::MapTextures,
                    TAG,
                    &cooked.join(name),
                    Placement::RootReplace,
                    bytes,
                ) {
                    // Never leave half the pack installed.
                    let _ = writer::restore_owner(&app2, Owner::MapTextures, None);
                    return Err(err);
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))??;
        sources::emit_progress(app, REMOTE_ID, DownloadPhase::Done, downloaded, total);
        Ok(())
    }
    .await;

    let _ = std::fs::remove_dir_all(&staging);
    if let Err(err) = result {
        sources::emit_progress(app, REMOTE_ID, DownloadPhase::Failed, 0, None);
        return Err(err);
    }
    tracing::info!("workshop textures installed");
    status(app)
}

/// Removes the files this app installed (refused while the game runs).
pub fn remove(app: &AppHandle) -> AppResult<TexturesStatus> {
    writer::restore_owner(app, Owner::MapTextures, None)?;
    tracing::info!("workshop textures removed");
    status(app)
}

/// After a game update / verify: the pack does not depend on the build, so
/// only files the game removed matter. They are forgotten and the user is
/// told to reinstall (a download is never started on its own).
pub fn reapply(app: &AppHandle) -> AppResult<ReapplyOutcome> {
    let broken: Vec<String> = writer::audit(app)?
        .into_iter()
        .filter(|a| a.entry.owner == Owner::MapTextures && a.state != EntryState::Intact)
        .map(|a| a.entry.rel_path)
        .collect();
    if broken.is_empty() {
        return Ok(ReapplyOutcome::NothingToDo);
    }
    writer::forget(Owner::MapTextures, &broken)?;
    Ok(ReapplyOutcome::NeedsUser(
        "custom map textures were removed by the game — install them again from Maps".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_names_match_case_insensitively() {
        assert_eq!(pack_file("mods.upk"), Some("mods.upk"));
        assert_eq!(
            pack_file("ENGINEMATERIALS.UPK"),
            Some("EngineMaterials.upk")
        );
        assert_eq!(pack_file("Engine.upk"), None);
        assert_eq!(pack_file("../mods.upk"), None);
        assert_eq!(FILES.len(), 14);
    }

    #[test]
    fn reads_only_the_pack_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for f in FILES {
            zip.start_file(format!("Workshop-textures/{f}"), opts)
                .unwrap();
            zip.write_all(f.as_bytes()).unwrap();
        }
        zip.start_file("readme.txt", opts).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        let pack = read_pack(&path).unwrap();
        assert_eq!(pack.len(), 14);
        assert!(pack.iter().all(|(n, b)| b == n.as_bytes()));

        let partial = dir.path().join("p.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&partial).unwrap());
        zip.start_file("mods.upk", opts).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        assert!(read_pack(&partial).is_err());
    }
}
