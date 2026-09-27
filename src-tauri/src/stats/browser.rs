//! tracker.gg lookups through a hidden, real browser (WebView2).
//!
//! tracker.gg sits behind Cloudflare, which rejects plain HTTP clients. A
//! hidden webview is a genuine browser: it opens the public profile page
//! exactly like the user would, then a small script reads the profile data
//! the page itself uses and hands a trimmed copy back.
//!
//! Isolation:
//! - the page gets **no** Tauri capability (remote origin, no IPC);
//! - navigation is limited to tracker.gg / Cloudflare hosts;
//! - the result comes back by navigating to `https://alxs-result.invalid/…`,
//!   which `on_navigation` intercepts and cancels (never leaves the machine);
//! - the payload is size-capped and parsed as untrusted JSON.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;
use url::Url;

use crate::base::error::{AppError, AppResult};

const LABEL: &str = "tracker-fetch";
const RESULT_HOST: &str = "alxs-result.invalid";
const TIMEOUT: Duration = Duration::from_secs(35);
const MAX_PAYLOAD: usize = 512 * 1024;
const ALLOWED_HOSTS: &[&str] = &["tracker.gg", "tracker.network", "challenges.cloudflare.com"];

/// Pending lookups by request id.
type Pending = Mutex<HashMap<String, oneshot::Sender<Result<String, String>>>>;

fn pending() -> &'static Pending {
    static P: OnceLock<Pending> = OnceLock::new();
    P.get_or_init(|| Mutex::new(HashMap::new()))
}

/// One lookup at a time: the hidden window is shared.
fn gate() -> &'static tokio::sync::Mutex<()> {
    static G: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    G.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// The script injected into the profile page once it has loaded. It keeps
/// retrying while Cloudflare's check runs, then reports back.
fn script(request_id: &str, api_url: &str) -> String {
    let id = serde_json::to_string(request_id).unwrap_or_default();
    let api = serde_json::to_string(api_url).unwrap_or_default();
    format!(
        r#"(() => {{
  if (window.__alxsRunning === {id}) return;
  window.__alxsRunning = {id};
  const report = (ok, payload) => {{
    const q = new URLSearchParams({{ id: {id}, ok: ok ? "1" : "0", data: payload }});
    location.href = "https://{RESULT_HOST}/r?" + q.toString();
  }};
  const trim = (json) => {{
    const d = (json && json.data) || {{}};
    const segs = (d.segments || []).filter((s) => s && s.type === "playlist").map((s) => ({{
      type: "playlist",
      metadata: {{ name: s.metadata && s.metadata.name }},
      stats: {{
        rating: {{ value: s.stats && s.stats.rating && s.stats.rating.value }},
        tier: {{ metadata: {{
          name: s.stats && s.stats.tier && s.stats.tier.metadata && s.stats.tier.metadata.name,
          iconUrl: s.stats && s.stats.tier && s.stats.tier.metadata && s.stats.tier.metadata.iconUrl,
        }} }},
        division: {{ metadata: {{ name: s.stats && s.stats.division && s.stats.division.metadata && s.stats.division.metadata.name }} }},
        matchesPlayed: {{ value: s.stats && s.stats.matchesPlayed && s.stats.matchesPlayed.value }},
      }},
    }}));
    return {{ data: {{ platformInfo: {{ platformUserHandle: d.platformInfo && d.platformInfo.platformUserHandle }}, segments: segs }} }};
  }};
  let attempt = 0;
  const tick = async () => {{
    attempt += 1;
    try {{
      const r = await fetch({api}, {{ credentials: "include", cache: "no-store", headers: {{ Accept: "application/json" }} }});
      if (r.status === 404) return report(false, "404");
      if (r.ok) return report(true, JSON.stringify(trim(await r.json())));
      if (attempt >= 12) return report(false, String(r.status));
    }} catch (e) {{
      if (attempt >= 12) return report(false, "fetch");
    }}
    setTimeout(tick, 2000);
  }};
  tick();
}})();"#
    )
}

fn host_allowed(url: &Url) -> bool {
    url.host_str().is_some_and(|h| {
        let h = h.to_ascii_lowercase();
        ALLOWED_HOSTS
            .iter()
            .any(|a| h == *a || h.ends_with(&format!(".{a}")))
    })
}

