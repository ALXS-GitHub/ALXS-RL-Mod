//! Community map sources.
//!
//! - **bakkesplugins.com** — public JSON API:
//!   `GET /api/rocket-league-maps?page=&pageSize=&search=&sortBy=` (paged
//!   list, `sortBy` = downloads | newest | rating | views),
//!   `GET /api/rocket-league-maps/{id}` (detail, with `files[]`),
//!   `GET /api/rocket-league-maps/{id}/versions` (same `files[]`, fallback).
//!   A file is an `edgeUrl` zip on `cdn.bakkesplugins.com`; its `fileHash`
//!   is the SHA-256 of the map package *inside* the zip (`fileName`), not
//!   of the zip. Fully downloadable.
//! - **lethamyr.com** — server-rendered HTML list (`/maps?page=N`, cards
//!   `<a href="/maps/{id}">` with cover image, title and subtitle). Its
//!   download links redirect to Google Drive, which is outside our network
//!   allowlist: Lethamyr maps are browse-only, the user downloads in the
//!   browser and imports the file.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::base::error::{AppError, AppResult};
use crate::base::security::{ensure_allowed, http_client, Purpose};
use crate::maps::library;
use crate::maps::model::{
    display_name, BrowseResult, DownloadPhase, DownloadProgress, MapEntry, MapOrigin, MapSort,
    RemoteMap, RemoteSource,
};

pub const EVENT_DOWNLOAD: &str = "maps://download";
const BP_API: &str = "https://bakkesplugins.com/api/rocket-league-maps";
const BP_SITE: &str = "https://bakkesplugins.com/maps";
const LETHAMYR_SITE: &str = "https://lethamyr.com";
const PAGE_SIZE: u32 = 24;
const MAX_DOWNLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

