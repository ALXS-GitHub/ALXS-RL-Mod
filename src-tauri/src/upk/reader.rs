//! Bounds-checked little-endian reader over a byte slice. Every parser in
//! the engine uses it so a truncated or hostile file yields an error instead
//! of a panic.

use crate::upk::error::{UpkError, UpkResult};

#[derive(Debug, Clone)]
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn at(buf: &'a [u8], pos: usize) -> Self {
        Self { buf, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub fn bytes(&mut self, n: usize, what: &'static str) -> UpkResult<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(UpkError::Truncated(what))?;
        let slice = self
            .buf
            .get(self.pos..end)
            .ok_or(UpkError::Truncated(what))?;
        self.pos = end;
        Ok(slice)
    }

    pub fn skip(&mut self, n: usize, what: &'static str) -> UpkResult<()> {
        self.bytes(n, what).map(|_| ())
    }

    pub fn u8(&mut self, what: &'static str) -> UpkResult<u8> {
        Ok(self.bytes(1, what)?[0])
    }

    pub fn u16(&mut self, what: &'static str) -> UpkResult<u16> {
        let b = self.bytes(2, what)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self, what: &'static str) -> UpkResult<u32> {
        let b = self.bytes(4, what)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&mut self, what: &'static str) -> UpkResult<i32> {
        self.u32(what).map(|v| v as i32)
    }

    pub fn u64(&mut self, what: &'static str) -> UpkResult<u64> {
        let b = self.bytes(8, what)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }

    pub fn i64(&mut self, what: &'static str) -> UpkResult<i64> {
        self.u64(what).map(|v| v as i64)
    }

    /// UE3 `FString`: positive length = ANSI bytes incl. NUL, negative =
    /// UTF-16 code units incl. NUL.
    pub fn fstring(&mut self, what: &'static str) -> UpkResult<String> {
        let len = self.i32(what)?;
        if len == 0 {
            return Ok(String::new());
        }
        if len > 0 {
            let raw = self.bytes(len as usize, what)?;
            let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
            Ok(String::from_utf8_lossy(&raw[..end]).into_owned())
        } else {
            let units = len.unsigned_abs() as usize;
            let raw = self.bytes(units * 2, what)?;
            let utf16: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&utf16)
                .trim_end_matches('\0')
                .to_string())
        }
    }
}

/// Unchecked-free helpers for fixed offsets (return `None` when out of range).
pub fn read_u32(buf: &[u8], pos: usize) -> Option<u32> {
    buf.get(pos..pos.checked_add(4)?)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn read_i32(buf: &[u8], pos: usize) -> Option<i32> {
    read_u32(buf, pos).map(|v| v as i32)
}

pub fn read_u64(buf: &[u8], pos: usize) -> Option<u64> {
    let b = buf.get(pos..pos.checked_add(8)?)?;
    let mut a = [0u8; 8];
    a.copy_from_slice(b);
    Some(u64::from_le_bytes(a))
}

pub fn read_f32(buf: &[u8], pos: usize) -> Option<f32> {
    read_u32(buf, pos).map(f32::from_bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_and_fails_cleanly() {
        let data = [1u8, 0, 0, 0, 0xff];
        let mut r = Reader::new(&data);
        assert_eq!(r.u32("a").unwrap(), 1);
        assert!(r.u32("b").is_err());
        assert_eq!(read_i32(&[0xff, 0xff, 0xff, 0xff], 0), Some(-1));
        assert_eq!(read_u32(&data, 3), None);
    }

    #[test]
    fn reads_ansi_and_utf16_fstrings() {
        let mut ansi = 5i32.to_le_bytes().to_vec();
        ansi.extend(b"None\0");
        assert_eq!(Reader::new(&ansi).fstring("s").unwrap(), "None");

        let mut wide = (-3i32).to_le_bytes().to_vec();
        for u in "hé\0".encode_utf16() {
            wide.extend(u.to_le_bytes());
        }
        assert_eq!(Reader::new(&wide).fstring("s").unwrap(), "hé");
    }
}
