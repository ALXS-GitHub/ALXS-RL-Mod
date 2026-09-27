//! Stats API switch.
//!
//! The game ships `TAGame/Config/DefaultStatsAPI.ini` with
//! `PacketSendRate=0` (disabled), `Port=49123` (raw TCP) and
//! `WebPort=49124` (WebSocket). EAC verifies the files of the install's
//! `Config` folder, so we never touch it: the per-user override
//! `Documents\My Games\Rocket League\TAGame\Config\TAStatsAPI.ini` wins over
//! the default and is what we edit. The original user file is kept in
//! `<data>/stats/` and put back on disable.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};

pub const SECTION: &str = "TAGame.MatchStatsExporter_TA";
pub const DEFAULT_WEB_PORT: u16 = 49124;
pub const PACKET_SEND_RATE: u32 = 30;
const INI_NAME: &str = "TAStatsAPI.ini";

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IniSettings {
    pub port: Option<u16>,
    pub web_port: Option<u16>,
    pub packet_send_rate: Option<f32>,
}

impl IniSettings {
    pub fn enabled(&self) -> bool {
        self.packet_send_rate.is_some_and(|r| r > 0.0) && self.web_port.is_some_and(|p| p > 0)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct IniState {
    /// We enabled it (so disable restores the saved original).
    enabled_by_us: bool,
    /// The user file did not exist before us.
    created_by_us: bool,
}

fn is_section(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .map(str::trim)
}

fn key_value(line: &str) -> Option<(&str, &str)> {
    let t = line.trim();
    if t.starts_with(';') || t.starts_with('#') {
        return None;
    }
    t.split_once('=').map(|(k, v)| (k.trim(), v.trim()))
}

/// Reads the exporter section. Pure.
pub fn parse(content: &str) -> IniSettings {
    let mut s = IniSettings::default();
    let mut in_section = false;
    for line in content.lines() {
        if let Some(name) = is_section(line) {
            in_section = name.eq_ignore_ascii_case(SECTION);
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((k, v)) = key_value(line) {
            match k.to_ascii_lowercase().as_str() {
                "port" => s.port = v.parse().ok(),
                "webport" => s.web_port = v.parse().ok(),
                "packetsendrate" => s.packet_send_rate = v.parse().ok(),
                _ => {}
            }
        }
    }
    s
}

/// Sets `keys` in the exporter section, adding the section/keys if missing
/// and keeping every other line (comments, `[IniVersion]`…) intact. Pure.
pub fn upsert(content: &str, keys: &[(&str, String)]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_section = false;
    let mut section_seen = false;
    let mut written = vec![false; keys.len()];
    let flush_missing = |out: &mut Vec<String>, written: &mut [bool]| {
        for (i, (k, v)) in keys.iter().enumerate() {
            if !written[i] {
                out.push(format!("{k}={v}"));
                written[i] = true;
            }
        }
    };
    for line in content.lines() {
        if let Some(name) = is_section(line) {
            if in_section {
                // Keep a blank line before the next section.
                let trailing_blank = out.last().is_some_and(|l| l.trim().is_empty());
                if trailing_blank {
                    out.pop();
                }
                flush_missing(&mut out, &mut written);
                if trailing_blank {
                    out.push(String::new());
                }
            }
            in_section = name.eq_ignore_ascii_case(SECTION);
            section_seen |= in_section;
            out.push(line.to_string());
            continue;
        }
        if in_section {
            if let Some((k, _)) = key_value(line) {
                if let Some(i) = keys.iter().position(|(key, _)| key.eq_ignore_ascii_case(k)) {
                    out.push(format!("{}={}", keys[i].0, keys[i].1));
                    written[i] = true;
                    continue;
                }
            }
        }
        out.push(line.to_string());
    }
    if in_section {
        flush_missing(&mut out, &mut written);
    } else if !section_seen {
        if out.last().is_some_and(|l| !l.trim().is_empty()) {
            out.push(String::new());
        }
        out.push(format!("[{SECTION}]"));
        flush_missing(&mut out, &mut written);
    }
    let mut text = out.join("\r\n");
    text.push_str("\r\n");
    text
}

pub fn user_ini_path() -> AppResult<PathBuf> {
    let config_dir = paths::rl_user_dir()
        .map(|d| d.join("Config"))
        .filter(|d| d.is_dir())
        .ok_or_else(|| {
            AppError::NotFound("game user config folder (launch Rocket League once)".into())
        })?;
    Ok(config_dir.join(INI_NAME))
}

fn state_path() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("stats")?.join("ini-state.json"))
}

fn original_backup() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("stats")?.join("TAStatsAPI.ini.orig"))
}