// ── bakkesplugins ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BpPage {
    #[serde(default)]
    items: Vec<BpMap>,
    total_pages: Option<u32>,
    #[serde(default)]
    has_next_page: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BpMap {
    id: u64,
    name: String,
    short_description: Option<String>,
    description: Option<String>,
    banner_url: Option<String>,
    updated_at: Option<String>,
    #[serde(default)]
    tags: Vec<BpTag>,
    member: Option<BpMember>,
    latest_version_file_size_bytes: Option<u64>,
    download_count: Option<u64>,
    average_rating: Option<f64>,
    rating_count: Option<u32>,
    latest_version_string: Option<String>,
    /// Present on the detail endpoint only (same shape as `/versions`).
    files: Option<Vec<BpVersion>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BpTag {
    short_name: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BpMember {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BpVersion {
    edge_url: Option<String>,
    file_name: Option<String>,
    file_size_bytes: Option<u64>,
    file_hash: Option<String>,
    version_string: Option<String>,
    created_at: Option<String>,
}

fn bp_to_remote(m: BpMap, installed: &HashMap<(RemoteSource, String), String>) -> RemoteMap {
    let remote_id = m.id.to_string();
    RemoteMap {
        source: RemoteSource::BakkesPlugins,
        installed_map_id: installed
            .get(&(RemoteSource::BakkesPlugins, remote_id.clone()))
            .cloned(),
        page_url: format!("{BP_SITE}/{remote_id}"),
        remote_id,
        name: m.name,
        author: m.member.and_then(|mm| mm.display_name),
        description: m.short_description.or(m.description),
        // Banners live on cdn.bakkesplugins.com; anything else is dropped.
        preview_url: m
            .banner_url
            .filter(|u| ensure_allowed(u, Purpose::Maps).is_ok()),
        size_bytes: m.latest_version_file_size_bytes,
        tags: m
            .tags
            .into_iter()
            .filter_map(|t| t.short_name.or(t.key))
            .collect(),
        updated_at: m.updated_at,
        download_count: m.download_count,
        average_rating: m.average_rating.filter(|r| r.is_finite()),
        rating_count: m.rating_count,
        latest_version_string: m.latest_version_string,
        downloadable: true,
    }
}

/// Newest downloadable file of a map (by `createdAt`).
fn latest_version(versions: Vec<BpVersion>) -> Option<BpVersion> {
    versions
        .into_iter()
        .filter(|v| v.edge_url.as_deref().is_some_and(|u| !u.is_empty()))
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
}

/// Picks the extracted package the published `fileHash` covers: the one
/// named `file_name` (base name, case-insensitive, raw or sanitised as
/// extraction does), otherwise the only package. Pure (tested).
pub fn hashed_package<'a>(file_name: Option<&str>, packages: &'a [String]) -> Option<&'a str> {
    let wanted = file_name
        .map(|n| n.rsplit(['/', '\\']).next().unwrap_or(n).trim())
        .filter(|n| !n.is_empty());
    if let Some(wanted) = wanted {
        let sanitised = crate::base::fsx::sanitize_file_name(wanted);
        if let Some(hit) = packages
            .iter()
            .find(|p| p.eq_ignore_ascii_case(wanted) || p.eq_ignore_ascii_case(&sanitised))
        {
            return Some(hit);
        }
    }
    match packages {
        [only] => Some(only),
        _ => None,
    }
}

/// Published hash, if usable (64 hex chars; empty or malformed → skipped).
fn published_hash(raw: Option<&str>) -> Option<&str> {
    raw.map(str::trim)
        .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> AppResult<T> {
    let url = ensure_allowed(url, Purpose::Maps)?;
    let resp = http_client(Purpose::Maps)?
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(AppError::Network(format!(
            "bakkesplugins answered HTTP {}",
            resp.status().as_u16()
        )));
    }
    resp.json::<T>()
        .await
        .map_err(|e| AppError::Network(format!("unexpected bakkesplugins answer: {e}")))
}

// ── Lethamyr ───────────────────────────────────────────────────────────────

/// Extracts map cards from the Lethamyr `/maps` HTML. Pure (tested).
pub fn parse_lethamyr_cards(html: &str) -> Vec<(String, String, Option<String>, Option<String>)> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<a href=\"/maps/") {
        let after = &rest[start + "<a href=\"/maps/".len()..];
        let Some(id_end) = after.find('"') else { break };
        let id = &after[..id_end];
        let block_end = after.find("</a>").unwrap_or(after.len());
        let block = &after[..block_end];
        rest = &after[block_end..];
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let title = between(block, "<h1", "</h1>")
            .map(strip_tags)
            .map(|t| html_unescape(t.trim()));
        let Some(title) = title.filter(|t| !t.is_empty()) else {
            continue;
        };
        let subtitle = between(block, "<h2", "</h2>")
            .map(strip_tags)
            .map(|t| html_unescape(t.trim()))
            .filter(|t| !t.is_empty());
        let img = between(block, "src=\"", "\"").map(|s| s.to_string());
        let image = img.map(|src| {
            if src.starts_with('/') {
                format!("{LETHAMYR_SITE}{src}")
            } else {
                src
            }
        });
        out.push((id.to_string(), title, subtitle, image));
    }
    out
}

fn between<'a>(hay: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = hay.find(start)? + start.len();
    let j = hay[i..].find(end)? + i;
    Some(&hay[i..j])
}

/// Drops the remainder of an opening tag (`class="…">text`) and inner tags.
fn strip_tags(s: &str) -> String {
    let content = s.split_once('>').map(|(_, c)| c).unwrap_or(s);
    let mut out = String::new();
    let mut in_tag = false;
    for c in content.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&#039;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn lethamyr_has_next(html: &str, page: u32) -> bool {
    html.contains(&format!("maps?page={}", page + 1))
}

// ── Public API ─────────────────────────────────────────────────────────────

fn installed_index(library: &[MapEntry]) -> HashMap<(RemoteSource, String), String> {
    library
        .iter()
        .filter_map(|m| match &m.origin {
            MapOrigin::Remote {
                source, remote_id, ..
            } => Some(((*source, remote_id.clone()), m.id.clone())),
            _ => None,
        })
        .collect()
}

