//! Outbound network policy.
//!
//! The app is local-first: the only network calls are explicit user actions
//! against a short allowlist (map sources, MMR lookup). Every request goes
//! through [`http_client`] + [`ensure_allowed`] so the policy lives in one
//! place and can be audited at a glance.

use std::time::Duration;

use url::Url;

use crate::base::error::{AppError, AppResult};

/// What a host is allowed to be used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Community map listings and downloads.
    Maps,
    /// Public MMR lookup (read-only, no login).
    Ratings,
    /// Opening a link in the user's browser.
    Browse,
}

const MAP_HOSTS: &[&str] = &["bakkesplugins.com", "lethamyr.com", "rocketleaguemaps.us"];
const RATING_HOSTS: &[&str] = &[
    "api.tracker.gg",
    "tracker.gg",
    "rocketleague.tracker.network",
];
const BROWSE_HOSTS: &[&str] = &[
    "rocketleague.com",
    "bakkesplugins.com",
    "lethamyr.com",
    "tracker.gg",
    "rocketleague.tracker.network",
    "ballchasing.com",
    "github.com",
];

fn hosts_for(purpose: Purpose) -> &'static [&'static str] {
    match purpose {
        Purpose::Maps => MAP_HOSTS,
        Purpose::Ratings => RATING_HOSTS,
        Purpose::Browse => BROWSE_HOSTS,
    }
}

/// Identifies the app to the few services it talks to.
pub const USER_AGENT: &str = concat!("ALXS-RL-Mod/", env!("CARGO_PKG_VERSION"));

/// Validates `raw` for `purpose`: HTTPS only, no credentials, host (or a
/// sub-domain of it) in the allowlist, no IP literals.
pub fn ensure_allowed(raw: &str, purpose: Purpose) -> AppResult<Url> {
    let url = Url::parse(raw).map_err(|e| AppError::InvalidInput(format!("url {raw}: {e}")))?;
    if url.scheme() != "https" {
        return Err(AppError::NotAllowed(format!("non-HTTPS url: {raw}")));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::NotAllowed("credentials in url".into()));
    }
    let host = match url.host() {
        Some(url::Host::Domain(d)) => d.to_ascii_lowercase(),
        _ => return Err(AppError::NotAllowed(format!("host not allowed: {raw}"))),
    };
    let ok = hosts_for(purpose)
        .iter()
        .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")));
    if !ok {
        return Err(AppError::NotAllowed(format!(
            "host not allowed for {purpose:?}: {host}"
        )));
    }
    Ok(url)
}

/// Shared HTTP client: rustls, timeouts, no redirects to other hosts
/// (redirects are re-validated against the same purpose).
pub fn http_client(purpose: Purpose) -> AppResult<reqwest::Client> {
    let policy = reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() > 5 {
            return attempt.error("too many redirects");
        }
        match ensure_allowed(attempt.url().as_str(), purpose) {
            Ok(_) => attempt.follow(),
            Err(_) => attempt.stop(),
        }
    });
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .redirect(policy)
        .build()
        .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_allowlisted_https() {
        assert!(ensure_allowed("https://bakkesplugins.com/maps", Purpose::Maps).is_ok());
        assert!(ensure_allowed("https://cdn.bakkesplugins.com/x.zip", Purpose::Maps).is_ok());
    }

    #[test]
    fn rejects_everything_else() {
        assert!(ensure_allowed("http://bakkesplugins.com", Purpose::Maps).is_err());
        assert!(ensure_allowed("https://evil-bakkesplugins.com", Purpose::Maps).is_err());
        assert!(ensure_allowed("https://127.0.0.1/", Purpose::Maps).is_err());
        assert!(ensure_allowed("https://u:p@bakkesplugins.com", Purpose::Maps).is_err());
        assert!(ensure_allowed("https://api.tracker.gg/x", Purpose::Maps).is_err());
    }
}
