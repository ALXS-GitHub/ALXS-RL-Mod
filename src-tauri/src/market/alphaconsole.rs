//! alphaconsole.io — AlphaConsole's community library.
//!
//! The site is a Blazor Server app without a public JSON API, but its first
//! HTML response is server-rendered and honours the query string
//! (`textures=decal|ball`, `sort=`, `search=`): 20 cards per query, no
//! paging over plain HTTP. Packs are public zips on `s3.alphaconsole.io`.
//! Items link back to their page and credit their author; nothing is
//! mirrored.

use url::Url;

use crate::base::error::{AppError, AppResult};
use crate::base::security::{self, Purpose};
use crate::market::model::{MarketItem, MarketKind, MarketPage, MarketSort, MarketSource};

const SITE: &str = "https://alphaconsole.io";
const FILES: &str = "https://s3.alphaconsole.io";
/// Packs are a few MB; refuse anything absurd.
pub const MAX_PACK_BYTES: usize = 64 * 1024 * 1024;

fn sort_param(sort: MarketSort) -> &'static str {
    match sort {
        MarketSort::Downloads => "downloads",
        MarketSort::Likes => "likes",
        MarketSort::Newest => "newest",
        MarketSort::Updated => "recently updated",
        MarketSort::Trending => "trending",
    }
}

fn kind_param(kind: MarketKind) -> &'static str {
    match kind {
        MarketKind::Decal => "decal",
        MarketKind::Ball => "ball",
    }
}

pub fn browse_url(kind: MarketKind, query: &str, sort: MarketSort) -> AppResult<Url> {
    let mut url =
        Url::parse(&format!("{SITE}/browse")).map_err(|e| AppError::Internal(e.to_string()))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("textures", kind_param(kind));
        q.append_pair("sort", sort_param(sort));
        if !query.trim().is_empty() {
            q.append_pair("search", query.trim());
        }
    }
    Ok(url)
}

pub async fn browse(kind: MarketKind, query: &str, sort: MarketSort) -> AppResult<MarketPage> {
    let url = browse_url(kind, query, sort)?;
    security::ensure_allowed(url.as_str(), Purpose::Market)?;
    let html = security::http_client(Purpose::Market)?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let items = parse_cards(&html, kind);
    let total = parse_total(&html);
    Ok(MarketPage {
        truncated: total.is_some_and(|t| t > items.len() as u64),
        total,
        items,
    })
}

/// AlphaConsole ids are UUIDs: anything else is refused before it reaches
/// a URL.
pub fn valid_id(id: &str) -> bool {
    id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

pub async fn download(id: &str) -> AppResult<Vec<u8>> {
    if !valid_id(id) {
        return Err(AppError::InvalidInput(format!(
            "bad AlphaConsole id {id:?}"
        )));
    }
    let url = format!("{FILES}/uploads/{id}.zip");
    security::ensure_allowed(&url, Purpose::Market)?;
    let res = security::http_client(Purpose::Market)?
        .get(&url)
        .send()
        .await?
        .error_for_status()?;
    if res
        .content_length()
        .is_some_and(|n| n as usize > MAX_PACK_BYTES)
    {
        return Err(AppError::InvalidInput("pack larger than 64 MB".into()));
    }
    let bytes = res.bytes().await?;
    if bytes.len() > MAX_PACK_BYTES {
        return Err(AppError::InvalidInput("pack larger than 64 MB".into()));
    }
    Ok(bytes.to_vec())
}

fn between<'a>(hay: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = hay.find(start)? + start.len();
    let j = hay[i..].find(end)? + i;
    Some(&hay[i..j])
}

/// Decodes the entities Blazor emits: named basics and `&#…;` / `&#x…;`.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(end) = tail.find(';').filter(|&e| e <= 10) else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let entity = &tail[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "nbsp" => Some(' '),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .map(|h| u32::from_str_radix(h, 16))
                .or_else(|| entity.strip_prefix('#').map(str::parse::<u32>))
                .and_then(Result::ok)
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

fn number(s: &str) -> Option<u64> {
    s.trim().replace([',', ' '], "").parse().ok()
}

/// `Found items - 476`
fn parse_total(html: &str) -> Option<u64> {
    between(html, "Found items - ", "<").and_then(number)
}

