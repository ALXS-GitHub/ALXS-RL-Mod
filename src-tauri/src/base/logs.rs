//! Log viewer: reads the app's own logs and Rocket League's `Launch*.log`
//! and parses each line (time, level, source) for the Logs page.
//!
//! Read-only, and only inside the two known folders: the requested file
//! name must be one of the listed files (no path from the front-end).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};
use crate::base::paths;

/// Only the end of a big file is shown.
const MAX_BYTES: u64 = 4 * 1024 * 1024;
const MAX_LINES: usize = 5000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LogSource {
    App,
    Game,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    /// Timestamp as written (ISO for the app, seconds since start for the game).
    pub time: Option<String>,
    pub level: LogLevel,
    /// Module (app) or category (game).
    pub target: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileInfo {
    pub name: String,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogView {
    pub source: LogSource,
    /// File shown (`None` when the folder has no log yet).
    pub file: Option<String>,
    pub files: Vec<LogFileInfo>,
    pub lines: Vec<LogLine>,
    /// Older lines were left out (file bigger than the view).
    pub truncated: bool,
    pub folder: Option<String>,
}

fn folder(source: LogSource) -> Option<PathBuf> {
    match source {
        LogSource::App => paths::logs_dir().ok(),
        LogSource::Game => paths::rl_user_dir().map(|d| d.join("Logs")),
    }
}

fn is_log_file(source: LogSource, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    match source {
        LogSource::App => lower.starts_with("alxs-rl-mod.log"),
        LogSource::Game => lower.starts_with("launch") && lower.ends_with(".log"),
    }
}

/// Log files of a folder, newest first.
fn list(source: LogSource, dir: &Path) -> Vec<LogFileInfo> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<LogFileInfo> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            is_log_file(source, &name).then(|| LogFileInfo {
                name,
                size: meta.len(),
                modified: meta.modified().ok().map(DateTime::<Utc>::from),
            })
        })
        .collect();
    files.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| b.name.cmp(&a.name))
    });
    files
}

/// UTF-8 when valid, else Windows-1252-ish (the game writes the system code page).
fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// Reads the end of `path` (the game keeps `Launch.log` open: shared read).
fn read_tail(path: &Path) -> AppResult<(String, bool)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(|e| AppError::from_game_io(e, path))?;
    let len = file.metadata()?.len();
    let truncated = len > MAX_BYTES;
    if truncated {
        file.seek(SeekFrom::Start(len - MAX_BYTES))?;
    }
    let mut bytes = Vec::with_capacity(len.min(MAX_BYTES) as usize);
    file.read_to_end(&mut bytes)?;
    let mut text = decode(&bytes);
    if truncated {
        // Drop the partial first line.
        if let Some(i) = text.find('\n') {
            text.drain(..=i);
        }
    }
    Ok((text, truncated))
}

fn app_level(token: &str) -> Option<LogLevel> {
    match token {
        "ERROR" => Some(LogLevel::Error),
        "WARN" => Some(LogLevel::Warn),
        "INFO" => Some(LogLevel::Info),
        "DEBUG" | "TRACE" => Some(LogLevel::Debug),
        _ => None,
    }
}

/// `2026-09-27T08:52:43.785533Z  INFO alxs_rl_mod_lib::maps::session: message`
pub fn parse_app_line(line: &str) -> Option<LogLine> {
    let mut parts = line.splitn(2, char::is_whitespace);
    let time = parts.next()?;
    if !time.ends_with('Z') || !time.contains('T') {
        return None;
    }
    let rest = parts.next()?.trim_start();
    let (level_token, rest) = rest.split_once(char::is_whitespace)?;
    let level = app_level(level_token)?;
    let rest = rest.trim_start();
    let (target, message) = match rest.split_once(": ") {
        Some((t, m)) if !t.contains(' ') => (Some(t.to_string()), m.to_string()),
        _ => (None, rest.to_string()),
    };
    Some(LogLine {
        time: Some(time.to_string()),
        level,
        target,
        message,
    })
}

fn game_level(category: &str, message: &str) -> LogLevel {
    let critical = message.contains("appError") || message.contains("Critical error");
    match category {
        "Error" | "Critical" => LogLevel::Error,
        _ if critical => LogLevel::Error,
        "Warning" | "ScriptWarning" => LogLevel::Warn,
        _ => LogLevel::Info,
    }
}

/// `[0024.47] Warning: Package ... ` (`[time] Category: message`).
pub fn parse_game_line(line: &str) -> Option<LogLine> {
    let rest = line.strip_prefix('[')?;
    let (time, rest) = rest.split_once(']')?;
    if !time.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let rest = rest.trim_start();
    let (category, message) = match rest.split_once(": ") {
        Some((c, m)) if !c.contains(' ') => (Some(c), m),
        _ => (None, rest),
    };
    Some(LogLine {
        time: Some(time.to_string()),
        level: game_level(category.unwrap_or(""), message),
        target: category.map(str::to_string),
        message: message.to_string(),
    })
}

/// Parses a whole file. Lines that do not start a new entry (multi-line
/// messages, indented dumps) continue the previous one's level.
pub fn parse(source: LogSource, text: &str) -> Vec<LogLine> {
    let parse_line = match source {
        LogSource::App => parse_app_line,
        LogSource::Game => parse_game_line,
    };
    let mut out: Vec<LogLine> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(line) {
            Some(l) => out.push(l),
            None => {
                let level = out.last().map_or(LogLevel::Info, |l| l.level);
                out.push(LogLine {
                    time: None,
                    level,
                    target: None,
                    message: line.to_string(),
                });
            }
        }
    }
    out
}

