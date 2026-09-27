//! MMR of the players in your current match.
//!
//! The Stats API gives every player's `PrimaryId` (`Epic|<id>|0`,
//! `Steam|<steamid64>|0`, `PS4|…`, `XboxOne|…`, `Switch|…`). That is mapped to
//! a tracker.gg profile and looked up through the hidden browser. Pure
//! helpers live here; `client.rs` schedules the lookups.

use serde::{Deserialize, Serialize};

use crate::stats::mmr::PlaylistRating;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LobbyStatus {
    #[default]
    Pending,
    Ok,
    /// Bot, private profile, unknown platform or lookup failure.
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LobbyRating {
    /// `PrimaryId` from the Stats API (stable key for the match).
    pub key: String,
    pub name: String,
    pub team: i32,
    pub status: LobbyStatus,
    pub playlist: Option<String>,
    pub rating: Option<i64>,
    pub tier: Option<String>,
    pub division: Option<String>,
    pub tier_icon: Option<String>,
}

/// tracker.gg `(platform, identifier)` for a Stats API player.
/// Epic, PlayStation, Xbox and Switch profiles are addressed by display
/// name; Steam by its 64-bit id. Bots and splitscreen guests have none.
pub fn tracker_target(primary_id: &str, name: &str) -> Option<(&'static str, String)> {
    let mut parts = primary_id.split('|');
    let platform = parts.next()?.to_ascii_lowercase();
    let id = parts.next().unwrap_or_default();
    let name = name.trim();
    let by_name = |p: &'static str| (!name.is_empty()).then(|| (p, name.to_string()));
    match platform.as_str() {
        "epic" => by_name("epic"),
        "steam" => (id.len() >= 15 && id.bytes().all(|b| b.is_ascii_digit()))
            .then(|| ("steam", id.to_string())),
        "ps4" | "ps5" | "psn" | "playstation" => by_name("psn"),
        "xboxone" | "xbox" | "xbl" => by_name("xbl"),
        "switch" | "nintendo" => by_name("switch"),
        _ => None,
    }
}

/// Ranked playlist that matches the current team size.
pub fn ranked_playlist(team_size: usize) -> &'static str {
    match team_size {
        0 | 1 => "Ranked Duel 1v1",
        2 => "Ranked Doubles 2v2",
        4 => "Ranked 4v4 Quads",
        _ => "Ranked Standard 3v3",
    }
}

/// Fills a lobby entry from a profile, preferring the ranked playlist of the
/// current format, else the best rated ranked playlist.
pub fn rating_for(entry: &mut LobbyRating, playlists: &[PlaylistRating], team_size: usize) {
    let wanted = ranked_playlist(team_size);
    let pick = playlists
        .iter()
        .find(|p| p.playlist == wanted && p.rating.is_some())
        .or_else(|| {
            playlists
                .iter()
                .filter(|p| p.playlist.starts_with("Ranked") && p.rating.is_some())
                .max_by_key(|p| p.rating)
        });
    match pick {
        Some(p) => {
            entry.status = LobbyStatus::Ok;
            entry.playlist = Some(p.playlist.clone());
            entry.rating = p.rating;
            entry.tier = p.tier.clone();
            entry.division = p.division.clone();
            entry.tier_icon = p.tier_icon.clone();
        }
        None => entry.status = LobbyStatus::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_platforms_to_tracker_profiles() {
        assert_eq!(
            tracker_target("Epic|a1b2c3|0", "ALXS_RL"),
            Some(("epic", "ALXS_RL".into()))
        );
        assert_eq!(
            tracker_target("Steam|76561198000000000|0", "Bob"),
            Some(("steam", "76561198000000000".into()))
        );
        assert_eq!(
            tracker_target("PS4|123|0", "psGuy"),
            Some(("psn", "psGuy".into()))
        );
        assert_eq!(
            tracker_target("XboxOne|123|0", "xGuy"),
            Some(("xbl", "xGuy".into()))
        );
        assert_eq!(tracker_target("Unknown|0|0", "Bot Sundown"), None);
        assert_eq!(tracker_target("Steam|abc|0", "x"), None);
        assert_eq!(tracker_target("Epic|a1|0", "  "), None);
    }

    #[test]
    fn picks_the_ranked_playlist_of_the_format() {
        let lists = vec![
            PlaylistRating {
                playlist: "Ranked Doubles 2v2".into(),
                rating: Some(1661),
                ..Default::default()
            },
            PlaylistRating {
                playlist: "Ranked Standard 3v3".into(),
                rating: Some(1450),
                ..Default::default()
            },
            PlaylistRating {
                playlist: "Casual".into(),
                rating: Some(1801),
                ..Default::default()
            },
        ];
        let mut e = LobbyRating::default();
        rating_for(&mut e, &lists, 3);
        assert_eq!(e.rating, Some(1450));
        rating_for(&mut e, &lists, 1);
        assert_eq!(
            e.rating,
            Some(1661),
            "falls back to the best ranked playlist, never casual"
        );
        assert_eq!(e.status, LobbyStatus::Ok);
        rating_for(&mut e, &[], 2);
        assert_eq!(e.status, LobbyStatus::Unavailable);
    }
}
