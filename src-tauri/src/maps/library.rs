//! Map library on disk: listing (with transparent legacy migration),
//! imports (files, folders, zips), edits and deletion.

use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};
use crate::maps::model::{
    display_name, migrate_legacy_meta, new_map_id, normalize_target, pick_main_package, MapEntry,
    MapOrigin, MapPatch, ENTRY_FILE, LEGACY_META_FILE, MAP_EXTENSIONS,
};

/// Hard cap on a single extracted package (a map is < 1 GB in practice).
const MAX_PACKAGE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// `jfif` is plain JPEG (Workshop downloads use it); copied as `.jpg`.
const PREVIEW_EXTENSIONS: &[&str] = &["jpg", "jpeg", "jfif", "png", "webp"];

pub fn library_dir() -> AppResult<PathBuf> {
    paths::data_subdir("maps")
}

pub fn map_dir(id: &str) -> AppResult<PathBuf> {
    if id.is_empty() || id.contains(['/', '\\', '.']) {
        return Err(AppError::InvalidInput(format!("bad map id {id:?}")));
    }
    Ok(library_dir()?.join(id))
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

fn is_package(p: &Path) -> bool {
    MAP_EXTENSIONS.contains(&ext_of(p).as_str())
}

/// Packages directly inside a map folder, `(file name, size)`.
fn packages_in(dir: &Path) -> Vec<(String, u64)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_package(p))
        .filter_map(|p| {
            let size = p.metadata().ok()?.len();
            Some((p.file_name()?.to_string_lossy().into_owned(), size))
        })
        .collect()
}

/// Library file name of a preview image with extension `ext`.
fn preview_name(ext: &str) -> String {
    match ext {
        "jfif" | "jpeg" => "preview.jpg".into(),
        other => format!("preview.{other}"),
    }
}

/// Maps imported from a BakkesMod Workshop folder by the previous app came
/// without their image: fetch it from that folder if it is still there.
fn backfill_workshop_preview(dir: &Path, folder_name: &str) -> Option<String> {
    if folder_name.is_empty() || folder_name.contains(['/', '\\']) || folder_name.contains("..") {
        return None;
    }
    let source = paths::roaming_dir()?
        .join("bakkesmod")
        .join("bakkesmod")
        .join("Workshop")
        .join(folder_name);
    let image = preview_in(&source)?;
    let name = preview_name(&ext_of(Path::new(&image)));
    std::fs::copy(source.join(&image), dir.join(&name)).ok()?;
    Some(name)
}

fn preview_in(dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_file() && PREVIEW_EXTENSIONS.contains(&ext_of(p).as_str()))
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

/// Reads one map folder. New format first, then the previous app's
/// `meta.json` (migrated to `map.json` on the fly, original untouched).
fn read_entry(dir: &Path) -> Option<MapEntry> {
    let entry_path = dir.join(ENTRY_FILE);
    let mut entry = if entry_path.is_file() {
        let raw = std::fs::read(&entry_path).ok()?;
        match serde_json::from_slice::<MapEntry>(&raw) {
            Ok(e) => e,
            Err(err) => {
                tracing::warn!(?dir, %err, "unreadable map.json, skipping");
                return None;
            }
        }
    } else {
        let legacy = dir.join(LEGACY_META_FILE);
        let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(&legacy).ok()?).ok()?;
        let packages = packages_in(dir);
        let main = pick_main_package(&packages)?;
        let size = packages
            .iter()
            .find(|(n, _)| *n == main)
            .map(|(_, s)| *s)
            .unwrap_or(0);
        let mut migrated = migrate_legacy_meta(&raw, &main, size)?;
        migrated.companions = packages
            .into_iter()
            .map(|(n, _)| n)
            .filter(|n| *n != main)
            .collect();
        migrated.preview_file = preview_in(dir);
        if let Err(err) = fsx::write_json(&entry_path, &migrated) {
            tracing::warn!(?dir, %err, "could not persist migrated map entry");
        } else {
            tracing::info!(id = %migrated.id, "migrated legacy map meta");
        }
        migrated
    };
    // The folder name is the source of truth for the id.
    if let Some(name) = dir.file_name() {
        entry.id = name.to_string_lossy().into_owned();
    }
    if !dir.join(&entry.file_name).is_file() {
        tracing::warn!(?dir, file = %entry.file_name, "map package missing, skipping");
        return None;
    }
    if entry
        .preview_file
        .as_ref()
        .is_none_or(|f| !dir.join(f).is_file())
    {
        let found = preview_in(dir).or_else(|| match &entry.origin {
            MapOrigin::BakkesMod { folder_name } => backfill_workshop_preview(dir, folder_name),
            _ => None,
        });
        if found.is_some() {
            entry.preview_file = found;
            if let Err(err) = fsx::write_json(&dir.join(ENTRY_FILE), &entry) {
                tracing::warn!(?dir, %err, "could not save the map preview");
            }
        }
    }
    entry.folder = dir.to_string_lossy().into_owned();
    Some(entry)
}

