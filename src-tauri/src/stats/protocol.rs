//! Messages of the official Rocket League Stats API (WebSocket, JSON).
//!
//! Every message is `{ "Event": "<Name>", "Data": <object | JSON string> }`.
//! Field names follow the game's PascalCase; everything is optional so a
//! game update adding/removing fields never breaks parsing.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct PlayerRef {
    pub name: String,
    pub shortcut: Option<i64>,
    pub team_num: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Player {
    pub name: String,
    pub primary_id: Option<String>,
    pub shortcut: Option<i64>,
    pub team_num: i32,
    pub score: i32,
    pub goals: i32,
    pub shots: i32,
    pub assists: i32,
    pub saves: i32,
    pub touches: i32,
    pub demos: i32,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Team {
    pub name: Option<String>,
    pub team_num: i32,
    pub score: i32,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Game {
    #[serde(rename = "Teams")]
    pub teams: Vec<Team>,
    #[serde(rename = "TimeSeconds")]
    pub time_seconds: Option<f64>,
    #[serde(rename = "bOvertime")]
    pub overtime: bool,
    #[serde(rename = "bReplay")]
    pub replay: bool,
    #[serde(rename = "bHasWinner")]
    pub has_winner: bool,
    #[serde(rename = "Arena")]
    pub arena: Option<String>,
    #[serde(rename = "bHasTarget")]
    pub has_target: bool,
    #[serde(rename = "Target")]
    pub target: Option<PlayerRef>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct UpdateState {
    pub match_guid: Option<String>,
    pub players: Vec<Player>,
    pub game: Game,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GoalScored {
    pub match_guid: Option<String>,
    pub goal_speed: Option<f64>,
    pub goal_time: Option<f64>,
    pub scorer: PlayerRef,
    pub assister: Option<PlayerRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatsMessage {
    UpdateState(Box<UpdateState>),
    GoalScored(GoalScored),
    MatchCreated {
        guid: Option<String>,
    },
    MatchEnded {
        guid: Option<String>,
        winner_team: Option<i32>,
    },
    MatchDestroyed {
        guid: Option<String>,
    },
    /// Anything else (BallHit, StatfeedEvent, ClockUpdatedSeconds…).
    Other(String),
}

impl StatsMessage {
    pub fn name(&self) -> &str {
        match self {
            StatsMessage::UpdateState(_) => "UpdateState",
            StatsMessage::GoalScored(_) => "GoalScored",
            StatsMessage::MatchCreated { .. } => "MatchCreated",
            StatsMessage::MatchEnded { .. } => "MatchEnded",
            StatsMessage::MatchDestroyed { .. } => "MatchDestroyed",
            StatsMessage::Other(n) => n,
        }
    }
}

#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "Event")]
    event: String,
    #[serde(rename = "Data", default)]
    data: serde_json::Value,
}

fn guid_of(data: &serde_json::Value) -> Option<String> {
    data.get("MatchGuid")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Parses one text frame. Returns the event name even when its payload is
/// unexpected, so the raw feed stays useful.
pub fn parse(text: &str) -> Option<(StatsMessage, serde_json::Value)> {
    let env: Envelope = serde_json::from_str(text).ok()?;
    // Some builds double-encode `Data` as a JSON string.
    let data = match env.data {
        serde_json::Value::String(s) => serde_json::from_str(&s).unwrap_or(serde_json::Value::Null),
        other => other,
    };
    let msg = match env.event.as_str() {
        "UpdateState" => match serde_json::from_value::<UpdateState>(data.clone()) {
            Ok(u) => StatsMessage::UpdateState(Box::new(u)),
            Err(_) => StatsMessage::Other(env.event.clone()),
        },
        "GoalScored" => match serde_json::from_value::<GoalScored>(data.clone()) {
            Ok(g) => StatsMessage::GoalScored(g),
            Err(_) => StatsMessage::Other(env.event.clone()),
        },
        "MatchCreated" | "MatchInitialized" => StatsMessage::MatchCreated {
            guid: guid_of(&data),
        },
        "MatchEnded" => StatsMessage::MatchEnded {
            guid: guid_of(&data),
            winner_team: data
                .get("WinnerTeamNum")
                .and_then(|v| v.as_i64())
                .and_then(|n| i32::try_from(n).ok()),
        },
        "MatchDestroyed" => StatsMessage::MatchDestroyed {
            guid: guid_of(&data),
        },
        other => StatsMessage::Other(other.to_string()),
    };
    Some((msg, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_update_state_object() {
        let raw = r#"{"Event":"UpdateState","Data":{"MatchGuid":"ABC","Players":[
            {"Name":"Me","PrimaryId":"Epic|123|0","Shortcut":1,"TeamNum":0,"Score":320,"Goals":2,"Shots":4,"Assists":1,"Saves":3,"Touches":40,"Demos":1,"Speed":1200}],
            "Game":{"Teams":[{"Name":"Blue","TeamNum":0,"Score":2},{"Name":"Orange","TeamNum":1,"Score":1}],
            "TimeSeconds":120,"bOvertime":false,"bReplay":false,"bHasWinner":false,"Arena":"Stadium_P","bHasTarget":true,
            "Target":{"Name":"Me","Shortcut":1,"TeamNum":0}}}}"#;
        let (msg, _) = parse(raw).unwrap();
        let StatsMessage::UpdateState(u) = msg else {
            panic!("wrong kind")
        };
        assert_eq!(u.match_guid.as_deref(), Some("ABC"));
        assert_eq!(u.players[0].goals, 2);
        assert_eq!(u.game.teams[1].score, 1);
        assert_eq!(u.game.target.as_ref().unwrap().name, "Me");
        assert_eq!(u.game.arena.as_deref(), Some("Stadium_P"));
    }

    #[test]
    fn parses_double_encoded_data() {
        let raw = r#"{"Event":"MatchEnded","Data":"{\"MatchGuid\":\"G1\",\"WinnerTeamNum\":1}"}"#;
        let (msg, _) = parse(raw).unwrap();
        assert_eq!(
            msg,
            StatsMessage::MatchEnded {
                guid: Some("G1".into()),
                winner_team: Some(1)
            }
        );
    }

    #[test]
    fn unknown_events_and_garbage() {
        let (msg, _) = parse(r#"{"Event":"BallHit","Data":{}}"#).unwrap();
        assert_eq!(msg.name(), "BallHit");
        assert!(parse("not json").is_none());
        let (msg, _) = parse(r#"{"Event":"UpdateState","Data":{"Players":"nope"}}"#).unwrap();
        assert_eq!(msg, StatsMessage::Other("UpdateState".into()));
    }
}
