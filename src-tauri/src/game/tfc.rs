//! Read access to the game's texture caches (`CookedPCConsole/<name>.tfc`).

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::base::{AppError, AppResult};

/// `<name>.tfc` in `cooked_dir`, matched case-insensitively.
pub fn path(cooked_dir: &Path, name: &str) -> Option<PathBuf> {
    let file = format!("{name}.tfc");
    let exact = cooked_dir.join(&file);
    if exact.is_file() {
        return Some(exact);
    }
    std::fs::read_dir(cooked_dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&file))
        .map(|e| e.path())
}

/// `len` bytes at `offset` of texture cache `name`.
pub fn read_range(cooked_dir: &Path, name: &str, offset: u64, len: usize) -> AppResult<Vec<u8>> {
    let p = path(cooked_dir, name).ok_or_else(|| AppError::FileNotFound(format!("{name}.tfc")))?;
    let mut f = std::fs::File::open(&p).map_err(|e| AppError::from_game_io(e, &p))?;
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)
        .map_err(|e| AppError::from_game_io(e, &p))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_range_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Textures2.tfc"), b"0123456789").unwrap();
        assert_eq!(read_range(dir.path(), "textures2", 3, 4).unwrap(), b"3456");
        assert!(read_range(dir.path(), "Textures9", 0, 1).is_err());
        assert!(read_range(dir.path(), "Textures2", 8, 4).is_err());
    }
}