/// One [`MarketItem`] per item card of a browse page.
pub fn parse_cards(html: &str, kind: MarketKind) -> Vec<MarketItem> {
    let mut out: Vec<MarketItem> = Vec::new();
    for card in html.split("<div class=\"card ").skip(1) {
        let Some(id) = between(card, "href=\"/view/", "\"").filter(|id| valid_id(id)) else {
            continue;
        };
        if out.iter().any(|i| i.id == id) {
            continue;
        }
        let title = between(card, "alt=\"", "\"")
            .map(unescape)
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| id.to_string());
        let thumbnail = between(card, "src=\"", "\"")
            .filter(|s| s.starts_with(FILES))
            .map(unescape);
        let author = between(card, "href=\"/user/", "</a>")
            .and_then(|s| s.split_once('>'))
            .map(|(_, name)| unescape(name))
            .filter(|a| !a.is_empty());
        let description = between(card, "class=\"card-text", "</p>")
            .and_then(|s| s.split_once('>'))
            .map(|(_, d)| unescape(d))
            .filter(|d| !d.is_empty());
        let likes = between(card, "min-width:45px;\">", "<").and_then(number);
        let downloads = card
            .split_once("/uploads/")
            .and_then(|(_, rest)| between(rest, "<span class=\"px-1\">", "<"))
            .and_then(number);
        out.push(MarketItem {
            source: MarketSource::AlphaConsole,
            id: id.to_string(),
            kind,
            title,
            author,
            description,
            thumbnail,
            page_url: Some(format!("{SITE}/view/{id}")),
            downloads,
            likes,
            bodies: Vec::new(),
            ready: false,
            installed: false,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed from a real browse page (2026-09-27).
    const CARD: &str = r##"<p class="col">Found items - 476</p><div class="row mt-1 row-gap-2"><div class="col-12 my-2"><div class="card ratio hover-zoom" style="--bs-aspect-ratio: 1;"><div class="d-flex flex-column"><div class="preview-image ratio-1x1"><a href="/view/7b8e5d6f-38f0-40de-a72d-ee5694505798" class="d-block"><img alt="SoulyBlossomDecal" title="SoulyBlossomDecal" class="preview-image-img" src="https://s3.alphaconsole.io/images/7b8e5d6f-38f0-40de-a72d-ee5694505798/7b8e5d6f-38f0-40de-a72d-ee5694505798.jpg?v=0" /></a></div>
<div class="card-body"><div class="my-auto me-2 item-label item-label-decal"><a>&nbsp;DECAL</a></div>
<a class="text-body ms-1 fs-5" href="/view/7b8e5d6f-38f0-40de-a72d-ee5694505798">SoulyBlossomDecal</a>
<p class="mb-2 text-body-secondary" style="font-size: 0.75rem;">Created By: <a class="text-body-secondary" href="/user/59f601d7-e4c4-4a16-9a08-7d9a23458788">Soul</a></p>
<p class="card-text text-body-secondary mh-100 text-maxlines-3">A beautiful flower blossom decal &amp; more</p>
<button class="btn" disabled><i class="fa-regular fa-heart"></i>
<span class="px-1" style="min-width:45px;">70</span></button>
<form method="GET" action="https://s3.alphaconsole.io/uploads/7b8e5d6f-38f0-40de-a72d-ee5694505798.zip?v=0"><button type="submit" class="btn"><i class="fa fa-download"></i>
<span class="px-1">67890</span></button></form></div></div></div></div>"##;

    #[test]
    fn parses_a_card() {
        let items = parse_cards(CARD, MarketKind::Decal);
        assert_eq!(items.len(), 1);
        let i = &items[0];
        assert_eq!(i.id, "7b8e5d6f-38f0-40de-a72d-ee5694505798");
        assert_eq!(i.title, "SoulyBlossomDecal");
        assert_eq!(i.author.as_deref(), Some("Soul"));
        assert_eq!(
            i.description.as_deref(),
            Some("A beautiful flower blossom decal & more")
        );
        assert_eq!(i.likes, Some(70));
        assert_eq!(i.downloads, Some(67_890));
        assert!(i.thumbnail.as_deref().unwrap().starts_with(FILES));
        assert_eq!(parse_total(CARD), Some(476));
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(
            unescape("Ramen &#x1F35C; &amp; co&#39;s &lt;3"),
            "Ramen 🍜 & co's <3"
        );
        assert_eq!(unescape("AT&T &unknown; & x"), "AT&T &unknown; & x");
    }

    #[test]
    fn builds_the_query_and_checks_ids() {
        let url = browse_url(MarketKind::Ball, " blossom ", MarketSort::Updated).unwrap();
        assert_eq!(
            url.as_str(),
            "https://alphaconsole.io/browse?textures=ball&sort=recently+updated&search=blossom"
        );
        assert!(valid_id("7b8e5d6f-38f0-40de-a72d-ee5694505798"));
        assert!(!valid_id("../../etc/passwd-aaaaaaaaaaaaaaaaaaaaaa"));
    }
}
