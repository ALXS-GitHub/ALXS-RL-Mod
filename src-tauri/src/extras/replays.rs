//! Local replay files (`Documents\My Games\Rocket League\TAGame\Demos`,
//! and `DemosEpic` for the Epic Games version).
//!
//! Only the replay **header** is parsed (a few KB): Unreal property list with
//! name, map, date, score, team size and duration. Network frames are never
//! read, so listing hundreds of replays stays instant.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::base::error::AppResult;
use crate::base::paths;

/// Headers are small; never read more than this from a file.
const HEADER_READ_LIMIT: u64 = 96 * 1024;
const MAX_LISTED: usize = 400;
const MAX_DEPTH: usize = 4;

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayHeader {
    pub name: Option<String>,
    pub map: Option<String>,
    pub date: Option<String>,
    pub team_size: Option<i32>,
    pub blue_score: Option<i32>,
    pub orange_score: Option<i32>,
    pub duration_seconds: Option<f32>,
    pub match_type: Option<String>,
    pub player_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayFile {
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub header: Option<ReplayHeader>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayList {
    /// Existing replay folders (Steam `Demos`, Epic `DemosEpic`).
    pub folders: Vec<String>,
    pub replays: Vec<ReplayFile>,
}

pub fn demos_dirs() -> Vec<PathBuf> {
    paths::rl_user_dir()
        .map(|d| vec![d.join("DemosEpic"), d.join("Demos")])
        .unwrap_or_default()
        .into_iter()
        .filter(|d| d.is_dir())
        .collect()
}

pub fn list() -> AppResult<ReplayList> {
    let dirs = demos_dirs();
    // Newest first; headers are parsed only for the files we return.
    let mut files: Vec<(PathBuf, std::fs::Metadata)> = dirs
        .iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("replay"))
        })
        .filter_map(|p| p.metadata().ok().map(|m| (p, m)))
        .collect();
    files.sort_by_key(|(_, m)| std::cmp::Reverse(m.modified().ok()));
    files.truncate(MAX_LISTED);
    let replays = files
        .into_iter()
        .filter_map(|(p, meta)| {
            Some(ReplayFile {
                file_name: p.file_name()?.to_string_lossy().into_owned(),
                path: p.to_string_lossy().into_owned(),
                size_bytes: meta.len(),
                modified_at: meta
                    .modified()
                    .ok()
                    .map(chrono::DateTime::<chrono::Utc>::from),
                header: read_header(&p),
            })
        })
        .collect();
    Ok(ReplayList {
        folders: dirs
            .iter()
            .map(|d| d.to_string_lossy().into_owned())
            .collect(),
        replays,
    })
}

fn read_header(path: &Path) -> Option<ReplayHeader> {
    let mut buf = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(HEADER_READ_LIMIT)
        .read_to_end(&mut buf)
        .ok()?;
    parse_header(&buf)
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.buf.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    /// Unreal FString: i32 length incl. NUL; negative = UTF-16LE.
    fn fstring(&mut self) -> Option<String> {
        let len = self.i32()?;
        if len == 0 {
            return Some(String::new());
        }
        if len > 0 {
            let bytes = self.take(usize::try_from(len).ok().filter(|l| *l <= 4096)?)?;
            let trimmed = bytes.strip_suffix(&[0]).unwrap_or(bytes);
            // Positive-length FStrings are Latin-1 (UE3 ANSI), not UTF-8.
            Some(trimmed.iter().map(|&b| char::from(b)).collect())
        } else {
            let chars = usize::try_from(len.checked_neg()?)
                .ok()
                .filter(|l| *l <= 4096)?;
            let bytes = self.take(chars * 2)?;
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let trimmed = units.strip_suffix(&[0]).unwrap_or(&units);
            Some(String::from_utf16_lossy(trimmed))
        }
    }
}

#[derive(Default)]
struct Props {
    ints: Vec<(String, i32)>,
    strs: Vec<(String, String)>,
    floats: Vec<(String, f32)>,
}

impl Props {
    fn int(&self, k: &str) -> Option<i32> {
        self.ints.iter().find(|(n, _)| n == k).map(|(_, v)| *v)
    }
    fn str(&self, k: &str) -> Option<String> {
        self.strs
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.clone())
            .filter(|v| !v.is_empty())
    }
    fn float(&self, k: &str) -> Option<f32> {
        self.floats.iter().find(|(n, _)| n == k).map(|(_, v)| *v)
    }
}

/// Reads one property list (until "None"). Top-level values are collected;
/// nested array elements are parsed only to be skipped correctly.
fn read_props(c: &mut Cursor<'_>, out: &mut Props, collect: bool, depth: usize) -> Option<()> {
    if depth > MAX_DEPTH {
        return None;
    }
    for _ in 0..512 {
        let name = c.fstring()?;
        if name == "None" {
            return Some(());
        }
        let kind = c.fstring()?;
        let size = usize::try_from(c.u64()? & 0xFFFF_FFFF).ok()?;
        match kind.as_str() {
            "IntProperty" => {
                let v = c.i32()?;
                if collect {
                    out.ints.push((name, v));
                }
            }
            "StrProperty" | "NameProperty" => {
                let v = c.fstring()?;
                if collect {
                    out.strs.push((name, v));
                }
            }
            "FloatProperty" => {
                let v = c.f32()?;
                if collect {
                    out.floats.push((name, v));
                }
            }
            "BoolProperty" => {
                c.take(1)?;
            }
            "QWordProperty" => {
                c.u64()?;
            }
            "ByteProperty" => {
                let enum_name = c.fstring()?;
                // Platform bytes carry a single string; other enums a pair.
                if !enum_name.starts_with("OnlinePlatform_") {
                    c.fstring()?;
                }
            }
            // The tag size is the exact payload length: skip arrays wholesale
            // instead of walking their (deeply nested, struct-heavy) elements.
            "ArrayProperty" => {
                c.take(size)?;
            }
            "StructProperty" => {
                c.fstring()?; // struct type name, then `size` bytes of data
                c.take(size)?;
            }
            _ => {
                c.take(size)?;
            }
        }
    }
    None
}