pub fn read(source: LogSource, file: Option<&str>) -> AppResult<LogView> {
    let dir = folder(source);
    let files = dir.as_deref().map(|d| list(source, d)).unwrap_or_default();
    let chosen = match file {
        Some(name) => Some(
            files
                .iter()
                .find(|f| f.name == name)
                .ok_or_else(|| AppError::NotFound(format!("log file {name}")))?
                .name
                .clone(),
        ),
        None => files.first().map(|f| f.name.clone()),
    };
    let (mut lines, mut truncated) = (Vec::new(), false);
    if let (Some(dir), Some(name)) = (&dir, &chosen) {
        let (text, cut) = read_tail(&dir.join(name))?;
        lines = parse(source, &text);
        truncated = cut;
        if lines.len() > MAX_LINES {
            lines.drain(..lines.len() - MAX_LINES);
            truncated = true;
        }
    }
    Ok(LogView {
        source,
        file: chosen,
        files,
        lines,
        truncated,
        folder: dir.map(|d| d.to_string_lossy().into_owned()),
    })
}

/// App logs kept by default.
pub const RETENTION_DAYS: u64 = 30;

/// App log files other than today's (the one being written stays).
fn removable_app_logs(dir: &Path) -> Vec<LogFileInfo> {
    let today = format!("alxs-rl-mod.log.{}", chrono::Utc::now().format("%Y-%m-%d"));
    list(LogSource::App, dir)
        .into_iter()
        .filter(|f| f.name != today)
        .collect()
}

fn remove(dir: &Path, files: &[LogFileInfo]) -> u32 {
    files
        .iter()
        .filter(|f| std::fs::remove_file(dir.join(&f.name)).is_ok())
        .count() as u32
}

/// Deletes app logs older than [`RETENTION_DAYS`] (called at startup).
pub fn prune_old() {
    let Ok(dir) = paths::logs_dir() else { return };
    let cutoff = Utc::now() - chrono::Duration::days(RETENTION_DAYS as i64);
    let old: Vec<_> = removable_app_logs(&dir)
        .into_iter()
        .filter(|f| f.modified.is_some_and(|m| m < cutoff))
        .collect();
    let n = remove(&dir, &old);
    if n > 0 {
        tracing::info!(files = n, "old app logs removed");
    }
}

/// Deletes every app log except today's. Returns the number of files removed.
#[tauri::command]
pub fn logs_clean() -> AppResult<u32> {
    let dir = paths::logs_dir()?;
    let n = remove(&dir, &removable_app_logs(&dir));
    tracing::info!(files = n, "app logs cleaned");
    Ok(n)
}

#[tauri::command]
pub async fn logs_read(source: LogSource, file: Option<String>) -> AppResult<LogView> {
    tauri::async_runtime::spawn_blocking(move || read(source, file.as_deref()))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_app_lines() {
        let l = parse_app_line(
            "2026-09-27T08:52:43.785533Z  INFO alxs_rl_mod_lib::maps::session: leftover map files removed",
        )
        .unwrap();
        assert_eq!(l.level, LogLevel::Info);
        assert_eq!(l.target.as_deref(), Some("alxs_rl_mod_lib::maps::session"));
        assert_eq!(l.message, "leftover map files removed");
        let w = parse_app_line("2026-09-27T08:52:43Z  WARN x::y: could not: persist").unwrap();
        assert_eq!(
            (w.level, w.message.as_str()),
            (LogLevel::Warn, "could not: persist")
        );
        assert!(parse_app_line("  continuation").is_none());
    }

    #[test]
    fn parses_game_lines() {
        let e = parse_game_line(
            "[0025.62] Critical: appError called: Bad export index 33554419/98: ..\\Wheel_SoccerBall_SF.upk",
        )
        .unwrap();
        assert_eq!(
            (e.level, e.target.as_deref()),
            (LogLevel::Error, Some("Critical"))
        );
        let w = parse_game_line("[0024.47] Warning: Package 'x' has been saved").unwrap();
        assert_eq!(w.level, LogLevel::Warn);
        let i = parse_game_line("[0015.24] AuthEpicCritical: Sending Eos login request").unwrap();
        assert_eq!(
            i.level,
            LogLevel::Info,
            "category names ending in Critical are not errors"
        );
        assert_eq!(
            parse_game_line("[0046.84] Log: === Critical error: ===")
                .unwrap()
                .level,
            LogLevel::Error
        );
        assert!(parse_game_line("\tGameEvent_Soccar_TA Labs_Utopia_P").is_none());
    }

    #[test]
    fn continuation_lines_keep_the_previous_level() {
        let text = "[0001.00] Warning: first\n\tdetail\n[0002.00] Log: ok\r\n\n";
        let lines = parse(LogSource::Game, text);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1].level, LogLevel::Warn);
        assert_eq!(lines[1].time, None);
        assert_eq!(lines[2].message, "ok");
    }

    #[test]
    fn only_known_log_files_are_listed() {
        assert!(is_log_file(LogSource::App, "alxs-rl-mod.log.2026-09-27"));
        assert!(is_log_file(
            LogSource::Game,
            "Launch-backup-2026.09.27-10.38.32.log"
        ));
        assert!(!is_log_file(LogSource::Game, "..\\..\\secret.txt"));
        assert!(!is_log_file(LogSource::App, "config.json"));
    }

    #[test]
    fn decodes_ansi_game_logs() {
        assert_eq!(decode(b"L\xe9o"), "Léo");
        assert_eq!(decode("déjà".as_bytes()), "déjà");
    }
}
