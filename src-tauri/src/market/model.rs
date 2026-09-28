//! Marketplace data model shared with the UI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MarketSource {
    /// alphaconsole.io — the AlphaConsole community library.
    AlphaConsole,
    /// The RL-Designer GitHub repo, already in the app's format.
    RlDesigner,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MarketKind {
    Decal,
    Ball,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MarketSort {
    #[default]
    Downloads,
    Likes,
    Newest,
    Updated,
    Trending,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MarketItem {
    pub source: MarketSource,
    /// Source id: AlphaConsole item UUID, RL-Designer pack name.
    pub id: String,
    pub kind: MarketKind,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    /// Page of the item on its website (opened in the browser).
    pub page_url: Option<String>,
    pub downloads: Option<u64>,
    pub likes: Option<u64>,
    /// Car bodies (or ball variants) of the pack, when the source lists them.
    pub bodies: Vec<String>,
    /// Those of `bodies` already in the library.
    pub installed_bodies: Vec<String>,
    /// Already in the app's real-colour format: no conversion needed.
    pub ready: bool,
    /// Everything this item offers is already in the library (every listed
    /// variant, or the pack folder when the source lists none).
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPage {
    pub items: Vec<MarketItem>,
    /// Matches on the source (may exceed `items`).
    pub total: Option<u64>,
    /// The source returned only the first results: refine the search.
    pub truncated: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    /// Folder name of the pack in the library.
    pub pack: String,
    /// Variants installed (one per car body or ball).
    pub variants: u32,
    /// Variants converted from the AlphaConsole format.
    pub converted: u32,
    /// Variants skipped because the library already has them.
    pub skipped: u32,
    pub failed: Vec<String>,
}