pub async fn browse(
    source: RemoteSource,
    query: &str,
    page: u32,
    sort: MapSort,
) -> AppResult<BrowseResult> {
    let page = page.max(1);
    let installed = installed_index(&library::list().unwrap_or_default());
    match source {
        RemoteSource::BakkesPlugins => {
            let mut url = format!(
                "{BP_API}?page={page}&pageSize={PAGE_SIZE}&sortBy={}",
                sort.as_api()
            );
            let q = query.trim();
            if !q.is_empty() {
                url.push_str(&format!("&search={}", url_encode(q)));
            }
            let data: BpPage = get_json(&url).await?;
            Ok(BrowseResult {
                source,
                page,
                total_pages: data.total_pages,
                has_next: data.has_next_page,
                items: data
                    .items
                    .into_iter()
                    .map(|m| bp_to_remote(m, &installed))
                    .collect(),
            })
        }
        RemoteSource::Lethamyr => {
            let url = ensure_allowed(&format!("{LETHAMYR_SITE}/maps?page={page}"), Purpose::Maps)?;
            let resp = http_client(Purpose::Maps)?.get(url).send().await?;
            if !resp.status().is_success() {
                return Err(AppError::Network(format!(
                    "lethamyr.com answered HTTP {}",
                    resp.status().as_u16()
                )));
            }
            let html = resp.text().await?;
            let needle = query.trim().to_lowercase();
            let items = parse_lethamyr_cards(&html)
                .into_iter()
                // The site has no search endpoint: filter the current page.
                .filter(|(_, title, sub, _)| {
                    needle.is_empty()
                        || title.to_lowercase().contains(&needle)
                        || sub
                            .as_deref()
                            .is_some_and(|s| s.to_lowercase().contains(&needle))
                })
                .map(|(id, title, subtitle, image)| RemoteMap {
                    source,
                    installed_map_id: installed
                        .get(&(RemoteSource::Lethamyr, id.clone()))
                        .cloned(),
                    page_url: format!("{LETHAMYR_SITE}/maps/{id}"),
                    remote_id: id,
                    name: title,
                    author: None,
                    description: subtitle,
                    preview_url: image.filter(|u| ensure_allowed(u, Purpose::Maps).is_ok()),
                    size_bytes: None,
                    tags: Vec::new(),
                    updated_at: None,
                    download_count: None,
                    average_rating: None,
                    rating_count: None,
                    latest_version_string: None,
                    downloadable: false,
                })
                .collect();
            Ok(BrowseResult {
                source,
                page,
                total_pages: None,
                has_next: lethamyr_has_next(&html, page),
                items,
            })
        }
    }
}

fn emit(
    app: &AppHandle,
    source: RemoteSource,
    remote_id: &str,
    phase: DownloadPhase,
    downloaded: u64,
    total: Option<u64>,
) {
    let _ = app.emit(
        EVENT_DOWNLOAD,
        DownloadProgress {
            source,
            remote_id: remote_id.to_string(),
            phase,
            downloaded,
            total,
        },
    );
}

/// Downloads a remote map into the library. Progress on `maps://download`.
pub async fn download(
    app: &AppHandle,
    source: RemoteSource,
    remote_id: &str,
) -> AppResult<MapEntry> {
    if source == RemoteSource::Lethamyr {
        return Err(AppError::Unsupported(
            "Lethamyr hosts its files on Google Drive — open the map page, download it, then import the file".into(),
        ));
    }
    if !remote_id.chars().all(|c| c.is_ascii_digit()) || remote_id.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "bad bakkesplugins id {remote_id}"
        )));
    }
    if let Some(existing) = installed_index(&library::list()?).get(&(source, remote_id.to_string()))
    {
        return library::get(existing);
    }
    let result = download_bp(app, remote_id).await;
    if result.is_err() {
        emit(app, source, remote_id, DownloadPhase::Failed, 0, None);
    }
    result
}

