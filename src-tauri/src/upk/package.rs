//! A cooked package opened in memory: summary, decrypted header tables,
//! and (on demand) the decompressed body.

use crate::upk::chunks::{self, ChunkMap};
use crate::upk::crypto::{self, HeaderRegion};
use crate::upk::error::{UpkError, UpkResult};
use crate::upk::keys::KeyRing;
use crate::upk::names::NameTable;
use crate::upk::reader::{read_i32, read_u32};
use crate::upk::summary::PackageSummary;
use crate::upk::tables::{self, Export, Import};
use crate::upk::texture::{self, Texture2D};

pub struct Package {
    pub bytes: Vec<u8>,
    pub summary: PackageSummary,
    pub header: HeaderRegion,
    pub names: NameTable,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
}

impl Package {
    pub fn open(bytes: Vec<u8>, ring: &KeyRing) -> UpkResult<Self> {
        let summary = PackageSummary::parse(&bytes)?;
        let header = crypto::open_header(&bytes, &summary, ring)?;
        let names = NameTable::parse(&header.plain, summary.name_count);
        if names.entries.len() != summary.name_count {
            return Err(UpkError::Format(format!(
                "name table: parsed {} of {} entries",
                names.entries.len(),
                summary.name_count
            )));
        }
        let imports = tables::parse_imports(&header, &summary, &names);
        let exports = tables::parse_exports(&header, &summary, &names);
        Ok(Self {
            bytes,
            summary,
            header,
            names,
            imports,
            exports,
        })
    }

    pub fn class_of(&self, export: &Export) -> &str {
        tables::class_name(export, &self.imports, &self.exports)
    }

    /// Re-reads the name table after edits to `header.plain`.
    pub fn refresh_names(&mut self) {
        self.names = NameTable::parse(&self.header.plain, self.summary.name_count);
    }

    /// Decompresses the body (everything after `TotalHeaderSize`).
    pub fn body(&self) -> UpkResult<Body> {
        let map = ChunkMap::read_from(&self.bytes, self.summary.total_header_size)?;
        let data = chunks::decompress_all(&self.bytes, &map)?;
        let segments = chunk_table(&self.header.plain, &map);
        let first_export = self
            .exports
            .iter()
            .filter(|e| e.serial_size > 0)
            .map(|e| e.serial_offset)
            .min();
        let origin = body_origin(self.summary.total_header_size, first_export);
        let mut body = Body {
            map,
            data,
            preamble: 0,
            origin,
            segments,
        };
        if body.segments.is_empty() {
            body.preamble = body.detect_preamble(self);
        }
        Ok(body)
    }

    /// Every `Texture2D` export that parses.
    pub fn textures(&self, body: &Body) -> Vec<Texture2D> {
        self.exports
            .iter()
            .filter(|e| self.class_of(e) == "Texture2D")
            .filter_map(|e| {
                let pos = body.export_pos(e)?;
                match texture::parse_texture(
                    &body.data,
                    pos,
                    e.serial_size,
                    e.index,
                    &e.object_name,
                    &self.names,
                ) {
                    Ok(t) => Some(t),
                    Err(err) => {
                        tracing::debug!(texture = %e.object_name, %err, "texture skipped");
                        None
                    }
                }
            })
            .collect()
    }

    /// The package bytes with the (re-encrypted) header region spliced in.
    pub fn header_bytes(&self) -> UpkResult<Vec<u8>> {
        let mut out = self.bytes.clone();
        self.header.splice_into(&mut out)?;
        Ok(out)
    }
}

pub struct Body {
    pub map: ChunkMap,
    pub data: Vec<u8>,
    /// Bytes at the start of the decompressed body that precede export
    /// data (RL's cooker duplicates the tail of the header region there).
    pub preamble: usize,
    /// File offset that maps to `data[preamble]` (usually TotalHeaderSize).
    pub origin: usize,
    /// Where each chunk sits in the logical (uncompressed) file, from the
    /// package's chunk table. Empty when the table was not found: positions
    /// then fall back to `origin` + `preamble`.
    pub segments: Vec<Segment>,
}

/// One compressed chunk seen from the logical file: `len` bytes starting at
/// file offset `file_offset` are stored at `stream` in [`Body::data`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub file_offset: usize,
    pub len: usize,
    pub stream: usize,
}

/// Entry layouts of the compressed-chunk table: stride, then the offsets of
/// `UncompressedOffset`, `UncompressedSize` and `CompressedOffset` (the
/// offsets are 64-bit in Rocket League's 36-byte entries).
const CHUNK_ENTRY_LAYOUTS: &[(usize, usize, usize, usize, bool)] =
    &[(36, 0, 8, 12, true), (16, 0, 4, 8, false)];

/// Finds the package's `FCompressedChunk` table in the decrypted header and
/// maps every chunk of `map` to its logical file offset. The table is only
/// accepted if every entry points at the matching chunk header on disk and
/// declares its exact decompressed size.
fn chunk_table(plain: &[u8], map: &ChunkMap) -> Vec<Segment> {
    let n = map.chunks.len();
    let Some(first) = map.chunks.first().map(|c| c.header_offset) else {
        return Vec::new();
    };
    let offset_at = |pos: usize, wide: bool| -> Option<usize> {
        if wide {
            let lo = read_u32(plain, pos)? as u64;
            let hi = read_u32(plain, pos + 4)? as u64;
            usize::try_from(lo | (hi << 32)).ok()
        } else {
            read_u32(plain, pos).map(|v| v as usize)
        }
    };
    for &(stride, u_off, u_size, c_off, wide) in CHUNK_ENTRY_LAYOUTS {
        let needle = first as u32;
        for at in 4..plain.len().saturating_sub(stride * n) {
            if read_u32(plain, at + c_off) != Some(needle)
                || read_u32(plain, at - 4).map(|c| c as usize) != Some(n)
            {
                continue;
            }
            let mut stream = 0usize;
            let mut out = Vec::with_capacity(n);
            for (k, chunk) in map.chunks.iter().enumerate() {
                let e = at + k * stride;
                let len: usize = chunk.blocks.iter().map(|b| b.u_size).sum();
                let ok = offset_at(e + c_off, wide) == Some(chunk.header_offset)
                    && read_u32(plain, e + u_size).map(|v| v as usize) == Some(len);
                let Some(file_offset) = offset_at(e + u_off, wide).filter(|_| ok) else {
                    out.clear();
                    break;
                };
                out.push(Segment {
                    file_offset,
                    len,
                    stream,
                });
                stream += len;
            }
            if out.len() == n {
                return out;
            }
        }
    }
    Vec::new()
}