/// Handles the result navigation. Returns `false` to cancel it.
fn on_navigation(url: &Url) -> bool {
    if url.host_str() == Some(RESULT_HOST) {
        let q: HashMap<String, String> = url.query_pairs().into_owned().collect();
        if let Some(id) = q.get("id") {
            let tx = pending().lock().ok().and_then(|mut p| p.remove(id));
            if let Some(tx) = tx {
                let data = q.get("data").cloned().unwrap_or_default();
                let ok = q.get("ok").map(String::as_str) == Some("1");
                let _ = tx.send(if ok { Ok(data) } else { Err(data) });
            }
        }
        return false;
    }
    url.scheme() == "https" && host_allowed(url)
}

/// Opens (or reuses) the hidden window on `page_url` and runs the lookup
/// script for `api_url`. Returns the trimmed profile JSON.
pub async fn fetch_profile(
    app: &AppHandle,
    page_url: &Url,
    api_url: &Url,
) -> AppResult<serde_json::Value> {
    let _turn = gate().lock().await;
    let request_id = uuid::Uuid::new_v4().simple().to_string();
    let (tx, rx) = oneshot::channel();
    pending()
        .lock()
        .map_err(|_| AppError::Internal("tracker lookup lock".into()))?
        .insert(request_id.clone(), tx);

    let js = script(&request_id, api_url.as_str());
    match app.get_webview_window(LABEL) {
        Some(win) => {
            // Fresh page for this lookup; the load handler injects the script.
            win.navigate(page_url.clone())
                .map_err(|e| AppError::Internal(format!("tracker window: {e}")))?;
        }
        None => {
            WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(page_url.clone()))
                .title("tracker.gg")
                .visible(false)
                .skip_taskbar(true)
                .focused(false)
                .inner_size(1024.0, 768.0)
                .on_navigation(on_navigation)
                .on_page_load(|win, payload| {
                    if payload.event() == PageLoadEvent::Finished && host_allowed(payload.url()) {
                        if let Some(js) = CURRENT_SCRIPT.lock().ok().and_then(|s| s.clone()) {
                            let _ = win.eval(&js);
                        }
                    }
                })
                .build()
                .map_err(|e| AppError::Internal(format!("tracker window: {e}")))?;
        }
    }
    if let Ok(mut s) = CURRENT_SCRIPT.lock() {
        *s = Some(js.clone());
    }
    // If the page was already loaded (reuse), run the script right away too.
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.eval(&js);
    }

    let outcome = tokio::time::timeout(TIMEOUT, rx).await;
    if let Ok(mut s) = CURRENT_SCRIPT.lock() {
        *s = None;
    }
    let payload = match outcome {
        Ok(Ok(Ok(data))) => data,
        Ok(Ok(Err(code))) if code == "404" => {
            return Err(AppError::NotFound("tracker.gg profile".into()))
        }
        Ok(Ok(Err(code))) => {
            return Err(AppError::Network(format!(
                "tracker.gg did not return the profile ({code})"
            )));
        }
        _ => {
            if let Ok(mut p) = pending().lock() {
                p.remove(&request_id);
            }
            return Err(AppError::Network(
                "tracker.gg did not answer in time".into(),
            ));
        }
    };
    if payload.len() > MAX_PAYLOAD {
        return Err(AppError::Network("tracker.gg answer too large".into()));
    }
    serde_json::from_str(&payload)
        .map_err(|e| AppError::Network(format!("unexpected tracker.gg answer: {e}")))
}

/// Script for the lookup in flight (read by the page-load handler).
static CURRENT_SCRIPT: Mutex<Option<String>> = Mutex::new(None);

/// Closes the hidden window (app exit / tracker disabled).
pub fn close(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_is_limited_to_tracker_hosts() {
        assert!(on_navigation(
            &Url::parse("https://rocketleague.tracker.network/rocket-league/profile/epic/x")
                .unwrap()
        ));
        assert!(on_navigation(
            &Url::parse("https://challenges.cloudflare.com/cdn-cgi/x").unwrap()
        ));
        assert!(!on_navigation(
            &Url::parse("https://evil.example/").unwrap()
        ));
        assert!(!on_navigation(&Url::parse("http://tracker.gg/").unwrap()));
    }

    #[test]
    fn result_navigation_is_captured_and_cancelled() {
        let (tx, mut rx) = oneshot::channel();
        pending().lock().unwrap().insert("abc".into(), tx);
        let url = Url::parse("https://alxs-result.invalid/r?id=abc&ok=1&data=%7B%7D").unwrap();
        assert!(!on_navigation(&url));
        assert_eq!(rx.try_recv().unwrap(), Ok("{}".to_string()));
    }

    #[test]
    fn script_embeds_ids_as_json_strings() {
        let js = script("id\"1", "https://api.tracker.gg/x");
        assert!(js.contains(r#""id\"1""#));
        assert!(js.contains(RESULT_HOST));
    }
}