async fn download_bp(app: &AppHandle, remote_id: &str) -> AppResult<MapEntry> {
    let src = RemoteSource::BakkesPlugins;
    let mut detail: BpMap = get_json(&format!("{BP_API}/{remote_id}")).await?;
    // The detail usually carries `files[]`; `/versions` is the fallback.
    let versions = match detail.files.take().filter(|f| !f.is_empty()) {
        Some(files) => files,
        None => get_json(&format!("{BP_API}/{remote_id}/versions")).await?,
    };
    let version = latest_version(versions)
        .ok_or_else(|| AppError::NotFound("no downloadable version".into()))?;
    let edge_url = version.edge_url.clone().unwrap_or_default();
    let url = ensure_allowed(&edge_url, Purpose::Maps)?;
    let expected_hash = published_hash(version.file_hash.as_deref()).map(str::to_string);

    let staging = library::new_staging()?;
    let lower = edge_url.to_ascii_lowercase();
    let download_name = if lower.ends_with(".zip") {
        "download.zip".to_string()
    } else {
        version
            .file_name
            .clone()
            .map(|n| crate::base::fsx::sanitize_file_name(&n))
            .unwrap_or_else(|| "map.upk".into())
    };
    let target = staging.join(&download_name);

    let outcome: AppResult<MapEntry> = async {
        let mut resp = http_client(Purpose::Maps)?.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Network(format!(
                "download answered HTTP {}",
                resp.status().as_u16()
            )));
        }
        let total = resp.content_length().or(version.file_size_bytes);
        let mut file = std::fs::File::create(&target)?;
        let mut hasher = Sha256::new();
        let mut downloaded = 0u64;
        let mut last_emit = 0u64;
        emit(app, src, remote_id, DownloadPhase::Download, 0, total);
        while let Some(chunk) = resp.chunk().await? {
            downloaded += chunk.len() as u64;
            if downloaded > MAX_DOWNLOAD_BYTES {
                return Err(AppError::InvalidInput("download too large".into()));
            }
            hasher.update(&chunk);
            file.write_all(&chunk)?;
            if downloaded - last_emit >= 512 * 1024 {
                last_emit = downloaded;
                emit(
                    app,
                    src,
                    remote_id,
                    DownloadPhase::Download,
                    downloaded,
                    total,
                );
            }
        }
        file.flush()?;
        drop(file);

        // Hash of the raw download: the map itself when it is not a zip.
        let download_hash = hex::encode(hasher.finalize());

        emit(
            app,
            src,
            remote_id,
            DownloadPhase::Extract,
            downloaded,
            total,
        );
        let is_zip = download_name == "download.zip";
        if is_zip {
            let (zip_path, staging_dir) = (target.clone(), staging.clone());
            let count = tauri::async_runtime::spawn_blocking(move || -> AppResult<usize> {
                let n = library::extract_zip(&zip_path, &staging_dir)?;
                let _ = std::fs::remove_file(&zip_path);
                Ok(n)
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))??;
            if count == 0 {
                return Err(AppError::InvalidInput(
                    "the download contains no .upk/.udk map".into(),
                ));
            }
        }

        emit(
            app,
            src,
            remote_id,
            DownloadPhase::Verify,
            downloaded,
            total,
        );
        if let Some(expected) = expected_hash {
            // `fileHash` covers the package inside the zip; a match on the
            // raw download is accepted too (non-zip uploads).
            let verified = download_hash.eq_ignore_ascii_case(&expected)
                || (is_zip && {
                    let (staging_dir, file_name) = (staging.clone(), version.file_name.clone());
                    tauri::async_runtime::spawn_blocking(move || {
                        verify_extracted(&staging_dir, file_name.as_deref(), &expected)
                    })
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))??
                });
            if !verified {
                return Err(AppError::HashMismatch(format!(
                    "bakkesplugins map {remote_id}"
                )));
            }
        }

        let name = if detail.name.trim().is_empty() {
            display_name(&download_name)
        } else {
            detail.name.clone()
        };
        let entry = library::finalize_staging(
            &staging,
            name,
            MapOrigin::Remote {
                source: src,
                remote_id: remote_id.to_string(),
                version: version.version_string.clone(),
            },
            detail.member.as_ref().and_then(|m| m.display_name.clone()),
            detail
                .short_description
                .clone()
                .or(detail.description.clone()),
            detail
                .banner_url
                .clone()
                .filter(|u| ensure_allowed(u, Purpose::Maps).is_ok()),
        )?;
        let mut entry = entry;
        entry.tags = detail
            .tags
            .iter()
            .filter_map(|t| t.short_name.clone().or(t.key.clone()))
            .take(8)
            .collect();
        library::save(&entry)?;
        emit(app, src, remote_id, DownloadPhase::Done, downloaded, total);
        library::get(&entry.id)
    }
    .await;

    if outcome.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    outcome
}

