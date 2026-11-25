//! Minimal client for BakkesMod's local RCON websocket
//! (`ws://127.0.0.1:<rcon_port>`).
//!
//! Handshake: the first message is `rcon_password <password>`; BakkesMod
//! answers `authyes` / `authno`. Every later text message is a console
//! command, accepted only if its first word is in
//! `data/rcon_commands.cfg` (see [`super::bakkes_cfg`]). Only loopback is
//! ever contacted.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::base::error::{AppError, AppResult};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const AUTH_TIMEOUT: Duration = Duration::from_secs(3);
/// Leaves BakkesMod time to run one command before the next.
const BETWEEN_COMMANDS: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthReply {
    Accepted,
    Refused,
    /// No (or an unexpected) answer: older builds stay silent.
    Unknown,
}

pub(crate) fn auth_reply(text: &str) -> AuthReply {
    match text.trim().to_ascii_lowercase().as_str() {
        "authyes" => AuthReply::Accepted,
        "authno" => AuthReply::Refused,
        _ => AuthReply::Unknown,
    }
}

fn url(port: u16) -> String {
    format!("ws://127.0.0.1:{port}")
}

/// BakkesMod's RCON answers (it is loaded in a running game).
pub async fn reachable(port: u16) -> bool {
    matches!(
        tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url(port))).await,
        Ok(Ok(_))
    )
}

/// Authenticates and runs `commands` in order.
pub async fn send(port: u16, password: &str, commands: &[String]) -> AppResult<()> {
    let (mut ws, _) =
        tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url(port)))
            .await
            .map_err(|_| AppError::Network("BakkesMod RCON did not answer".into()))?
            .map_err(|e| AppError::Network(format!("BakkesMod RCON: {e}")))?;
    ws.send(Message::Text(format!("rcon_password {password}")))
        .await
        .map_err(|e| AppError::Network(format!("BakkesMod RCON: {e}")))?;
    let reply = match tokio::time::timeout(AUTH_TIMEOUT, ws.next()).await {
        Ok(Some(Ok(Message::Text(t)))) => auth_reply(&t),
        _ => AuthReply::Unknown,
    };
    if reply == AuthReply::Refused {
        return Err(AppError::NotAllowed(
            "BakkesMod refused the RCON password — restart the game so it reloads its config"
                .into(),
        ));
    }
    for command in commands {
        ws.send(Message::Text(command.clone()))
            .await
            .map_err(|e| AppError::Network(format!("BakkesMod RCON: {e}")))?;
        tracing::info!(%command, "sent to BakkesMod");
        tokio::time::sleep(BETWEEN_COMMANDS).await;
    }
    let _ = ws.close(None).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_replies() {
        assert_eq!(auth_reply("authyes"), AuthReply::Accepted);
        assert_eq!(auth_reply(" AUTHNO\n"), AuthReply::Refused);
        assert_eq!(auth_reply("hello"), AuthReply::Unknown);
    }
}