/// File offset that corresponds to the start of the decompressed body.
///
/// Export offsets are normally absolute (≥ TotalHeaderSize). Some small
/// cooked packages (item thumbnails `*_T_SF.upk`) store offsets *below* the
/// header size, relative to their first export — take whichever is lower.
fn body_origin(total_header_size: usize, first_export_offset: Option<u64>) -> usize {
    first_export_offset
        .and_then(|o| usize::try_from(o).ok())
        .map_or(total_header_size, |o| o.min(total_header_size))
}

impl Body {
    /// Position of an export's serial data inside `data`.
    pub fn export_pos(&self, export: &Export) -> Option<usize> {
        if !self.segments.is_empty() {
            let off = usize::try_from(export.serial_offset).ok()?;
            let seg = self
                .segments
                .iter()
                .find(|s| (s.file_offset..s.file_offset + s.len).contains(&off))?;
            let pos = seg.stream + (off - seg.file_offset);
            return (pos + 8 <= self.data.len()).then_some(pos);
        }
        let base = usize::try_from(export.serial_offset)
            .ok()?
            .checked_sub(self.origin)?;
        let pos = base + self.preamble;
        (pos + 8 <= self.data.len()).then_some(pos)
    }

    /// Votes on the shift that makes the most exports start with
    /// `NetIndex = -1` followed by a real property tag.
    fn detect_preamble(&self, pkg: &Package) -> usize {
        const MAX_SHIFT: usize = 65_536;
        let names = &pkg.names;
        let is_tag = |pos: usize| -> bool {
            let (Some(net), Some(name), Some(ty)) = (
                read_i32(&self.data, pos),
                read_i32(&self.data, pos + 4),
                read_i32(&self.data, pos + 12),
            ) else {
                return false;
            };
            net == -1 && !names.get(name).is_empty() && names.get(ty).ends_with("Property")
        };
        let mut votes: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
        for e in pkg.exports.iter().filter(|e| e.serial_size >= 16).take(64) {
            let Some(base) = usize::try_from(e.serial_offset)
                .ok()
                .and_then(|o| o.checked_sub(self.origin))
            else {
                continue;
            };
            for k in 0..MAX_SHIFT {
                if base + k + 16 > self.data.len() {
                    break;
                }
                if is_tag(base + k) {
                    *votes.entry(k).or_default() += 1;
                }
            }
        }
        // Most votes wins; BTreeMap iteration makes ties resolve to the smallest shift.
        votes
            .iter()
            .fold(
                (0usize, 0usize),
                |best, (&k, &v)| if v > best.1 { (k, v) } else { best },
            )
            .0
    }
}

#[cfg(test)]
mod tests {
    use super::{body_origin, chunk_table, Segment};
    use crate::upk::chunks::{wrap_chunked, ChunkMap};

    /// Two chunks on disk after a 100-byte header; the table in the
    /// decrypted header says they hold file offsets 40.. and 40+len0...
    #[test]
    fn chunk_table_maps_chunks_to_file_offsets() {
        let first = wrap_chunked(&[1u8; 300]).unwrap();
        let second = wrap_chunked(&[2u8; 50]).unwrap();
        let mut file = vec![0u8; 100];
        file.extend(&first);
        file.extend(&second);
        let map = ChunkMap::read_from(&file, 100).unwrap();
        assert_eq!(map.chunks.len(), 2);

        let mut plain = vec![0xAAu8; 24];
        plain.extend(2u32.to_le_bytes());
        for (u_off, u_size, c_off, c_size) in [
            (40u64, 300u32, 100u64, first.len() as u32),
            (340, 50, 100 + first.len() as u64, second.len() as u32),
        ] {
            plain.extend(u_off.to_le_bytes());
            plain.extend(u_size.to_le_bytes());
            plain.extend(c_off.to_le_bytes());
            plain.extend(c_size.to_le_bytes());
            plain.extend([0u8; 12]);
        }
        plain.extend([0u8; 16]);
        assert_eq!(
            chunk_table(&plain, &map),
            vec![
                Segment {
                    file_offset: 40,
                    len: 300,
                    stream: 0
                },
                Segment {
                    file_offset: 340,
                    len: 50,
                    stream: 300
                },
            ]
        );
        // A table that disagrees with the chunks on disk is ignored.
        plain[28 + 8] = 7;
        assert!(chunk_table(&plain, &map).is_empty());
    }

    #[test]
    fn origin_is_header_size_for_regular_packages() {
        assert_eq!(body_origin(18_153, Some(20_000)), 18_153);
        assert_eq!(body_origin(18_153, None), 18_153);
    }

    #[test]
    fn origin_follows_relative_offsets_of_thumbnail_packages() {
        // wheel_Amber_T_SF.upk: header 18153, first export at 1769.
        assert_eq!(body_origin(18_153, Some(1_769)), 1_769);
    }
}