/// Hashes the extracted package `fileHash` refers to and compares it.
fn verify_extracted(staging: &Path, file_name: Option<&str>, expected: &str) -> AppResult<bool> {
    let packages: Vec<String> = library::packages_in(staging)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    match hashed_package(file_name, &packages) {
        Some(p) => {
            Ok(crate::base::fsx::sha256_file(&staging.join(p))?.eq_ignore_ascii_case(expected))
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LETH_SAMPLE: &str = r#"
      <a href="/maps/192" class="bg-card">
        <img loading="lazy" class="w-full" src="/storage/01JF5T5XG7.jpg" alt="The Apex - Halo Cover Image" />
        <div class="mt-3 mx-4">
          <h1 class="inline text-main-header">The Apex &amp; Halo</h1>
          <p class="inline ml-2"></p>
        </div>
        <h2 class="ml-4 mt-1"> Floating Rings by Leth</h2>
      </a>
      <a href="/maps/abc">not a map</a>
      <a href="/maps/191" class="bg-card"><img src="https://lethamyr.com/storage/x.jpg"/><h1 class="x">Gradient</h1></a>
      <a href="https://lethamyr.com/maps?page=2">2</a>
    "#;

    #[test]
    fn parses_lethamyr_cards() {
        let cards = parse_lethamyr_cards(LETH_SAMPLE);
        assert_eq!(cards.len(), 2);
        let (id, title, sub, img) = &cards[0];
        assert_eq!(id, "192");
        assert_eq!(title, "The Apex & Halo");
        assert_eq!(sub.as_deref(), Some("Floating Rings by Leth"));
        assert_eq!(
            img.as_deref(),
            Some("https://lethamyr.com/storage/01JF5T5XG7.jpg")
        );
        assert_eq!(cards[1].1, "Gradient");
        assert!(cards[1].2.is_none());
        assert!(lethamyr_has_next(LETH_SAMPLE, 1));
        assert!(!lethamyr_has_next(LETH_SAMPLE, 2));
    }

    #[test]
    fn parses_bakkesplugins_page() {
        let raw = r#"{"items":[{"id":364,"name":"Cheese Grater","shortDescription":"It grates cheese...",
          "bannerUrl":"https://cdn.bakkesplugins.com/uploads/a.jpg","updatedAt":"2026-09-18T06:32:52Z",
          "tags":[{"key":"rumble","shortName":"Rumble"}],"member":{"displayName":"aCookieSnatcher"},
          "latestVersionFileSizeBytes":36004810}],"totalCount":345,"page":1,"pageSize":2,"totalPages":173,
          "hasNextPage":true}"#;
        let page: BpPage = serde_json::from_str(raw).unwrap();
        assert!(page.has_next_page);
        let m = bp_to_remote(page.items.into_iter().next().unwrap(), &HashMap::new());
        assert_eq!(m.remote_id, "364");
        assert_eq!(m.author.as_deref(), Some("aCookieSnatcher"));
        assert_eq!(m.tags, vec!["Rumble"]);
        assert!(m.preview_url.is_some());
        assert!(m.downloadable);
    }

    #[test]
    fn rejects_foreign_banner() {
        let raw = r#"{"id":1,"name":"x","bannerUrl":"https://evil.example/a.jpg"}"#;
        let m: BpMap = serde_json::from_str(raw).unwrap();
        assert!(bp_to_remote(m, &HashMap::new()).preview_url.is_none());
    }

    #[test]
    fn picks_the_hashed_package() {
        let pk = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // Named file wins, case-insensitive, even among several packages.
        let many = pk(&["Textures.upk", "Turn_By_Turn.udk"]);
        assert_eq!(
            hashed_package(Some("turn_by_turn.UDK"), &many),
            Some("Turn_By_Turn.udk")
        );
        // A path in `fileName` is reduced to its base name.
        assert_eq!(
            hashed_package(Some("maps/Turn_By_Turn.udk"), &many),
            Some("Turn_By_Turn.udk")
        );
        // Name as sanitised by the extraction.
        let odd = pk(&["My_Map_.upk", "b.upk"]);
        assert_eq!(
            hashed_package(Some("My Map!.upk"), &pk(&["My Map_.upk"])),
            Some("My Map_.upk")
        );
        assert_eq!(
            hashed_package(Some("My:Map?.upk"), &odd),
            Some("My_Map_.upk")
        );
        // Unknown or missing name: only a single package is accepted.
        let one = pk(&["Other.upk"]);
        assert_eq!(
            hashed_package(Some("Turn_By_Turn.udk"), &one),
            Some("Other.upk")
        );
        assert_eq!(hashed_package(None, &one), Some("Other.upk"));
        assert_eq!(hashed_package(Some(""), &one), Some("Other.upk"));
        assert_eq!(hashed_package(Some("nope.udk"), &many), None);
        assert_eq!(hashed_package(None, &many), None);
        assert_eq!(hashed_package(Some("a.upk"), &[]), None);
    }

    #[test]
    fn published_hash_is_validated() {
        let h = "219B028C29C7649543709636C38ECA6F5DC35F35DCDC2FA0D0512A4586014FF3";
        assert_eq!(published_hash(Some(h)), Some(h));
        assert_eq!(published_hash(Some("")), None);
        assert_eq!(published_hash(None), None);
        assert_eq!(published_hash(Some(&"z".repeat(64))), None);
    }

    #[test]
    fn uses_detail_files_and_latest_version() {
        let raw = r#"{"id":294,"name":"Turn By Turn","downloadCount":43,"averageRating":0,
          "ratingCount":0,"files":[
            {"fileName":"Old.udk","edgeUrl":"https://cdn.bakkesplugins.com/uploads/a-0.9.zip",
             "fileHash":"","versionString":"0.9","createdAt":"2026-02-01T00:00:00Z"},
            {"fileName":"Turn_By_Turn.udk","edgeUrl":"https://cdn.bakkesplugins.com/uploads/b-1.0.0.zip",
             "fileSizeBytes":20956,"fileHash":"219B028C29C7649543709636C38ECA6F5DC35F35DCDC2FA0D0512A4586014FF3",
             "versionString":"1.0.0","createdAt":"2026-03-01T18:53:34Z"},
            {"fileName":"Broken.udk","createdAt":"2027-01-01T00:00:00Z"}]}"#;
        let mut m: BpMap = serde_json::from_str(raw).unwrap();
        let v = latest_version(m.files.take().unwrap()).unwrap();
        assert_eq!(v.version_string.as_deref(), Some("1.0.0"));
        assert_eq!(v.file_name.as_deref(), Some("Turn_By_Turn.udk"));
        let r = bp_to_remote(m, &HashMap::new());
        assert_eq!(r.download_count, Some(43));
        assert_eq!(r.rating_count, Some(0));
        // List items have no `files`.
        let item: BpMap = serde_json::from_str(r#"{"id":1,"name":"x"}"#).unwrap();
        assert!(item.files.is_none());
    }

    #[test]
    fn encodes_query() {
        assert_eq!(url_encode("dribble 2&x"), "dribble%202%26x");
    }
}
