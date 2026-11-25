//! Read-only Tailscale detection: is the client installed, connected, what
//! is our tailnet address and which friends are online.
//!
//! Only the local CLI is used (`tailscale status --json`, which talks to the
//! local daemon). The app never installs, logs in or reconfigures Tailscale.

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TailscaleState {
    /// No `tailscale.exe` found.
    NotInstalled,
    /// Installed, but the background service does not answer.
    ServiceDown,
    /// The user has to sign in (or approve this device).
    NeedsLogin,
    /// Signed in but disconnected in the Tailscale menu.
    Stopped,
    Running,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TailscalePeer {
    pub name: String,
    pub ipv4: Option<String>,
    pub online: bool,
    pub os: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TailscaleStatus {
    pub state: TailscaleState,
    pub cli_path: Option<String>,
    /// This device's name on the tailnet.
    pub self_name: Option<String>,
    /// This device's tailnet IPv4 (`100.x.y.z`): what friends join.
    pub self_ipv4: Option<String>,
    /// Name of the tailnet (account) when known.
    pub tailnet: Option<String>,
    pub peers: Vec<TailscalePeer>,
}

impl TailscaleStatus {
    fn with_state(state: TailscaleState, cli: Option<&PathBuf>) -> Self {
        Self {
            state,
            cli_path: cli.map(|p| p.to_string_lossy().into_owned()),
            self_name: None,
            self_ipv4: None,
            tailnet: None,
            peers: Vec::new(),
        }
    }
}

/// Default install locations, then `PATH`.
fn cli_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(pf) = std::env::var_os(var) {
            out.push(PathBuf::from(pf).join("Tailscale").join("tailscale.exe"));
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        out.extend(std::env::split_paths(&path).map(|d| d.join("tailscale.exe")));
    }
    out
}

pub fn find_cli() -> Option<PathBuf> {
    cli_candidates().into_iter().find(|p| p.is_file())
}

/// Runs a console program without flashing a console window.
pub(crate) fn hidden_command(exe: &std::path::Path) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

pub fn status() -> TailscaleStatus {
    let Some(cli) = find_cli() else {
        return TailscaleStatus::with_state(TailscaleState::NotInstalled, None);
    };
    let output = hidden_command(&cli).args(["status", "--json"]).output();
    let Ok(output) = output else {
        return TailscaleStatus::with_state(TailscaleState::ServiceDown, Some(&cli));
    };
    // `status --json` prints the JSON even when logged out (exit code 0);
    // a stopped service prints an error on stderr instead.
    match parse_status(&output.stdout) {
        Some(mut s) => {
            s.cli_path = Some(cli.to_string_lossy().into_owned());
            s
        }
        None => TailscaleStatus::with_state(TailscaleState::ServiceDown, Some(&cli)),
    }
}

// --- `tailscale status --json` (only the fields we read) -------------------

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawStatus {
    backend_state: Option<String>,
    #[serde(rename = "Self")]
    self_node: Option<RawNode>,
    #[serde(default)]
    peer: Option<std::collections::HashMap<String, RawNode>>,
    current_tailnet: Option<RawTailnet>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawNode {
    host_name: Option<String>,
    #[serde(rename = "DNSName")]
    dns_name: Option<String>,
    #[serde(rename = "OS")]
    os: Option<String>,
    #[serde(default, rename = "TailscaleIPs")]
    tailscale_ips: Option<Vec<String>>,
    #[serde(default)]
    online: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTailnet {
    name: Option<String>,
}

fn first_ipv4(ips: Option<&Vec<String>>) -> Option<String> {
    ips?.iter()
        .find_map(|ip| ip.parse::<Ipv4Addr>().ok())
        .map(|ip| ip.to_string())
}

/// MagicDNS short name (`laptop.tail1234.ts.net.` → `laptop`), else host name.
fn node_name(node: &RawNode) -> String {
    node.dns_name
        .as_deref()
        .and_then(|d| d.split('.').next())
        .filter(|s| !s.is_empty())
        .or(node.host_name.as_deref())
        .unwrap_or("?")
        .to_string()
}

pub(crate) fn parse_status(stdout: &[u8]) -> Option<TailscaleStatus> {
    let raw: RawStatus = serde_json::from_slice(stdout).ok()?;
    let state = match raw.backend_state.as_deref() {
        Some("Running") => TailscaleState::Running,
        Some("NeedsLogin") | Some("NeedsMachineAuth") | Some("NoState") => {
            TailscaleState::NeedsLogin
        }
        Some("Stopped") => TailscaleState::Stopped,
        Some("Starting") => TailscaleState::Stopped,
        _ => TailscaleState::ServiceDown,
    };
    let mut peers: Vec<TailscalePeer> = raw
        .peer
        .unwrap_or_default()
        .values()
        .map(|n| TailscalePeer {
            name: node_name(n),
            ipv4: first_ipv4(n.tailscale_ips.as_ref()),
            online: n.online,
            os: n.os.clone().filter(|o| !o.is_empty()),
        })
        .collect();
    peers.sort_by(|a, b| {
        b.online
            .cmp(&a.online)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Some(TailscaleStatus {
        state,
        cli_path: None,
        self_name: raw.self_node.as_ref().map(node_name),
        self_ipv4: raw
            .self_node
            .as_ref()
            .and_then(|n| first_ipv4(n.tailscale_ips.as_ref()))
            .filter(|_| state == TailscaleState::Running),
        tailnet: raw.current_tailnet.and_then(|t| t.name),
        peers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "Version": "1.80.0",
      "BackendState": "Running",
      "TailscaleIPs": ["100.101.102.103", "fd7a:115c:a1e0::1"],
      "Self": {
        "HostName": "DESKTOP-ABC",
        "DNSName": "desktop-abc.tail1234.ts.net.",
        "OS": "windows",
        "TailscaleIPs": ["100.101.102.103", "fd7a:115c:a1e0::1"],
        "Online": true
      },
      "CurrentTailnet": { "Name": "example.github", "MagicDNSSuffix": "tail1234.ts.net" },
      "Peer": {
        "nodekey:aaa": {
          "HostName": "zed-pc",
          "DNSName": "zed-pc.tail1234.ts.net.",
          "OS": "windows",
          "TailscaleIPs": ["fd7a:115c:a1e0::2", "100.64.0.2"],
          "Online": false
        },
        "nodekey:bbb": {
          "HostName": "Amy-Laptop",
          "DNSName": "",
          "OS": "",
          "TailscaleIPs": ["100.64.0.3"],
          "Online": true
        }
      }
    }"#;

    #[test]
    fn parses_running_status() {
        let s = parse_status(SAMPLE.as_bytes()).unwrap();
        assert_eq!(s.state, TailscaleState::Running);
        assert_eq!(s.self_name.as_deref(), Some("desktop-abc"));
        assert_eq!(s.self_ipv4.as_deref(), Some("100.101.102.103"));
        assert_eq!(s.tailnet.as_deref(), Some("example.github"));
        assert_eq!(s.peers.len(), 2);
        // Online first, IPv4 picked even when listed after the IPv6.
        assert_eq!(s.peers[0].name, "Amy-Laptop");
        assert!(s.peers[0].online);
        assert_eq!(s.peers[0].os, None);
        assert_eq!(s.peers[1].ipv4.as_deref(), Some("100.64.0.2"));
    }

    #[test]
    fn logged_out_has_no_address() {
        let raw = r#"{ "BackendState": "NeedsLogin", "Self": { "HostName": "pc", "TailscaleIPs": ["100.1.1.1"] }, "Peer": null }"#;
        let s = parse_status(raw.as_bytes()).unwrap();
        assert_eq!(s.state, TailscaleState::NeedsLogin);
        assert_eq!(s.self_ipv4, None);
        assert!(s.peers.is_empty());
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(parse_status(b"failed to connect to local tailscaled").is_none());
    }
}
