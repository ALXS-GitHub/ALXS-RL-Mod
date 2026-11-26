//! Rocket Plugin match options: game mode, player count, mutators and the
//! user's Rocket Plugin presets, turned into `rp` console commands sent
//! before `rp host` (Rocket Plugin applies them when it hosts).
//!
//! The game modes and mutators come from Rocket Plugin's public config
//! (`stanbroek.github.io/RocketPlugin-config/static/RLConstants.json`,
//! 2026-02), trimmed into `rocket_plugin_options.json`. Presets are read
//! from `data/RocketPlugin/presets/*.cfg` and replayed as `rp mutator`
//! lines (not through `rp host <map> <preset>`, which depends on the user's
//! `rp_preset_path` cvar). Every category not chosen is reset to `Default`,
//! so nothing leaks from a previous session.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};
use crate::multi::bakkes_cfg;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub display: String,
    /// Value Rocket Plugin accepts (`""` = the game's default).
    pub internal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutatorCategory {
    pub category: String,
    pub label: String,
    pub values: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bundled {
    game_modes: Vec<Choice>,
    mutators: Vec<MutatorCategory>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpCatalog {
    pub game_modes: Vec<Choice>,
    pub mutators: Vec<MutatorCategory>,
    /// The user's Rocket Plugin presets (file names without `.cfg`).
    pub presets: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct MatchOptions {
    /// The host starts the match from Rocket Plugin's own window: the app
    /// installs the map, starts everything and gives the invite code, but
    /// sends no `rp` command.
    pub manual: bool,
    /// Internal game mode (`TAGame.GameInfo_Soccar_TA`); `None` = Rocket
    /// Plugin's current choice.
    pub game_mode: Option<String>,
    pub players: Option<u8>,
    /// One of [`RpCatalog::presets`].
    pub preset: Option<String>,
    /// Category → internal value, applied over the preset.
    pub mutators: BTreeMap<String, String>,
}

fn bundled() -> &'static Bundled {
    static BUNDLED: OnceLock<Bundled> = OnceLock::new();
    BUNDLED.get_or_init(|| {
        // Invariant: the file is checked in and covered by a test.
        serde_json::from_str(include_str!("rocket_plugin_options.json"))
            .expect("rocket_plugin_options.json is valid")
    })
}

fn presets_dir() -> Option<PathBuf> {
    bakkes_cfg::data_dir()
        .map(|d| d.join("data").join("RocketPlugin").join("presets"))
        .filter(|d| d.is_dir())
}

/// Preset names are file stems shown in the UI and used in commands: plain
/// characters only (no quote, `;` or path separator).
fn safe_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.' | '(' | ')'))
}

pub fn list_presets() -> Vec<String> {
    let Some(dir) = presets_dir() else {
        return Vec::new();
    };
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("cfg")))
                .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .filter(|s| safe_token(s))
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|s| s.to_lowercase());
    out
}

pub fn catalog() -> RpCatalog {
    let b = bundled();
    RpCatalog {
        game_modes: b.game_modes.clone(),
        mutators: b.mutators.clone(),
        presets: list_presets(),
    }
}

/// `rp mutator "Cat" "Value"` lines of a preset file.
pub(crate) fn parse_preset(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("rp mutator")?;
            let parts: Vec<&str> = rest.split('"').collect();
            // ` "Cat" "Value"` → ["", "Cat", " ", "Value", ""]
            match parts.as_slice() {
                [_, cat, _, value, ..] if !cat.is_empty() => {
                    Some((cat.to_string(), value.to_string()))
                }
                _ => None,
            }
        })
        .collect()
}

/// Mutator names and values reach the console: identifiers only.
fn safe_ident(s: &str) -> bool {
    s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn bad(msg: String) -> AppError {
    AppError::InvalidInput(format!("match options: {msg}"))
}

/// Console commands for `options`, `rp host <arena>` last. Empty in manual
/// mode. Everything is validated against the bundled lists / preset files.
pub fn host_commands(
    options: &MatchOptions,
    level: &str,
    preset_text: impl Fn(&str) -> Option<String>,
) -> AppResult<Vec<String>> {
    if options.manual {
        return Ok(Vec::new());
    }
    let b = bundled();
    let mut commands = Vec::new();
    if let Some(mode) = &options.game_mode {
        if !b.game_modes.iter().any(|m| &m.internal == mode) {
            return Err(bad(format!("unknown game mode {mode}")));
        }
        commands.push(format!("rp gamemode \"{mode}\""));
    }
    if let Some(n) = options.players {
        if !(2..=8).contains(&n) {
            return Err(bad(format!("players must be 2 to 8, not {n}")));
        }
        commands.push(format!("rp players {n}"));
    }

    // Preset first, explicit choices over it; every other category = Default.
    let mut chosen: BTreeMap<String, String> = BTreeMap::new();
    if let Some(name) = &options.preset {
        if !safe_token(name) {
            return Err(bad(format!("bad preset name {name}")));
        }
        let text = preset_text(name).ok_or_else(|| bad(format!("preset {name} not found")))?;
        for (cat, value) in parse_preset(&text) {
            if safe_ident(&cat) && safe_ident(&value) {
                chosen.insert(cat, value);
            }
        }
    }
    for (cat, value) in &options.mutators {
        let Some(known) = b.mutators.iter().find(|m| &m.category == cat) else {
            return Err(bad(format!("unknown mutator {cat}")));
        };
        if !known.values.iter().any(|v| &v.internal == value) {
            return Err(bad(format!("unknown value {value} for {cat}")));
        }
        chosen.insert(cat.clone(), value.clone());
    }
    for m in &b.mutators {
        let value = chosen
            .remove(&m.category)
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "Default".into());
        commands.push(format!("rp mutator \"{}\" \"{value}\"", m.category));
    }
    // Preset categories Rocket Plugin knows but the bundled list does not.
    for (cat, value) in chosen {
        if !value.is_empty() {
            commands.push(format!("rp mutator \"{cat}\" \"{value}\""));
        }
    }
    commands.push(format!("rp host {level}"));
    Ok(commands)
}

