//! Invite codes: what a friend needs to join a hosted game — the host's
//! tailnet address and which map to install (and over which Labs arena).
//!
//! Format: `ALXS-MULTI-` + base64url(JSON). Everything is validated on
//! decode: the address and port end up in a console command.

use std::net::Ipv4Addr;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};
use crate::maps::model::{normalize_target, RemoteSource};

pub const PREFIX: &str = "ALXS-MULTI-";
pub const VERSION: u8 = 1;
/// Rocket Plugin's default game port (`DEFAULT_PORT` in RocketPlugin.h).
pub const DEFAULT_PORT: u16 = 7777;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InviteMap {
    pub name: String,
    /// Labs arena the map replaces (`Labs_Underpass_P.upk`): the level name
    /// the host opens, so both sides must use the same one.
    pub target: String,
    /// SHA-256 of the map package: both sides need the same file.
    pub sha256: Option<String>,
    pub source: Option<RemoteSource>,
    pub remote_id: Option<String>,
    /// Repair mapping version the host's map was repaired with (`None` =
    /// original package). Friends repair theirs the same way: same result.
    #[serde(default)]
    pub repair_version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    pub v: u8,
    pub host: Ipv4Addr,
    pub port: u16,
    pub host_name: Option<String>,
    pub map: InviteMap,
}

pub fn encode(invite: &Invite) -> AppResult<String> {
    let json = serde_json::to_vec(invite)?;
    Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json)))
}

fn bad(msg: &str) -> AppError {
    AppError::InvalidInput(format!("invite code: {msg}"))
}

pub fn decode(code: &str) -> AppResult<Invite> {
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let body = compact
        .strip_prefix(PREFIX)
        .ok_or_else(|| bad("not an ALXS-RL-Mod invite"))?;
    if body.len() > 4096 {
        return Err(bad("too long"));
    }
    let json = URL_SAFE_NO_PAD
        .decode(body)
        .map_err(|_| bad("damaged (copy it again)"))?;
    let mut invite: Invite = serde_json::from_slice(&json).map_err(|_| bad("unreadable"))?;
    if invite.v != VERSION {
        return Err(bad("made by another version of the app"));
    }
    if invite.host.is_unspecified() || invite.host.is_broadcast() || invite.host.is_multicast() {
        return Err(bad("bad host address"));
    }
    if invite.port == 0 {
        return Err(bad("bad port"));
    }
    invite.map.target = normalize_target(&invite.map.target)
        .ok_or_else(|| bad("unknown Labs arena"))?
        .to_string();
    invite.map.name = invite.map.name.trim().chars().take(120).collect();
    if let Some(sha) = &invite.map.sha256 {
        if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad("bad map hash"));
        }
    }
    if let Some(id) = &invite.map.remote_id {
        if id.is_empty() || id.len() > 32 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(bad("bad map id"));
        }
    }
    invite.host_name = invite
        .host_name
        .map(|n| n.chars().filter(|c| !c.is_control()).take(64).collect());
    Ok(invite)
}

/// Level name of a Labs arena file (`Labs_Underpass_P.upk` → `Labs_Underpass_P`).
pub fn level_name(target: &str) -> &str {
    target
        .strip_suffix(".upk")
        .or_else(|| target.strip_suffix(".UPK"))
        .unwrap_or(target)
}

/// Rocket Plugin command hosting `target` (a validated Labs arena).
pub fn host_command(target: &str) -> String {
    format!("rp host {}", level_name(target))
}

/// Rocket Plugin command joining a host (port is a separate argument).
pub fn join_command(host: Ipv4Addr, port: u16) -> String {
    format!("rp join {host} {port}")
}

/// True for Tailscale's CGNAT range (100.64.0.0/10).
pub fn is_tailnet(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 100 && (64..128).contains(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Invite {
        Invite {
            v: VERSION,
            host: Ipv4Addr::new(100, 101, 102, 103),
            port: DEFAULT_PORT,
            host_name: Some("desktop".into()),
            map: InviteMap {
                name: "Dribble Challenge".into(),
                target: "Labs_Underpass_P.upk".into(),
                sha256: Some("a".repeat(64)),
                source: Some(RemoteSource::BakkesPlugins),
                remote_id: Some("123".into()),
                repair_version: Some(1),
            },
        }
    }

    #[test]
    fn round_trip() {
        let code = encode(&sample()).unwrap();
        assert!(code.starts_with(PREFIX));
        assert_eq!(decode(&code).unwrap(), sample());
        // Pasted with line breaks / spaces.
        let messy = format!("  {}\n{} ", &code[..20], &code[20..]);
        assert_eq!(decode(&messy).unwrap(), sample());
    }

    #[test]
    fn target_is_normalised() {
        let mut i = sample();
        i.map.target = "labs_underpass_p".into();
        assert_eq!(
            decode(&encode(&i).unwrap()).unwrap().map.target,
            "Labs_Underpass_P.upk"
        );
    }

    #[test]
    fn rejects_bad_codes() {
        assert!(decode("hello").is_err());
        assert!(decode("ALXS-MULTI-!!!").is_err());
        let mut i = sample();
        i.map.target = "../../Engine.upk".into();
        assert!(decode(&encode(&i).unwrap()).is_err());
        let mut i = sample();
        i.map.remote_id = Some("1; quit".into());
        assert!(decode(&encode(&i).unwrap()).is_err());
        let mut i = sample();
        i.host = Ipv4Addr::UNSPECIFIED;
        assert!(decode(&encode(&i).unwrap()).is_err());
        let mut i = sample();
        i.v = 9;
        assert!(decode(&encode(&i).unwrap()).is_err());
        // A host field that is not an IPv4 never parses (no command injection).
        let forged = format!(
            "{PREFIX}{}",
            URL_SAFE_NO_PAD.encode(
                br#"{"v":1,"host":"1.2.3.4; quit","port":7777,"hostName":null,"map":{"name":"x","target":"Labs_Underpass_P.upk","sha256":null,"source":null,"remoteId":null}}"#
            )
        );
        assert!(decode(&forged).is_err());
    }

    #[test]
    fn commands() {
        assert_eq!(
            host_command("Labs_Underpass_P.upk"),
            "rp host Labs_Underpass_P"
        );
        assert_eq!(
            join_command(Ipv4Addr::new(100, 64, 0, 2), 7777),
            "rp join 100.64.0.2 7777"
        );
        assert!(is_tailnet(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(is_tailnet(Ipv4Addr::new(100, 127, 255, 255)));
        assert!(!is_tailnet(Ipv4Addr::new(100, 128, 0, 1)));
        assert!(!is_tailnet(Ipv4Addr::new(192, 168, 1, 2)));
    }
}
