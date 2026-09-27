//! Cheap identity of the installed game build.
//!
//! `Engine.upk` is rewritten by every game update, so `size:mtime` of that
//! file changes with each build without us having to hash gigabytes.

use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::game::install::RlInstall;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildFingerprint {
    pub engine_size: u64,
    pub engine_mtime: u64,
}

impl BuildFingerprint {
    /// Compact form stored alongside anything we write (`"size:mtime"`).
    pub fn id(&self) -> String {
        format!("{}:{}", self.engine_size, self.engine_mtime)
    }
}

pub fn current(install: &RlInstall) -> Option<BuildFingerprint> {
    let meta = std::fs::metadata(install.cooked_dir.join("Engine.upk")).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(BuildFingerprint {
        engine_size: meta.len(),
        engine_mtime: mtime,
    })
}