/// Reads a preset of the user's Rocket Plugin presets folder.
pub fn read_preset(name: &str) -> Option<String> {
    if !safe_token(name) {
        return None;
    }
    std::fs::read_to_string(presets_dir()?.join(format!("{name}.cfg"))).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_lists_load() {
        let b = bundled();
        assert!(b
            .game_modes
            .iter()
            .any(|m| m.internal == "TAGame.GameInfo_Soccar_TA"));
        assert!(b.mutators.len() > 30);
        assert!(b
            .mutators
            .iter()
            .all(|m| m.values.first().is_some_and(|v| v.internal.is_empty())));
    }

    #[test]
    fn preset_lines_parse() {
        let text = "// comment\r\nrp mutator \"BallType\" \"Ball_CubeBall\"\r\nrp mutator \"Boosters\" \"SlowRecharge\"\nrp players 4\n";
        assert_eq!(
            parse_preset(text),
            vec![
                ("BallType".to_string(), "Ball_CubeBall".to_string()),
                ("Boosters".to_string(), "SlowRecharge".to_string())
            ]
        );
    }

    #[test]
    fn commands_reset_everything_and_host_last() {
        let mut opts = MatchOptions {
            game_mode: Some("TAGame.GameInfo_Basketball_TA".into()),
            players: Some(4),
            preset: Some("Cubic".into()),
            ..Default::default()
        };
        opts.mutators
            .insert("Boosters".into(), "UnlimitedBooster".into());
        let preset = |n: &str| {
            (n == "Cubic").then(|| {
                "rp mutator \"BallType\" \"Ball_CubeBall\"\nrp mutator \"Boosters\" \"SlowRecharge\"\nrp mutator \"BoostOption\" \"UnlimitedBooster\"".to_string()
            })
        };
        let cmds = host_commands(&opts, "Labs_Underpass_P", preset).unwrap();
        assert_eq!(cmds[0], "rp gamemode \"TAGame.GameInfo_Basketball_TA\"");
        assert_eq!(cmds[1], "rp players 4");
        assert!(cmds.contains(&"rp mutator \"BallType\" \"Ball_CubeBall\"".to_string()));
        // Explicit choice wins over the preset.
        assert!(cmds.contains(&"rp mutator \"Boosters\" \"UnlimitedBooster\"".to_string()));
        assert!(cmds.contains(&"rp mutator \"GameTimes\" \"Default\"".to_string()));
        // Preset category unknown to the bundled list is kept.
        assert!(cmds.contains(&"rp mutator \"BoostOption\" \"UnlimitedBooster\"".to_string()));
        assert_eq!(cmds.last().unwrap(), "rp host Labs_Underpass_P");
    }

    #[test]
    fn manual_and_invalid_options() {
        let manual = MatchOptions {
            manual: true,
            ..Default::default()
        };
        assert!(host_commands(&manual, "Labs_Underpass_P", |_| None)
            .unwrap()
            .is_empty());

        let bad_mode = MatchOptions {
            game_mode: Some("TAGame.GameInfo_Soccar_TA\"; quit".into()),
            ..Default::default()
        };
        assert!(host_commands(&bad_mode, "L", |_| None).is_err());
        let mut bad_mut = MatchOptions::default();
        bad_mut.mutators.insert("Boosters".into(), "Lots".into());
        assert!(host_commands(&bad_mut, "L", |_| None).is_err());
        let bad_preset = MatchOptions {
            preset: Some("../x\";quit".into()),
            ..Default::default()
        };
        assert!(host_commands(&bad_preset, "L", |_| None).is_err());
        // A preset line that is not a plain identifier is dropped.
        let sneaky = MatchOptions {
            preset: Some("P".into()),
            ..Default::default()
        };
        let cmds = host_commands(&sneaky, "L", |_| {
            Some("rp mutator \"Boosters\" \"x;quit\"".into())
        })
        .unwrap();
        assert!(cmds.iter().all(|c| !c.contains("quit")));
        let players = MatchOptions {
            players: Some(9),
            ..Default::default()
        };
        assert!(host_commands(&players, "L", |_| None).is_err());
    }
}
