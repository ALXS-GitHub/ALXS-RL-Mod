//! Public MMR lookup on tracker.gg (read-only, no login).
//!
//! tracker.gg sits behind Cloudflare, which rejects plain HTTP clients, so
//! the lookup runs in a hidden real browser (`stats::browser`) that opens the
//! public profile page like the user would. No protection is bypassed.

use serde::Serialize;
use tauri::AppHandle;

use crate::base::error::{AppError, AppResult};
use crate::base::security::{ensure_allowed, Purpose};
use crate::stats::browser;

const API: &str = "https://api.tracker.gg/api/v2/rocket-league/standard/profile";
const PAGE: &str = "https://rocketleague.tracker.network/rocket-league/profile";
pub const PLATFORMS: &[&str] = &["epic", "steam", "psn", "xbl", "switch"];

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistRating {
    pub playlist: String,
    pub rating: Option<i64>,
    pub tier: Option<String>,
    pub division: Option<String>,
    pub tier_icon: Option<String>,
    pub matches_played: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MmrProfile {
    pub platform: String,
    pub player: String,
    pub display_name: Option<String>,
    pub playlists: Vec<PlaylistRating>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// Extracts playlist ratings from a tracker.gg profile payload. Pure.
pub fn parse_profile(value: &serde_json::Value) -> (Option<String>, Vec<PlaylistRating>) {
    let data = value.get("data").unwrap_or(value);
    let display = data
        .pointer("/platformInfo/platformUserHandle")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let str_at =
        |v: &serde_json::Value, p: &str| v.pointer(p).and_then(|x| x.as_str()).map(str::to_string);
    let int_at = |v: &serde_json::Value, p: &str| {
        v.pointer(p)
            .and_then(|x| x.as_f64())
            .map(|f| f.round() as i64)
    };
    let playlists = data
        .get("segments")
        .and_then(|s| s.as_array())
        .map(|segs| {
            segs.iter()
                .filter(|s| s.get("type").and_then(|t| t.as_str()) == Some("playlist"))
                .filter_map(|s| {
                    Some(PlaylistRating {
                        playlist: str_at(s, "/metadata/name")?,
                        rating: int_at(s, "/stats/rating/value"),
                        tier: str_at(s, "/stats/tier/metadata/name"),
                        division: str_at(s, "/stats/division/metadata/name"),
                        tier_icon: str_at(s, "/stats/tier/metadata/iconUrl")
                            .filter(|u| u.starts_with("https://trackercdn.com/")),
                        matches_played: int_at(s, "/stats/matchesPlayed/value"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (display, playlists)
}

pub async fn lookup(app: &AppHandle, platform: &str, player: &str) -> AppResult<MmrProfile> {
    if !crate::base::config::current(app).tracker.online_ratings {
        return Err(AppError::RatingsDisabled);
    }
    let platform = platform.trim().to_ascii_lowercase();
    if !PLATFORMS.contains(&platform.as_str()) {
        return Err(AppError::InvalidInput(format!(
            "unknown platform {platform}"
        )));
    }
    let player = player.trim();
    if player.is_empty() || player.len() > 64 {
        return Err(AppError::InvalidInput("player id".into()));
    }
    let id = encode(player);
    let api = ensure_allowed(&format!("{API}/{platform}/{id}"), Purpose::Ratings)?;
    let page = ensure_allowed(
        &format!("{PAGE}/{platform}/{id}/overview"),
        Purpose::Ratings,
    )?;
    let value = browser::fetch_profile(app, &page, &api)
        .await
        .map_err(|e| match e {
            AppError::NotFound(_) => AppError::NotFound(format!("player {player} on {platform}")),
            other => other,
        })?;
    let (display_name, playlists) = parse_profile(&value);
    Ok(MmrProfile {
        platform,
        player: player.to_string(),
        display_name,
        playlists,
        fetched_at: chrono::Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_playlists() {
        let v = serde_json::json!({ "data": {
        "platformInfo": { "platformUserHandle": "ALXS" },
        "segments": [
            { "type": "overview", "metadata": { "name": "Lifetime" } },
            { "type": "playlist", "metadata": { "name": "Ranked Doubles 2v2" }, "stats": {
                "rating": { "value": 1134 },
                "tier": { "metadata": { "name": "Champion II", "iconUrl": "https://trackercdn.com/cdn/tracker.gg/rocket-league/ranks/s4-17.png" } },
                "division": { "metadata": { "name": "Division III" } },
                "matchesPlayed": { "value": 52 } } },
            { "type": "playlist", "metadata": { "name": "Hoops" }, "stats": {
                "tier": { "metadata": { "iconUrl": "https://evil.example/x.png" } } } }
        ]}});
        let (name, lists) = parse_profile(&v);
        assert_eq!(name.as_deref(), Some("ALXS"));
        assert_eq!(lists.len(), 2);
        assert_eq!(lists[0].rating, Some(1134));
        assert_eq!(lists[0].tier.as_deref(), Some("Champion II"));
        assert!(lists[0].tier_icon.is_some());
        assert!(lists[1].tier_icon.is_none());
        assert_eq!(lists[1].rating, None);
    }

    #[test]
    fn encodes_ids() {
        assert_eq!(encode("my name#1"), "my%20name%231");
    }
}
