//! The match tracker overlay: a small transparent always-on-top window
//! (`index.html?window=overlay`) that only listens to `tracker://session`.
//! Its position/size are remembered between sessions.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::base::error::{AppError, AppResult};
use crate::base::{fsx, paths};

pub const LABEL: &str = "overlay";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Placement {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            x: 40.0,
            y: 40.0,
            width: 360.0,
            height: 168.0,
        }
    }
}

fn placement_path() -> AppResult<std::path::PathBuf> {
    Ok(paths::data_subdir("stats")?.join("overlay.json"))
}

pub fn is_open(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL).is_some()
}

pub fn open(app: &AppHandle) -> AppResult<()> {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.show();
        return Ok(());
    }
    let p: Placement = fsx::read_json_or_default(&placement_path()?)?;
    let window = WebviewWindowBuilder::new(
        app,
        LABEL,
        WebviewUrl::App("index.html?window=overlay".into()),
    )
    .title("ALXS-RL-Mod overlay")
    .inner_size(p.width.clamp(240.0, 900.0), p.height.clamp(96.0, 600.0))
    .position(p.x, p.y)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(true)
    .focused(false)
    .build()
    .map_err(|e| AppError::Internal(format!("overlay window: {e}")))?;

    let handle = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
            let scale = handle.scale_factor().unwrap_or(1.0);
            if let (Ok(pos), Ok(size)) = (handle.outer_position(), handle.inner_size()) {
                let placement = Placement {
                    x: f64::from(pos.x) / scale,
                    y: f64::from(pos.y) / scale,
                    width: f64::from(size.width) / scale,
                    height: f64::from(size.height) / scale,
                };
                if let Ok(path) = placement_path() {
                    let _ = fsx::write_json(&path, &placement);
                }
            }
        }
    });
    tracing::info!("overlay opened");
    Ok(())
}

pub fn close(app: &AppHandle) -> AppResult<()> {
    if let Some(win) = app.get_webview_window(LABEL) {
        win.close()
            .map_err(|e| AppError::Internal(format!("overlay close: {e}")))?;
    }
    Ok(())
}
