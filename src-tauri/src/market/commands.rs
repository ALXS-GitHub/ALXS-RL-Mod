//! IPC commands of the marketplace (names fixed by docs/ARCHITECTURE.md).

use crate::base::error::{AppError, AppResult};
use crate::market::model::{InstallReport, MarketKind, MarketPage, MarketSort, MarketSource};
use crate::market::{alphaconsole, install, rl_designer};

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn market_browse(
    source: MarketSource,
    kind: MarketKind,
    query: Option<String>,
    sort: Option<MarketSort>,
) -> AppResult<MarketPage> {
    let query = query.unwrap_or_default();
    let mut page = match source {
        MarketSource::AlphaConsole => {
            alphaconsole::browse(kind, &query, sort.unwrap_or_default()).await?
        }
        MarketSource::RlDesigner => rl_designer::browse(kind, &query).await?,
    };
    install::mark_installed(&mut page.items);
    Ok(page)
}

/// Downloads a pack and adds it to the decal or ball library. `convert`
/// turns AlphaConsole decal packs into real-colour (hybrid) packs.
#[tauri::command]
pub async fn market_install(
    source: MarketSource,
    kind: MarketKind,
    id: String,
    title: String,
    convert: bool,
) -> AppResult<InstallReport> {
    let staging = install::new_staging()?;
    let result = async {
        match source {
            MarketSource::AlphaConsole => {
                let bytes = alphaconsole::download(&id).await?;
                let dest = staging.clone();
                blocking(move || install::extract_zip(&bytes, &dest)).await?;
            }
            MarketSource::RlDesigner => rl_designer::download_pack(kind, &id, &staging).await?,
        }
        let src = staging.clone();
        blocking(move || install::install(&src, kind, &title, convert)).await
    }
    .await;
    let _ = std::fs::remove_dir_all(&staging);
    result
}
