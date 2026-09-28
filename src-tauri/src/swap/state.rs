//! Persisted swap state: active swaps, history, and the last "owned" item
//! used per slot (what the player equips in game — reused by random presets).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};
use crate::catalog::Slot;

pub const HISTORY_CAP: usize = 300;

/// Serialises every read-modify-write of the state file.
pub static STATE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    pub slot: Slot,
    pub owned_id: u32,
    pub wanted_id: u32,
    /// An official paint (the game's PaintID, see `swap::paint`).
    /// `None`/0 = unpainted.
    #[serde(default)]
    pub paint: Option<u8>,
    /// Custom colour (`#rrggbb`, experimental): every data colour of the
    /// shown item recoloured towards it. See `upk::recolor`.
    #[serde(default)]
    pub color: Option<String>,
    /// Earlier prototype: a hue in degrees. Read only, turned into `color`.
    #[serde(default, skip_serializing)]
    pub tint: Option<u16>,
}

impl SwapRequest {
    /// Turns a legacy `tint` (hue) into `color`, which is what gets saved.
    pub fn migrate(&mut self) {
        if let Some(h) = self.tint.take() {
            if self.color.is_none() {
                self.color = Some(crate::upk::recolor::Target::hue(f32::from(h)).to_hex());
            }
        }
    }

    /// The custom colour to apply, including a legacy `tint`.
    pub fn custom_color(&self) -> Option<crate::upk::recolor::Target> {
        match (&self.color, self.tint) {
            (Some(hex), _) => crate::upk::recolor::Target::from_hex(hex),
            (None, Some(h)) => Some(crate::upk::recolor::Target::hue(f32::from(h))),
            (None, None) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSwap {
    pub id: String,
    pub request: SwapRequest,
    pub owned_label: String,
    pub wanted_label: String,
    /// Package overwritten in the install (the owned item's file).
    pub target_package: String,
    /// Package whose content now lives in `target_package`.
    pub source_package: String,
    /// A painted variant package was found and used.
    pub painted: bool,
    pub applied_at: chrono::DateTime<chrono::Utc>,
    pub build: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SwapEventKind {
    Applied,
    Restored,
    /// Rebuilt from current stock files after a game update / verify.
    Reapplied,
    /// Could not be rebuilt; the swap was removed.
    Dropped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapEvent {
    pub at: chrono::DateTime<chrono::Utc>,
    pub kind: SwapEventKind,
    pub swap_id: String,
    pub slot: Slot,
    pub owned_label: String,
    pub wanted_label: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SwapState {
    pub active: Vec<ActiveSwap>,
    pub history: Vec<SwapEvent>,
    pub owned_defaults: BTreeMap<Slot, u32>,
}

impl SwapState {
    pub fn push_event(&mut self, swap: &ActiveSwap, kind: SwapEventKind, detail: Option<String>) {
        self.history.push(SwapEvent {
            at: chrono::Utc::now(),
            kind,
            swap_id: swap.id.clone(),
            slot: swap.request.slot,
            owned_label: swap.owned_label.clone(),
            wanted_label: swap.wanted_label.clone(),
            detail,
        });
        if self.history.len() > HISTORY_CAP {
            let excess = self.history.len() - HISTORY_CAP;
            self.history.drain(..excess);
        }
    }
}

fn state_path() -> AppResult<PathBuf> {
    Ok(paths::data_subdir("swap")?.join("state.json"))
}

pub fn load() -> AppResult<SwapState> {
    let mut state: SwapState = fsx::read_json_or_default(&state_path()?)?;
    for swap in &mut state.active {
        swap.request.migrate();
    }
    Ok(state)
}

pub fn save(state: &SwapState) -> AppResult<()> {
    fsx::write_json(&state_path()?, state)
}

/// Runs `f` on the state under the lock and persists the result.
pub fn update<T>(f: impl FnOnce(&mut SwapState) -> AppResult<T>) -> AppResult<T> {
    let _guard = STATE_LOCK
        .lock()
        .map_err(|_| AppError::Internal("swap state lock poisoned".into()))?;
    let mut state = load()?;
    let out = f(&mut state)?;
    save(&state)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_roundtrips_with_enum_keys() {
        let mut s = SwapState::default();
        s.owned_defaults.insert(Slot::Wheels, 42);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"wheels\":42"));
        let back: SwapState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.owned_defaults.get(&Slot::Wheels), Some(&42));
    }

    #[test]
    fn history_is_capped() {
        let mut s = SwapState::default();
        let swap = ActiveSwap {
            id: "x".into(),
            request: SwapRequest {
                slot: Slot::Boost,
                owned_id: 1,
                wanted_id: 2,
                paint: None,
                color: None,
                tint: None,
            },
            owned_label: "a".into(),
            wanted_label: "b".into(),
            target_package: "a_SF.upk".into(),
            source_package: "b_SF.upk".into(),
            painted: false,
            applied_at: chrono::Utc::now(),
            build: String::new(),
        };
        for _ in 0..(HISTORY_CAP + 10) {
            s.push_event(&swap, SwapEventKind::Applied, None);
        }
        assert_eq!(s.history.len(), HISTORY_CAP);
    }
}
