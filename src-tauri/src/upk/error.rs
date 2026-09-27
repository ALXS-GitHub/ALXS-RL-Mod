//! Errors of the cooked-package engine. Converted to [`AppError`] at the
//! module boundary (`KeysMissing` keeps its own kind so the UI can guide the
//! user; everything else becomes `AppError::Upk`).

use thiserror::Error;

use crate::base::AppError;

#[derive(Debug, Error)]
pub enum UpkError {
    #[error("not a cooked Rocket League package (bad magic)")]
    BadMagic,

    #[error("package truncated while reading {0}")]
    Truncated(&'static str),

    #[error("no AES key available (keys.txt missing)")]
    KeysMissing,

    #[error("no AES key matches this package")]
    NoKeyMatched,

    #[error("name {0:?} not found in the package name table")]
    NameNotFound(String),

    #[error("rename {from:?} -> {to:?}: target longer than the source slot")]
    TargetTooLong { from: String, to: String },

    #[error(
        "no name slot is long enough for {target:?} ({needed} bytes needed, largest {largest})"
    )]
    NoFit {
        target: String,
        needed: usize,
        largest: usize,
    },

    #[error("zlib: {0}")]
    Zlib(String),

    #[error("recompressed block does not fit its on-disk budget ({new} > {budget} bytes)")]
    BudgetExceeded { new: usize, budget: usize },

    #[error("texture: {0}")]
    Texture(String),

    #[error("unexpected package layout: {0}")]
    Format(String),

    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
}

impl From<UpkError> for AppError {
    fn from(err: UpkError) -> Self {
        match err {
            UpkError::KeysMissing => AppError::KeysMissing,
            UpkError::Io(e) => AppError::Io(e),
            other => AppError::Upk(other.to_string()),
        }
    }
}

pub type UpkResult<T> = Result<T, UpkError>;