/// Parses the replay header. `None` if the data is not a replay.
pub fn parse_header(buf: &[u8]) -> Option<ReplayHeader> {
    let mut c = Cursor { buf, pos: 0 };
    let _header_size = c.u32()?;
    let _crc = c.u32()?;
    let major = c.u32()?;
    let minor = c.u32()?;
    if major >= 868 && minor >= 18 {
        c.u32()?; // net version
    }
    let game_type = c.fstring()?;
    if !game_type.starts_with("TAGame.") {
        return None;
    }
    let mut props = Props::default();
    read_props(&mut c, &mut props, true, 0)?;
    let duration = match (props.int("NumFrames"), props.float("RecordFPS")) {
        (Some(frames), Some(fps)) if fps > 0.0 && frames > 0 => Some(frames as f32 / fps),
        _ => None,
    };
    Some(ReplayHeader {
        name: props.str("ReplayName"),
        map: props.str("MapName"),
        date: props.str("Date"),
        team_size: props.int("TeamSize"),
        blue_score: props.int("Team0Score"),
        orange_score: props.int("Team1Score"),
        duration_seconds: duration,
        match_type: props.str("MatchType"),
        player_name: props.str("PlayerName"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fstr(out: &mut Vec<u8>, s: &str) {
        out.extend((s.len() as i32 + 1).to_le_bytes());
        out.extend(s.as_bytes());
        out.push(0);
    }
    fn prop_header(out: &mut Vec<u8>, name: &str, kind: &str, size: u64) {
        fstr(out, name);
        fstr(out, kind);
        out.extend(size.to_le_bytes());
    }

    fn sample() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(1000u32.to_le_bytes()); // header size
        b.extend(0u32.to_le_bytes()); // crc
        b.extend(868u32.to_le_bytes());
        b.extend(32u32.to_le_bytes());
        b.extend(11u32.to_le_bytes()); // net version
        fstr(&mut b, "TAGame.Replay_Soccar_TA");
        prop_header(&mut b, "TeamSize", "IntProperty", 4);
        b.extend(2i32.to_le_bytes());
        prop_header(&mut b, "Team0Score", "IntProperty", 4);
        b.extend(3i32.to_le_bytes());
        prop_header(&mut b, "ReplayName", "StrProperty", 10);
        fstr(&mut b, "Clutch");
        // Array payload: count + one element (props until "None").
        let mut goals = Vec::new();
        goals.extend(1i32.to_le_bytes());
        prop_header(&mut goals, "frame", "IntProperty", 4);
        goals.extend(120i32.to_le_bytes());
        prop_header(&mut goals, "PlayerTeam", "ByteProperty", 8);
        fstr(&mut goals, "ETeam");
        fstr(&mut goals, "Blue");
        fstr(&mut goals, "None");
        prop_header(&mut b, "Goals", "ArrayProperty", goals.len() as u64);
        b.extend(goals);
        prop_header(&mut b, "Platform", "ByteProperty", 8);
        fstr(&mut b, "OnlinePlatform_Steam");
        prop_header(&mut b, "bUnfairBots", "BoolProperty", 0);
        b.push(0);
        prop_header(&mut b, "MapName", "NameProperty", 8);
        fstr(&mut b, "Stadium_P");
        prop_header(&mut b, "NumFrames", "IntProperty", 4);
        b.extend(9000i32.to_le_bytes());
        prop_header(&mut b, "RecordFPS", "FloatProperty", 4);
        b.extend(30.0f32.to_le_bytes());
        prop_header(&mut b, "Weird", "StructPropertyX", 3);
        b.extend([1, 2, 3]);
        fstr(&mut b, "None");
        b
    }

    #[test]
    fn ansi_strings_are_latin1() {
        let mut b = Vec::new();
        b.extend(6i32.to_le_bytes());
        b.extend([b'b', b'a', b'l', 0xE0, b'!', 0]);
        let mut c = Cursor { buf: &b, pos: 0 };
        assert_eq!(c.fstring().as_deref(), Some("balà!"));
    }

    #[test]
    fn parses_synthetic_header() {
        let h = parse_header(&sample()).unwrap();
        assert_eq!(h.name.as_deref(), Some("Clutch"));
        assert_eq!(h.map.as_deref(), Some("Stadium_P"));
        assert_eq!(h.team_size, Some(2));
        assert_eq!(h.blue_score, Some(3));
        assert_eq!(h.orange_score, None);
        assert_eq!(h.duration_seconds, Some(300.0));
    }

    #[test]
    fn rejects_garbage_and_truncation() {
        assert!(parse_header(b"not a replay").is_none());
        let full = sample();
        assert!(parse_header(&full[..full.len() / 2]).is_none());
    }

    #[test]
    fn reads_utf16_strings() {
        let mut b = Vec::new();
        let s: Vec<u16> = "Éa".encode_utf16().chain([0]).collect();
        b.extend((-(s.len() as i32)).to_le_bytes());
        for u in s {
            b.extend(u.to_le_bytes());
        }
        let mut c = Cursor { buf: &b, pos: 0 };
        assert_eq!(c.fstring().as_deref(), Some("Éa"));
    }
}
