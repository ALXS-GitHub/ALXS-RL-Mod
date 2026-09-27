//! Name table (FName entries) of a decrypted header region.
//!
//! Layout per entry: `i32 length` (payload bytes incl. NUL; negative =
//! UTF-16 units), payload, `u64 flags`. Renames never change the length
//! prefix: shorter names are NUL-padded in place so every offset after the
//! table stays valid.

use crate::upk::reader::read_i32;

#[derive(Debug, Clone)]
pub struct NameEntry {
    pub index: usize,
    /// Offset of the length prefix inside the region plaintext.
    pub pos: usize,
    /// Offset of the payload.
    pub payload_pos: usize,
    /// Payload width in bytes (slot width available to a rename).
    pub slot_len: usize,
    /// True for UTF-16 entries (never renamed).
    pub wide: bool,
    /// Text without trailing NULs.
    pub name: String,
}

impl NameEntry {
    pub fn end(&self) -> usize {
        self.payload_pos + self.slot_len + 8
    }
}

/// Parses exactly `count` entries (stops early on a malformed entry — the
/// caller can compare lengths to detect a bad table).
pub fn parse(plain: &[u8], count: usize) -> Vec<NameEntry> {
    let mut out = Vec::with_capacity(count.min(65_536));
    let mut pos = 0usize;
    for index in 0..count {
        let Some(len) = read_i32(plain, pos) else {
            break;
        };
        let (slot_len, wide) = match len {
            1..=4096 => (len as usize, false),
            -4096..=-1 => (len.unsigned_abs() as usize * 2, true),
            _ => break,
        };
        let payload_pos = pos + 4;
        let Some(payload) = plain.get(payload_pos..payload_pos + slot_len) else {
            break;
        };
        if plain.len() < payload_pos + slot_len + 8 {
            break;
        }
        let name = if wide {
            let units: Vec<u16> = payload
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
                .trim_end_matches('\0')
                .to_string()
        } else {
            let end = payload.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
            String::from_utf8_lossy(&payload[..end]).into_owned()
        };
        out.push(NameEntry {
            index,
            pos,
            payload_pos,
            slot_len,
            wide,
            name,
        });
        pos = payload_pos + slot_len + 8;
    }
    out
}

#[derive(Debug, Clone, Default)]
pub struct NameTable {
    pub entries: Vec<NameEntry>,
}

impl NameTable {
    pub fn parse(plain: &[u8], count: usize) -> Self {
        Self {
            entries: parse(plain, count),
        }
    }

    pub fn get(&self, index: i32) -> &str {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.entries.get(i))
            .map_or("", |e| e.name.as_str())
    }

    /// Exact (case-sensitive) lookup.
    pub fn find(&self, name: &str) -> Option<&NameEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    pub fn find_ci(&self, name: &str) -> Option<&NameEntry> {
        self.entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
    }

    pub fn index_of(&self, name: &str) -> Option<i32> {
        self.find(name).map(|e| e.index as i32)
    }

    /// Offset just past the last entry (start of whatever follows the table).
    pub fn end(&self) -> usize {
        self.entries.last().map_or(0, NameEntry::end)
    }
}

/// Writes `text` into a slot, NUL-padding to the slot width.
/// Caller guarantees `text.len() + 1 <= slot_len`.
pub fn write_slot(plain: &mut [u8], entry: &NameEntry, text: &str) {
    let bytes = text.as_bytes();
    for (i, b) in plain[entry.payload_pos..entry.payload_pos + entry.slot_len]
        .iter_mut()
        .enumerate()
    {
        *b = bytes.get(i).copied().unwrap_or(0);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn build(names: &[&str]) -> Vec<u8> {
        let mut p = Vec::new();
        for n in names {
            p.extend(((n.len() + 1) as i32).to_le_bytes());
            p.extend(n.as_bytes());
            p.push(0);
            p.extend([0u8; 8]);
        }
        p
    }

    #[test]
    fn parses_and_looks_up() {
        let plain = build(&["Core", "Texture2D", "Textures7"]);
        let t = NameTable::parse(&plain, 3);
        assert_eq!(t.entries.len(), 3);
        assert_eq!(t.get(1), "Texture2D");
        assert_eq!(t.index_of("Textures7"), Some(2));
        assert_eq!(t.end(), plain.len());
        assert_eq!(t.get(-1), "");
    }

    #[test]
    fn padded_names_are_trimmed() {
        let mut plain = build(&["Skin_Octane_GaleFire_RGB"]);
        let t = NameTable::parse(&plain, 1);
        write_slot(&mut plain, &t.entries[0], "Skin_Octane_Stars_RGB");
        let t2 = NameTable::parse(&plain, 1);
        assert_eq!(t2.get(0), "Skin_Octane_Stars_RGB");
        assert_eq!(t2.entries[0].slot_len, t.entries[0].slot_len);
    }

    #[test]
    fn stops_on_garbage() {
        let mut plain = build(&["A"]);
        plain.extend(i32::MAX.to_le_bytes());
        assert_eq!(parse(&plain, 5).len(), 1);
    }
}
