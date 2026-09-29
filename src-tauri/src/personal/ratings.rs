//! MMR state of the personal edition: your MMR per playlist (with the
//! session delta) and the ratings of the players of the current match.
//! Pure state — no I/O, unit-tested; `service.rs` feeds and persists it.

use serde::{Deserialize, Serialize};

use crate::personal::lobby::{tracker_target, LobbyRating, LobbyStatus};
use crate::stats::tracker::PlayerLine;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MmrLine {
    pub playlist: String,
    /// First value seen this session (the delta starts here).
    pub baseline: i64,
    pub latest: i64,
    pub tier_icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RatingsState {
    pub mmr: Vec<MmrLine>,
    /// MMR of the players of the current (or last) match.
    pub lobby: Vec<LobbyRating>,
}

impl RatingsState {
    /// New session: the delta restarts from the latest known values.
    pub fn reset_baselines(&mut self) {
        for line in &mut self.mmr {
            line.baseline = line.latest;
        }
    }

    /// Merges a fresh lookup: the first value per playlist becomes the baseline.
    pub fn merge_mmr(&mut self, ratings: &[(String, i64, Option<String>)]) {
        for (playlist, rating, icon) in ratings {
            match self.mmr.iter_mut().find(|l| l.playlist == *playlist) {
                Some(line) => {
                    line.latest = *rating;
                    line.tier_icon = icon.clone().or(line.tier_icon.take());
                }
                None => self.mmr.push(MmrLine {
                    playlist: playlist.clone(),
                    baseline: *rating,
                    latest: *rating,
                    tier_icon: icon.clone(),
                }),
            }
        }
    }

    /// Aligns the lobby with the players of the live match and returns the
    /// entries that still need a lookup. Players who left are dropped;
    /// players without a tracker profile are marked unavailable at once.
    pub fn sync_lobby(&mut self, players: &[PlayerLine]) -> Vec<LobbyRating> {
        let current: Vec<&PlayerLine> = players.iter().filter(|p| p.primary_id.is_some()).collect();
        if current.is_empty() {
            return Vec::new();
        }
        let keys: Vec<&str> = current
            .iter()
            .filter_map(|p| p.primary_id.as_deref())
            .collect();
        self.lobby.retain(|e| keys.contains(&e.key.as_str()));
        let mut to_lookup = Vec::new();
        for p in current {
            let key = p.primary_id.clone().unwrap_or_default();
            if let Some(existing) = self.lobby.iter_mut().find(|e| e.key == key) {
                existing.team = p.team;
                continue;
            }
            let status = if tracker_target(&key, &p.name).is_some() {
                LobbyStatus::Pending
            } else {
                LobbyStatus::Unavailable
            };
            let entry = LobbyRating {
                key,
                name: p.name.clone(),
                team: p.team,
                status,
                ..Default::default()
            };
            if status == LobbyStatus::Pending {
                to_lookup.push(entry.clone());
            }
            self.lobby.push(entry);
        }
        to_lookup
    }

    /// Stores a finished lookup (ignored if the player already left).
    pub fn set_lobby_rating(&mut self, rating: LobbyRating) {
        if let Some(e) = self.lobby.iter_mut().find(|e| e.key == rating.key) {
            let team = e.team;
            *e = LobbyRating { team, ..rating };
        }
    }
}

/// Players per team (1 for 1v1, 2 for 2v2…).
pub fn team_size(players: &[PlayerLine]) -> usize {
    let blue = players.iter().filter(|p| p.team == 0).count();
    let orange = players.iter().filter(|p| p.team == 1).count();
    blue.max(orange).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmr_baseline_and_reset() {
        let mut r = RatingsState::default();
        r.merge_mmr(&[("Ranked Doubles 2v2".into(), 1000, None)]);
        r.merge_mmr(&[("Ranked Doubles 2v2".into(), 1012, Some("icon".into()))]);
        assert_eq!((r.mmr[0].baseline, r.mmr[0].latest), (1000, 1012));
        assert_eq!(r.mmr[0].tier_icon.as_deref(), Some("icon"));
        r.reset_baselines();
        assert_eq!(r.mmr[0].baseline, 1012);
    }

    #[test]
    fn lobby_follows_the_players_of_the_match() {
        let player = |name: &str, id: &str, team: i32| PlayerLine {
            name: name.into(),
            primary_id: Some(id.into()),
            team,
            ..Default::default()
        };
        let mut players = vec![
            player("A", "Epic|1|0", 0),
            player("B", "Steam|76561198000000001|0", 1),
            player("Bot", "Unknown|0|0", 1),
        ];
        let mut r = RatingsState::default();
        let todo = r.sync_lobby(&players);
        assert_eq!(todo.len(), 2);
        assert_eq!(r.lobby.len(), 3);
        assert!(r.sync_lobby(&players).is_empty(), "no duplicate lookups");
        r.set_lobby_rating(LobbyRating {
            key: "Epic|1|0".into(),
            status: LobbyStatus::Ok,
            rating: Some(1500),
            ..Default::default()
        });
        assert_eq!(r.lobby[0].rating, Some(1500));
        assert_eq!(r.lobby[0].team, 0, "team is kept from the match");
        players.retain(|p| p.name != "B");
        r.sync_lobby(&players);
        assert_eq!(r.lobby.len(), 2);
        assert_eq!(team_size(&players), 1);
    }
}