pub fn list() -> AppResult<Vec<MapEntry>> {
    let root = library_dir()?;
    let mut maps: Vec<MapEntry> = std::fs::read_dir(&root)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .filter_map(|p| read_entry(&p))
        .collect();
    maps.sort_by(|a, b| {
        b.favorite
            .cmp(&a.favorite)
            .then(b.added_at.cmp(&a.added_at))
    });
    Ok(maps)
}

pub fn get(id: &str) -> AppResult<MapEntry> {
    read_entry(&map_dir(id)?).ok_or_else(|| AppError::NotFound(format!("map {id}")))
}

pub fn save(entry: &MapEntry) -> AppResult<()> {
    fsx::write_json(&map_dir(&entry.id)?.join(ENTRY_FILE), entry)
}

pub fn update(id: &str, patch: MapPatch) -> AppResult<MapEntry> {
    let mut entry = get(id)?;
    if let Some(name) = patch.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidInput("empty map name".into()));
        }
        entry.name = trimmed.chars().take(120).collect();
    }
    if let Some(f) = patch.favorite {
        entry.favorite = f;
    }
    if let Some(tags) = patch.tags {
        let mut clean: Vec<String> = tags
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .take(12)
            .collect();
        clean.dedup();
        entry.tags = clean;
    }
    if let Some(target) = patch.preferred_target {
        entry.preferred_target = match target {
            Some(t) => Some(
                normalize_target(&t)
                    .ok_or_else(|| AppError::InvalidInput(format!("unknown Labs target {t}")))?
                    .to_string(),
            ),
            None => None,
        };
    }
    save(&entry)?;
    get(id)
}

pub fn delete(id: &str) -> AppResult<()> {
    let dir = map_dir(id)?;
    if !dir.is_dir() {
        return Err(AppError::NotFound(format!("map {id}")));
    }
    std::fs::remove_dir_all(&dir)?;
    tracing::info!(%id, "map deleted");
    Ok(())
}

pub fn touch_played(id: &str) {
    if let Ok(mut e) = get(id) {
        e.last_played_at = Some(Utc::now());
        let _ = save(&e);
    }
}

/// Creates a new map folder from packages already written in `staging`
/// (a temp dir inside the library). Returns the saved entry.
pub fn finalize_staging(
    staging: &Path,
    name: String,
    origin: MapOrigin,
    author: Option<String>,
    description: Option<String>,
    preview_url: Option<String>,
) -> AppResult<MapEntry> {
    let packages = packages_in(staging);
    let main = pick_main_package(&packages)
        .ok_or_else(|| AppError::InvalidInput("no .upk/.udk map found".into()))?;
    let size = packages
        .iter()
        .find(|(n, _)| *n == main)
        .map(|(_, s)| *s)
        .unwrap_or(0);
    let id = new_map_id();
    let dest = map_dir(&id)?;
    std::fs::rename(staging, &dest)?;
    let entry = MapEntry {
        id: id.clone(),
        name,
        author,
        description,
        tags: Vec::new(),
        origin,
        companions: packages
            .into_iter()
            .map(|(n, _)| n)
            .filter(|n| *n != main)
            .collect(),
        file_name: main,
        preview_file: preview_in(&dest),
        preview_url,
        size_bytes: size,
        added_at: Utc::now(),
        favorite: false,
        preferred_target: None,
        last_played_at: None,
        folder: String::new(),
    };
    save(&entry)?;
    get(&id)
}