pub fn current() -> AppResult<(PathBuf, IniSettings)> {
    let path = user_ini_path()?;
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    Ok((path, parse(&content)))
}

/// Turns the Stats API on (takes effect at the next game start).
pub fn enable() -> AppResult<IniSettings> {
    let path = user_ini_path()?;
    let existing = std::fs::read_to_string(&path).ok();
    let mut state: IniState = fsx::read_json_or_default(&state_path()?)?;
    if !state.enabled_by_us {
        match &existing {
            Some(text) => fsx::write_atomic(&original_backup()?, text.as_bytes())?,
            None => state.created_by_us = true,
        }
    }
    let content = existing.unwrap_or_default();
    let settings = parse(&content);
    let web_port = settings
        .web_port
        .filter(|p| *p > 0)
        .unwrap_or(DEFAULT_WEB_PORT);
    let updated = upsert(
        &content,
        &[
            ("WebPort", web_port.to_string()),
            ("PacketSendRate", PACKET_SEND_RATE.to_string()),
        ],
    );
    fsx::write_atomic(&path, updated.as_bytes())?;
    state.enabled_by_us = true;
    fsx::write_json(&state_path()?, &state)?;
    tracing::info!(path = %path.display(), web_port, "stats API enabled");
    Ok(parse(&updated))
}

/// Puts the user's file back as it was (or sets the send rate to 0 when
/// we did not enable it ourselves).
pub fn disable() -> AppResult<IniSettings> {
    let path = user_ini_path()?;
    let state: IniState = fsx::read_json_or_default(&state_path()?)?;
    let backup = original_backup()?;
    if state.enabled_by_us && state.created_by_us {
        if path.is_file() {
            std::fs::remove_file(&path)?;
        }
    } else if state.enabled_by_us && backup.is_file() {
        fsx::copy_atomic(&backup, &path)?;
    } else {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        fsx::write_atomic(
            &path,
            upsert(&content, &[("PacketSendRate", "0".into())]).as_bytes(),
        )?;
    }
    let _ = std::fs::remove_file(&backup);
    fsx::write_json(&state_path()?, &IniState::default())?;
    tracing::info!("stats API disabled");
    Ok(current()?.1)
}

/// True when we enabled it but the game (update, ini regeneration) turned
/// it back off.
pub fn needs_reapply() -> bool {
    let Ok(path) = state_path() else { return false };
    let state: IniState = fsx::read_json_or_default(&path).unwrap_or_default();
    state.enabled_by_us && current().map(|(_, s)| !s.enabled()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: &str = "[TAGame.MatchStatsExporter_TA]\nPort=49123\nWebPort=49124\nPacketSendRate=0\n\n[IniVersion]\n0=1786207954.000000\n";

    #[test]
    fn parses_user_file() {
        let s = parse(USER);
        assert_eq!(s.port, Some(49123));
        assert_eq!(s.web_port, Some(49124));
        assert_eq!(s.packet_send_rate, Some(0.0));
        assert!(!s.enabled());
    }

    #[test]
    fn upsert_keeps_other_sections() {
        let out = upsert(
            USER,
            &[("WebPort", "49124".into()), ("PacketSendRate", "30".into())],
        );
        let s = parse(&out);
        assert!(s.enabled());
        assert_eq!(s.port, Some(49123));
        assert!(out.contains("[IniVersion]\r\n0=1786207954.000000"));
        assert_eq!(out.matches("PacketSendRate").count(), 1);
    }

    #[test]
    fn upsert_creates_section_and_missing_keys() {
        let out = upsert(
            "",
            &[("WebPort", "49124".into()), ("PacketSendRate", "30".into())],
        );
        assert!(out.starts_with("[TAGame.MatchStatsExporter_TA]"));
        assert!(parse(&out).enabled());
        let partial = upsert(
            "[TAGame.MatchStatsExporter_TA]\n; comment\nPort=1\n[Other]\nX=1\n",
            &[("PacketSendRate", "30".into())],
        );
        let lines: Vec<&str> = partial.lines().collect();
        assert_eq!(
            lines,
            vec![
                "[TAGame.MatchStatsExporter_TA]",
                "; comment",
                "Port=1",
                "PacketSendRate=30",
                "[Other]",
                "X=1"
            ]
        );
    }

    #[test]
    fn keys_are_case_insensitive() {
        let s = parse("[tagame.matchstatsexporter_ta]\nwebport = 5000\npacketsendrate=60\n");
        assert_eq!(s.web_port, Some(5000));
        assert!(s.enabled());
    }
}
