//! Typed application errors.
//!
//! Every Tauri command returns `AppResult<T>`. Errors cross the IPC boundary
//! as `{ kind, message }` so the front-end can branch on `kind` (and localise)
//! instead of parsing messages.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Rocket League installation not found")]
    RocketLeagueNotFound,

    #[error("Rocket League is running — close the game first")]
    GameRunning,

    #[error("AES keys for game packages are missing (keys.txt)")]
    KeysMissing,

    #[error("Game package error: {0}")]
    Upk(String),

    #[error("Not supported with this game build: {0}")]
    Unsupported(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("File is locked or in use: {0}")]
    FileLocked(String),

    /// A map swap hit the arena you are playing on.
    #[error("Arena in use by the game: {0}")]
    ArenaInUse(String),

    #[error("Integrity check failed for {0}")]
    HashMismatch(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Blocked by security policy: {0}")]
    NotAllowed(String),

    /// Online ratings (tracker.gg) are turned off in the tracker settings.
    #[error("Online ratings are turned off")]
    RatingsDisabled,

    #[error("Network error: {0}")]
    Network(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Stable identifier consumed by the front-end (`src/lib/errors.ts`).
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::RocketLeagueNotFound => "RocketLeagueNotFound",
            AppError::GameRunning => "GameRunning",
            AppError::KeysMissing => "KeysMissing",
            AppError::Upk(_) => "Upk",
            AppError::Unsupported(_) => "Unsupported",
            AppError::FileNotFound(_) => "FileNotFound",
            AppError::FileLocked(_) => "FileLocked",
            AppError::ArenaInUse(_) => "ArenaInUse",
            AppError::HashMismatch(_) => "HashMismatch",
            AppError::NotFound(_) => "NotFound",
            AppError::Conflict(_) => "Conflict",
            AppError::InvalidInput(_) => "InvalidInput",
            AppError::NotAllowed(_) => "NotAllowed",
            AppError::RatingsDisabled => "RatingsDisabled",
            AppError::Network(_) => "Network",
            AppError::Io(_) => "Io",
            AppError::Serde(_) => "Serde",
            AppError::Internal(_) => "Internal",
        }
    }

    /// Maps an I/O error on a game file to the more helpful `FileLocked`
    /// when Windows reports a sharing violation (game or launcher holds it).
    pub fn from_game_io(err: std::io::Error, path: &std::path::Path) -> Self {
        // ERROR_SHARING_VIOLATION = 32, ERROR_LOCK_VIOLATION = 33
        match err.raw_os_error() {
            Some(32) | Some(33) => AppError::FileLocked(path.display().to_string()),
            // ERROR_ACCESS_DENIED on a file that exists: replacing or
            // deleting a file another process holds open.
            Some(5) if path.exists() => AppError::FileLocked(path.display().to_string()),
            _ if err.kind() == std::io::ErrorKind::NotFound => {
                AppError::FileNotFound(path.display().to_string())
            }
            _ => AppError::Io(err),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            kind: &'a str,
            message: String,
        }
        Wire {
            kind: self.kind(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        AppError::InvalidInput(format!("archive: {e}"))
    }
}

impl From<image::ImageError> for AppError {
    fn from(e: image::ImageError) -> Self {
        AppError::InvalidInput(format!("image: {e}"))
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
