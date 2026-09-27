//! Read-only BakkesMod detection. BakkesMod injects into the game, which
//! EAC forbids: it only works when the game is launched without EAC
//! (offline). This app never starts or injects it.

use std::path::PathBuf;

use serde::Serialize;

use crate::base::paths;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BakkesPlugin {
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BakkesStatus {
    pub installed: bool,
    /// `%APPDATA%\bakkesmod\bakkesmod`
    pub data_dir: Option<String>,
    pub injector_path: Option<String>,
    pub plugins: Vec<BakkesPlugin>,
    /// AlphaConsole data folder present (decal packs can be imported).
    pub alpha_console_data: bool,
    /// Workshop maps folder, if the user kept one (importable in Maps).
    pub workshop_dir: Option<String>,
}

fn injector_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(pf) = std::env::var_os(var) {
            out.push(PathBuf::from(pf).join("BakkesMod").join("BakkesMod.exe"));
        }
    }
    if let Some(local) = paths::local_dir() {
        out.push(
            local
                .join("Programs")
                .join("BakkesMod")
                .join("BakkesMod.exe"),
        );
    }
    out
}

pub fn status() -> BakkesStatus {
    let data_dir = paths::roaming_dir()
        .map(|r| r.join("bakkesmod").join("bakkesmod"))
        .filter(|d| d.is_dir());
    let plugins = data_dir
        .as_ref()
        .and_then(|d| std::fs::read_dir(d.join("plugins")).ok())
        .map(|entries| {
            let mut list: Vec<BakkesPlugin> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
                })
                .filter_map(|p| {
                    Some(BakkesPlugin {
                        name: p.file_stem()?.to_string_lossy().into_owned(),
                        size_bytes: p.metadata().ok()?.len(),
                    })
                })
                .collect();
            list.sort_by_key(|a| a.name.to_lowercase());
            list
        })
        .unwrap_or_default();
    let injector_path = injector_candidates().into_iter().find(|p| p.is_file());
    BakkesStatus {
        installed: data_dir.is_some() || injector_path.is_some(),
        alpha_console_data: data_dir
            .as_ref()
            .is_some_and(|d| d.join("data").join("acplugin").is_dir()),
        workshop_dir: data_dir
            .as_ref()
            .map(|d| d.join("Workshop"))
            .filter(|w| w.is_dir())
            .map(|w| w.to_string_lossy().into_owned()),
        data_dir: data_dir.map(|d| d.to_string_lossy().into_owned()),
        injector_path: injector_path.map(|p| p.to_string_lossy().into_owned()),
        plugins,
    }
}
