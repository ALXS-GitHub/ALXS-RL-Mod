//! Small filesystem helpers shared by every module: hashing and atomic writes.
//!
//! Atomic = write to a sibling temp file, fsync, then rename over the target.
//! A crash mid-write can never leave a half-written game package behind.

use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::base::error::{AppError, AppResult};

pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn sha256_file(path: &Path) -> AppResult<String> {
    let file = File::open(path).map_err(|e| AppError::from_game_io(e, path))?;
    let mut reader = BufReader::with_capacity(1 << 20, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Hash of the file if it exists, `None` otherwise.
pub fn sha256_file_opt(path: &Path) -> AppResult<Option<String>> {
    if path.is_file() {
        sha256_file(path).map(Some)
    } else {
        Ok(None)
    }
}

fn temp_sibling(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    target.with_file_name(format!(".{name}.alxs-tmp"))
}

pub fn write_atomic(target: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = temp_sibling(target);
    {
        let mut f = File::create(&tmp).map_err(|e| AppError::from_game_io(e, &tmp))?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        AppError::from_game_io(e, target)
    })
}

pub fn copy_atomic(source: &Path, target: &Path) -> AppResult<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = temp_sibling(target);
    std::fs::copy(source, &tmp).map_err(|e| AppError::from_game_io(e, source))?;
    std::fs::rename(&tmp, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        AppError::from_game_io(e, target)
    })
}

/// Serialises `value` as pretty JSON and writes it atomically.
pub fn write_json<T: serde::Serialize>(target: &Path, value: &T) -> AppResult<()> {
    let raw = serde_json::to_vec_pretty(value)?;
    write_atomic(target, &raw)
}

/// Reads JSON; a missing file yields `T::default()`. A corrupt file is kept
/// aside (`*.corrupt`) and replaced by the default rather than crashing.
pub fn read_json_or_default<T>(path: &Path) -> AppResult<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    match std::fs::read(path) {
        Ok(raw) => match serde_json::from_slice(&raw) {
            Ok(v) => Ok(v),
            Err(err) => {
                tracing::warn!(?path, %err, "corrupt JSON state, resetting");
                let _ = std::fs::rename(path, path.with_extension("json.corrupt"));
                Ok(T::default())
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}

/// Keeps only characters that are safe in a Windows file name.
pub fn sanitize_file_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "unnamed".into()
    } else {
        trimmed.chars().take(120).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a").join("b.bin");
        write_atomic(&p, b"hello").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"hello");
        assert_eq!(sha256_file(&p).unwrap(), sha256_bytes(b"hello"));
    }

    #[test]
    fn sanitize_strips_reserved_chars() {
        assert_eq!(sanitize_file_name("a<b>:c/d?.upk"), "a_b__c_d_.upk");
        assert_eq!(sanitize_file_name("..."), "unnamed");
    }
}
