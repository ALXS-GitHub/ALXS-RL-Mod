//! Session tracker: turns Stats API messages into a gaming session
//! (live match, W/L, streak, totals, match history). Pure state machine —
//! no I/O, fully unit-tested; `client.rs` feeds it and persists it.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::stats::protocol::{Player, StatsMessage, UpdateState};

const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MatchResult {
    Win,
    Loss,
    /// Left early, or the winner could not be determined.
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerLine {
    pub name: String,
    /// Stats API `PrimaryId` (`Epic|…|0`, `Steam|…|0`…).
    #[serde(default)]
    pub primary_id: Option<String>,
    pub team: i32,
    pub score: i32,
    pub goals: i32,
    pub assists: i32,
    pub saves: i32,
    pub shots: i32,
    pub demos: i32,
    pub touches: i32,
}

impl From<&Player> for PlayerLine {
    fn from(p: &Player) -> Self {
        Self {
            name: p.name.clone(),
            primary_id: p.primary_id.clone(),
            team: p.team_num,
            score: p.score,
            goals: p.goals,
            assists: p.assists,
            saves: p.saves,
            shots: p.shots,
            demos: p.demos,
            touches: p.touches,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LiveMatch {
    pub guid: Option<String>,
    pub arena: Option<String>,
    pub time_seconds: Option<f64>,
    pub overtime: bool,
    pub replay: bool,
    pub blue_score: i32,
    pub orange_score: i32,
    pub your_team: Option<i32>,
    pub you: Option<PlayerLine>,
    pub players: Vec<PlayerLine>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended: bool,
    /// How often each player was the camera target outside replays.
    #[serde(skip)]
    target_counts: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchRecord {
    pub guid: Option<String>,
    pub arena: Option<String>,
    pub ended_at: DateTime<Utc>,
    pub result: MatchResult,
    pub your_team: Option<i32>,
    pub blue_score: i32,
    pub orange_score: i32,
    pub overtime: bool,
    pub you: Option<PlayerLine>,
    /// Highest score of the winning team.
    pub mvp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub goals: i32,
    pub assists: i32,
    pub saves: i32,
    pub shots: i32,
    pub demos: i32,
    pub score: i32,
    pub mvps: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Connection {
    /// Game not running.
    #[default]
    Idle,
    Connecting,
    Connected,
    /// Game running but the API does not answer (disabled or game not restarted).
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionState {
    pub started_at: DateTime<Utc>,
    pub wins: u32,
    pub losses: u32,
    /// +n = n wins in a row, -n = n losses in a row.
    pub streak: i32,
    pub best_streak: i32,
    pub totals: Totals,
    pub matches: Vec<MatchRecord>,
    pub live: Option<LiveMatch>,
    pub connection: Connection,
    /// Player name we attribute "you" to.
    pub you_name: Option<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            started_at: Utc::now(),
            wins: 0,
            losses: 0,
            streak: 0,
            best_streak: 0,
            totals: Totals::default(),
            matches: Vec::new(),
            live: None,
            connection: Connection::Idle,
            you_name: None,
        }
    }
}

/// Which way `apply` changed the state (drives persistence and the hooks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    None,
    Live,
    MatchRecorded,
}

pub struct Tracker {
    pub state: SessionState,
    /// Configured player (tracker id or display name) used to find "you".
    pub hint: Option<String>,
}

impl Tracker {
    pub fn new(state: SessionState, hint: Option<String>) -> Self {
        Self { state, hint }
    }

    pub fn reset(&mut self) {
        let connection = self.state.connection;
        self.state = SessionState {
            connection,
            ..Default::default()
        };
    }

    fn resolve_you<'a>(&self, live: &LiveMatch, players: &'a [Player]) -> Option<&'a Player> {
        if let Some(hint) = self
            .hint
            .as_deref()
            .map(str::trim)
            .filter(|h| !h.is_empty())
        {
            let h = hint.to_lowercase();
            let by_hint = players.iter().find(|p| {
                p.name.to_lowercase() == h
                    || p.primary_id
                        .as_deref()
                        .is_some_and(|id| id.to_lowercase().contains(&h))
            });
            if by_hint.is_some() {
                return by_hint;
            }
        }
        let best = live
            .target_counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(name, _)| name.clone())?;
        players.iter().find(|p| p.name == best)
    }

    fn on_update(&mut self, u: &UpdateState) -> Change {
        let same_match =
            self.state.live.as_ref().is_some_and(|l| {
                l.guid.is_none() || u.match_guid.is_none() || l.guid == u.match_guid
            });
        if !same_match || self.state.live.is_none() {
            self.state.live = Some(LiveMatch {
                guid: u.match_guid.clone(),
                started_at: Some(Utc::now()),
                ..Default::default()
            });
        }
        let mut live = self.state.live.take().unwrap_or_default();
        if live.guid.is_none() {
            live.guid = u.match_guid.clone();
        }
        let g = &u.game;
        live.arena = g.arena.clone().or(live.arena);
        live.time_seconds = g.time_seconds;
        live.overtime = g.overtime;
        live.replay = g.replay;
        for t in &g.teams {
            match t.team_num {
                0 => live.blue_score = t.score,
                1 => live.orange_score = t.score,
                _ => {}
            }
        }
        if g.has_target && !g.replay {
            if let Some(target) = g.target.as_ref().filter(|t| !t.name.is_empty()) {
                *live.target_counts.entry(target.name.clone()).or_default() += 1;
            }
        }
        live.players = u.players.iter().map(PlayerLine::from).collect();
        if let Some(me) = self.resolve_you(&live, &u.players) {
            live.your_team = Some(me.team_num);
            live.you = Some(PlayerLine::from(me));
            self.state.you_name = Some(me.name.clone());
        }
        self.state.live = Some(live);
        Change::Live
    }

    /// Closes the live match into a record. `winner` = team number.
    fn finish(&mut self, winner: Option<i32>) -> Change {
        let Some(live) = self.state.live.as_mut() else {
            return Change::None;
        };
        if live.ended {
            return Change::None;
        }
        live.ended = true;
        let live = live.clone();
        // Training / free play: no opponent, nothing to record.
        let teams_present =
            live.players.iter().any(|p| p.team == 0) && live.players.iter().any(|p| p.team == 1);
        if !teams_present {
            return Change::Live;
        }
        let winner = winner.or(match live.blue_score.cmp(&live.orange_score) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => None,
        });
        let result = match (winner, live.your_team) {
            (Some(w), Some(t)) if w == t => MatchResult::Win,
            (Some(_), Some(_)) => MatchResult::Loss,
            _ => MatchResult::Unknown,
        };
        let mvp = match (winner, live.you.as_ref()) {
            (Some(w), Some(me)) if me.team == w => {
                let top = live
                    .players
                    .iter()
                    .filter(|p| p.team == w)
                    .map(|p| p.score)
                    .max()
                    .unwrap_or(0);
                me.score >= top && me.score > 0
            }
            _ => false,
        };
        self.record(MatchRecord {
            guid: live.guid.clone(),
            arena: live.arena.clone(),
            ended_at: Utc::now(),
            result,
            your_team: live.your_team,
            blue_score: live.blue_score,
            orange_score: live.orange_score,
            overtime: live.overtime,
            you: live.you.clone(),
            mvp,
        });
        Change::MatchRecorded
    }

    fn record(&mut self, rec: MatchRecord) {
        let s = &mut self.state;
        match rec.result {
            MatchResult::Win => {
                s.wins += 1;
                s.streak = if s.streak >= 0 { s.streak + 1 } else { 1 };
            }
            MatchResult::Loss => {
                s.losses += 1;
                s.streak = if s.streak <= 0 { s.streak - 1 } else { -1 };
            }
            MatchResult::Unknown => {}
        }
        s.best_streak = s.best_streak.max(s.streak);
        if let Some(me) = &rec.you {
            s.totals.goals += me.goals;
            s.totals.assists += me.assists;
            s.totals.saves += me.saves;
            s.totals.shots += me.shots;
            s.totals.demos += me.demos;
            s.totals.score += me.score;
        }
        if rec.mvp {
            s.totals.mvps += 1;
        }
        s.matches.insert(0, rec);
        s.matches.truncate(MAX_HISTORY);
    }

    pub fn apply(&mut self, msg: &StatsMessage) -> Change {
        match msg {
            StatsMessage::UpdateState(u) => self.on_update(u),
            StatsMessage::MatchCreated { guid } => {
                self.state.live = Some(LiveMatch {
                    guid: guid.clone(),
                    started_at: Some(Utc::now()),
                    ..Default::default()
                });
                Change::Live
            }
            StatsMessage::MatchEnded { winner_team, .. } => self.finish(*winner_team),
            StatsMessage::MatchDestroyed { .. } => {
                // Left before the end: record it without a result.
                let change = match self.state.live.as_ref() {
                    Some(l)
                        if !l.ended
                            && (l.blue_score + l.orange_score > 0 || l.time_seconds.is_some()) =>
                    {
                        self.finish_abandoned()
                    }
                    _ => Change::Live,
                };
                self.state.live = None;
                change
            }
            StatsMessage::GoalScored(_) | StatsMessage::Other(_) => Change::None,
        }
    }

    fn finish_abandoned(&mut self) -> Change {
        let Some(live) = self.state.live.clone() else {
            return Change::None;
        };
        let teams_present =
            live.players.iter().any(|p| p.team == 0) && live.players.iter().any(|p| p.team == 1);
        if !teams_present {
            return Change::Live;
        }
        self.record(MatchRecord {
            guid: live.guid,
            arena: live.arena,
            ended_at: Utc::now(),
            result: MatchResult::Unknown,
            your_team: live.your_team,
            blue_score: live.blue_score,
            orange_score: live.orange_score,
            overtime: live.overtime,
            you: live.you,
            mvp: false,
        });
        Change::MatchRecorded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::protocol::{Game, PlayerRef, Team};

    fn player(name: &str, team: i32, score: i32, goals: i32) -> Player {
        Player {
            name: name.into(),
            team_num: team,
            score,
            goals,
            ..Default::default()
        }
    }

    fn update(
        guid: &str,
        blue: i32,
        orange: i32,
        target: &str,
        players: Vec<Player>,
    ) -> StatsMessage {
        StatsMessage::UpdateState(Box::new(UpdateState {
            match_guid: Some(guid.into()),
            players,
            game: Game {
                teams: vec![
                    Team {
                        team_num: 0,
                        score: blue,
                        name: None,
                    },
                    Team {
                        team_num: 1,
                        score: orange,
                        name: None,
                    },
                ],
                time_seconds: Some(100.0),
                has_target: true,
                target: Some(PlayerRef {
                    name: target.into(),
                    ..Default::default()
                }),
                arena: Some("Stadium_P".into()),
                ..Default::default()
            },
        }))
    }

    fn play(
        t: &mut Tracker,
        guid: &str,
        me_team: i32,
        blue: i32,
        orange: i32,
        winner: Option<i32>,
    ) -> Change {
        let players = vec![
            player("Me", me_team, 500, 2),
            player("Mate", me_team, 300, 1),
            player("Opp", 1 - me_team, 200, 0),
        ];
        t.apply(&StatsMessage::MatchCreated {
            guid: Some(guid.into()),
        });
        t.apply(&update(guid, blue, orange, "Me", players));
        t.apply(&StatsMessage::MatchEnded {
            guid: Some(guid.into()),
            winner_team: winner,
        })
    }

    #[test]
    fn wins_losses_and_streaks() {
        let mut t = Tracker::new(SessionState::default(), None);
        assert_eq!(play(&mut t, "a", 0, 3, 1, Some(0)), Change::MatchRecorded);
        play(&mut t, "b", 0, 2, 0, Some(0));
        assert_eq!(
            (t.state.wins, t.state.streak, t.state.best_streak),
            (2, 2, 2)
        );
        play(&mut t, "c", 1, 4, 1, Some(0));
        assert_eq!((t.state.losses, t.state.streak), (1, -1));
        play(&mut t, "d", 0, 0, 1, None); // winner inferred from score
        assert_eq!(t.state.streak, -2);
        assert_eq!(t.state.matches.len(), 4);
        assert_eq!(t.state.matches[0].result, MatchResult::Loss);
        assert_eq!(t.state.totals.goals, 8);
        assert_eq!(t.state.you_name.as_deref(), Some("Me"));
    }

    #[test]
    fn mvp_only_for_top_scorer_of_winners() {
        let mut t = Tracker::new(SessionState::default(), None);
        play(&mut t, "a", 0, 3, 1, Some(0));
        assert!(t.state.matches[0].mvp);
        assert_eq!(t.state.totals.mvps, 1);
        play(&mut t, "b", 1, 3, 1, Some(0));
        assert!(!t.state.matches[0].mvp);
    }

    #[test]
    fn ended_is_counted_once_and_training_is_ignored() {
        let mut t = Tracker::new(SessionState::default(), None);
        play(&mut t, "a", 0, 1, 0, Some(0));
        assert_eq!(
            t.apply(&StatsMessage::MatchEnded {
                guid: Some("a".into()),
                winner_team: Some(0)
            }),
            Change::None
        );
        assert_eq!(t.state.wins, 1);
        // Free play: only one team.
        t.apply(&StatsMessage::MatchCreated {
            guid: Some("fp".into()),
        });
        t.apply(&update("fp", 0, 0, "Me", vec![player("Me", 0, 0, 0)]));
        assert_eq!(
            t.apply(&StatsMessage::MatchEnded {
                guid: None,
                winner_team: Some(0)
            }),
            Change::Live
        );
        assert_eq!(t.state.matches.len(), 1);
    }

    #[test]
    fn abandoned_match_is_recorded_without_result() {
        let mut t = Tracker::new(SessionState::default(), None);
        t.apply(&StatsMessage::MatchCreated {
            guid: Some("x".into()),
        });
        t.apply(&update(
            "x",
            0,
            2,
            "Me",
            vec![player("Me", 0, 10, 0), player("Opp", 1, 50, 2)],
        ));
        assert_eq!(
            t.apply(&StatsMessage::MatchDestroyed {
                guid: Some("x".into())
            }),
            Change::MatchRecorded
        );
        assert_eq!(t.state.matches[0].result, MatchResult::Unknown);
        assert_eq!((t.state.wins, t.state.losses, t.state.streak), (0, 0, 0));
        assert!(t.state.live.is_none());
    }

    #[test]
    fn hint_beats_camera_target() {
        let mut t = Tracker::new(SessionState::default(), Some("mate".into()));
        play(&mut t, "a", 0, 1, 0, Some(0));
        assert_eq!(t.state.matches[0].you.as_ref().unwrap().name, "Mate");
    }
}