/// A fresh staging dir inside the library (same volume → cheap rename).
pub fn new_staging() -> AppResult<PathBuf> {
    let dir = library_dir()?.join(format!(".staging-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Removes leftovers of interrupted imports/downloads.
pub fn clean_staging() {
    let Ok(root) = library_dir() else { return };
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with(".staging-") || name.starts_with(".download-") {
            let _ = std::fs::remove_dir_all(e.path());
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Extracts every package (and the first preview image) of a zip into `dest`.
pub fn extract_zip(zip_path: &Path, dest: &Path) -> AppResult<usize> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut count = 0usize;
    let mut has_preview = false;
    for i in 0..archive.len() {
        let mut item = archive.by_index(i)?;
        if !item.is_file() {
            continue;
        }
        // `enclosed_name` rejects absolute paths and `..` (zip-slip).
        let Some(inner) = item.enclosed_name() else {
            continue;
        };
        let Some(file_name) = inner.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let ext = ext_of(Path::new(&file_name));
        let wanted = MAP_EXTENSIONS.contains(&ext.as_str())
            || (!has_preview && PREVIEW_EXTENSIONS.contains(&ext.as_str()));
        if !wanted {
            continue;
        }
        if item.size() > MAX_PACKAGE_BYTES {
            return Err(AppError::InvalidInput(format!("{file_name} is too large")));
        }
        let target = if PREVIEW_EXTENSIONS.contains(&ext.as_str()) {
            has_preview = true;
            dest.join(preview_name(&ext))
        } else {
            count += 1;
            dest.join(fsx::sanitize_file_name(&file_name))
        };
        let mut out = std::fs::File::create(&target)?;
        std::io::copy(&mut (&mut item).take(MAX_PACKAGE_BYTES), &mut out)?;
    }
    Ok(count)
}

/// Imports files or folders picked by the user. Returns the new entries.
/// Folders are walked: every folder containing packages becomes one map
/// (BakkesMod workshop layout).
pub fn import_paths(paths: &[PathBuf]) -> AppResult<Vec<MapEntry>> {
    let mut created = Vec::new();
    for p in paths {
        let result = if p.is_dir() {
            import_folder_tree(p)
        } else {
            import_single_file(p).map(|e| vec![e])
        };
        match result {
            Ok(mut entries) => created.append(&mut entries),
            Err(err) if paths.len() > 1 => {
                tracing::warn!(path = %p.display(), %err, "import skipped")
            }
            Err(err) => return Err(err),
        }
    }
    Ok(created)
}

fn import_single_file(path: &Path) -> AppResult<MapEntry> {
    if !path.is_file() {
        return Err(AppError::FileNotFound(path.display().to_string()));
    }
    let ext = ext_of(path);
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let staging = new_staging()?;
    let result = (|| {
        if ext == "zip" {
            if extract_zip(path, &staging)? == 0 {
                return Err(AppError::InvalidInput(
                    "the archive contains no .upk/.udk map".into(),
                ));
            }
        } else if MAP_EXTENSIONS.contains(&ext.as_str()) {
            std::fs::copy(path, staging.join(fsx::sanitize_file_name(&file_name)))?;
            copy_sibling_preview(path, &staging);
        } else {
            return Err(AppError::InvalidInput(format!(
                "unsupported file type .{ext}"
            )));
        }
        finalize_staging(
            &staging,
            display_name(&file_name),
            MapOrigin::Imported,
            None,
            None,
            None,
        )
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}

fn copy_sibling_preview(package: &Path, staging: &Path) {
    let Some(dir) = package.parent() else { return };
    if let Some(name) = preview_in(dir) {
        let ext = ext_of(Path::new(&name));
        let _ = std::fs::copy(dir.join(&name), staging.join(preview_name(&ext)));
    }
}

fn import_folder_tree(root: &Path) -> AppResult<Vec<MapEntry>> {
    let mut created = Vec::new();
    let mut map_dirs: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .max_depth(6)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_dir())
        .map(|e| e.into_path())
        .filter(|d| !packages_in(d).is_empty())
        .collect();
    map_dirs.sort();
    for dir in map_dirs {
        let staging = new_staging()?;
        let folder_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let res = (|| {
            for (name, _) in packages_in(&dir) {
                std::fs::copy(
                    dir.join(&name),
                    staging.join(fsx::sanitize_file_name(&name)),
                )?;
            }
            if let Some(prev) = preview_in(&dir) {
                let ext = ext_of(Path::new(&prev));
                let _ = std::fs::copy(dir.join(&prev), staging.join(preview_name(&ext)));
            }
            finalize_staging(
                &staging,
                display_name(&folder_name),
                MapOrigin::BakkesMod {
                    folder_name: folder_name.clone(),
                },
                None,
                None,
                None,
            )
        })();
        match res {
            Ok(e) => created.push(e),
            Err(err) => {
                let _ = std::fs::remove_dir_all(&staging);
                tracing::warn!(dir = %dir.display(), %err, "folder import skipped");
            }
        }
    }
    if created.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "no map found in {}",
            root.display()
        )));
    }
    Ok(created)
}
