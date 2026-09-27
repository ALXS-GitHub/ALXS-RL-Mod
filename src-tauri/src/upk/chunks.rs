//! Rocket League's chunked-zlib storage.
//!
//! A package body (and, for some packages, part of the header) is a
//! sequence of chunks:
//!
//! ```text
//! u32 magic (0x9E2A83C1) · u32 block_size (128 KiB) · u32 c_total · u32 u_total
//! N × (u32 c_size, u32 u_size)          N = ceil(u_total / block_size)
//! N concatenated zlib streams
//! ```
//!
//! Decompressing every block in order yields the *stream*. We patch game
//! files surgically: decompress one block, edit it, recompress, and pad
//! with zeros up to the original compressed size so no offset in the file
//! moves (RL's zlib ignores trailing bytes after the end of stream).

use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::upk::error::{UpkError, UpkResult};
use crate::upk::reader::read_u32;
use crate::upk::summary::PACKAGE_MAGIC;

pub const DEFAULT_BLOCK_SIZE: u32 = 131_072;
const MAX_BLOCKS_PER_CHUNK: usize = 1 << 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// File offset of the block's compressed bytes.
    pub file_offset: usize,
    pub c_size: usize,
    pub u_size: usize,
    /// Offset of the block's first decompressed byte in the stream.
    pub stream_offset: usize,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub header_offset: usize,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Default)]
pub struct ChunkMap {
    pub chunks: Vec<Chunk>,
}

/// Parses one chunk header at `pos`, validating every size. `stream_offset`
/// is the stream position of the chunk's first byte.
fn parse_chunk(buf: &[u8], pos: usize, stream_offset: usize) -> Option<(Chunk, usize)> {
    if read_u32(buf, pos)? != PACKAGE_MAGIC {
        return None;
    }
    let block_size = read_u32(buf, pos + 4)? as usize;
    let c_total = read_u32(buf, pos + 8)? as usize;
    let u_total = read_u32(buf, pos + 12)? as usize;
    if !(4096..=(1 << 22)).contains(&block_size)
        || !block_size.is_power_of_two()
        || u_total == 0
        || c_total == 0
    {
        return None;
    }
    let n = u_total.div_ceil(block_size);
    if n > MAX_BLOCKS_PER_CHUNK {
        return None;
    }
    let metas = pos + 16;
    let mut data = metas.checked_add(n * 8)?;
    let (mut c_sum, mut u_sum) = (0usize, 0usize);
    let mut blocks = Vec::with_capacity(n);
    let mut stream = stream_offset;
    for i in 0..n {
        let c = read_u32(buf, metas + i * 8)? as usize;
        let u = read_u32(buf, metas + i * 8 + 4)? as usize;
        if c == 0 || u == 0 || u > block_size {
            return None;
        }
        blocks.push(Block {
            file_offset: data,
            c_size: c,
            u_size: u,
            stream_offset: stream,
        });
        data = data.checked_add(c)?;
        stream += u;
        c_sum += c;
        u_sum += u;
    }
    if c_sum != c_total || u_sum != u_total || data > buf.len() {
        return None;
    }
    Some((
        Chunk {
            header_offset: pos,
            blocks,
        },
        data,
    ))
}

impl ChunkMap {
    /// Contiguous chunks starting exactly at `start` (a package body starts
    /// at `TotalHeaderSize`).
    pub fn read_from(buf: &[u8], start: usize) -> UpkResult<Self> {
        let mut map = ChunkMap::default();
        let mut pos = start;
        let mut stream = 0usize;
        while let Some((chunk, next)) = parse_chunk(buf, pos, stream) {
            stream += chunk.blocks.iter().map(|b| b.u_size).sum::<usize>();
            map.chunks.push(chunk);
            pos = next;
        }
        if map.chunks.is_empty() {
            return Err(UpkError::Format(format!(
                "no compressed chunk at offset {start}"
            )));
        }
        Ok(map)
    }

    /// Every chunk in the file, wherever it starts: finds the first valid
    /// chunk header after the summary, follows contiguous chunks, and
    /// searches forward again after any gap. Used on `TAGame.upk`, whose
    /// header tables are chunked too.
    pub fn scan(buf: &[u8]) -> UpkResult<Self> {
        let magic = PACKAGE_MAGIC.to_le_bytes();
        let mut map = ChunkMap::default();
        // Offset 0 is usually the package summary (same magic): its "block
        // size" field fails validation, so starting there is safe.
        let mut pos = 0usize;
        let mut stream = 0usize;
        while pos + 16 <= buf.len() {
            match parse_chunk(buf, pos, stream) {
                Some((chunk, next)) => {
                    stream += chunk.blocks.iter().map(|b| b.u_size).sum::<usize>();
                    map.chunks.push(chunk);
                    pos = next;
                }
                None => match find(&buf[pos + 1..], &magic) {
                    Some(i) => pos = pos + 1 + i,
                    None => break,
                },
            }
        }
        if map.chunks.is_empty() {
            return Err(UpkError::Format("no compressed chunk found".into()));
        }
        Ok(map)
    }

    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.chunks.iter().flat_map(|c| c.blocks.iter())
    }

    pub fn stream_len(&self) -> usize {
        self.blocks().map(|b| b.u_size).sum()
    }

    /// Block containing stream byte `offset`.
    pub fn block_at(&self, offset: usize) -> Option<&Block> {
        self.blocks()
            .find(|b| (b.stream_offset..b.stream_offset + b.u_size).contains(&offset))
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn inflate(compressed: &[u8], expected: usize) -> UpkResult<Vec<u8>> {
    let mut out = Vec::with_capacity(expected);
    ZlibDecoder::new(compressed)
        .read_to_end(&mut out)
        .map_err(|e| UpkError::Zlib(e.to_string()))?;
    Ok(out)
}

pub fn decompress_block(buf: &[u8], block: &Block) -> UpkResult<Vec<u8>> {
    let data = buf
        .get(block.file_offset..block.file_offset + block.c_size)
        .ok_or(UpkError::Truncated("compressed block"))?;
    let out = inflate(data, block.u_size)?;
    if out.len() != block.u_size {
        return Err(UpkError::Zlib(format!(
            "block inflated to {} bytes, expected {}",
            out.len(),
            block.u_size
        )));
    }
    Ok(out)
}

/// Whole stream in memory (item packages are a few MB).
pub fn decompress_all(buf: &[u8], map: &ChunkMap) -> UpkResult<Vec<u8>> {
    let mut out = Vec::with_capacity(map.stream_len());
    for block in map.blocks() {
        out.extend(decompress_block(buf, block)?);
    }
    Ok(out)
}

/// Reads `len` stream bytes starting at `start`, decompressing only the
/// blocks involved.
pub fn read_stream(buf: &[u8], map: &ChunkMap, start: usize, len: usize) -> UpkResult<Vec<u8>> {
    let end = start
        .checked_add(len)
        .ok_or(UpkError::Truncated("stream range"))?;
    let mut out = Vec::with_capacity(len);
    for block in map.blocks() {
        let b_end = block.stream_offset + block.u_size;
        if b_end <= start || block.stream_offset >= end {
            continue;
        }
        let data = decompress_block(buf, block)?;
        let from = start.max(block.stream_offset) - block.stream_offset;
        let to = end.min(b_end) - block.stream_offset;
        out.extend_from_slice(&data[from..to]);
    }
    if out.len() != len {
        return Err(UpkError::Truncated("stream range"));
    }
    Ok(out)
}

fn deflate(data: &[u8]) -> UpkResult<Vec<u8>> {
    let mut enc = ZlibEncoder::new(Vec::with_capacity(data.len() / 2), Compression::best());
    enc.write_all(data)
        .map_err(|e| UpkError::Zlib(e.to_string()))?;
    enc.finish().map_err(|e| UpkError::Zlib(e.to_string()))
}

fn deflate_zopfli(data: &[u8]) -> UpkResult<Vec<u8>> {
    let options = zopfli::Options {
        // 15 iterations: measured sweet spot (legacy palette research).
        iteration_count: std::num::NonZeroU64::new(15).unwrap_or(std::num::NonZeroU64::MIN),
        ..Default::default()
    };
    let mut out = Vec::with_capacity(data.len() / 2);
    zopfli::compress(options, zopfli::Format::Zlib, data, &mut out)
        .map_err(|e| UpkError::Zlib(e.to_string()))?;
    Ok(out)
}

/// Compresses `data` so it fits in `budget` bytes, then zero-pads to exactly
/// `budget`. zlib level 9 first (fast), zopfli as fallback (3–8 % smaller).
pub fn recompress_within(data: &[u8], budget: usize) -> UpkResult<Vec<u8>> {
    let mut out = deflate(data)?;
    if out.len() > budget {
        let z = deflate_zopfli(data)?;
        if z.len() > budget {
            return Err(UpkError::BudgetExceeded {
                new: z.len(),
                budget,
            });
        }
        out = z;
    }
    out.resize(budget, 0);
    Ok(out)
}

/// Edits applied to the decompressed stream, grouped per block when applied.
#[derive(Debug, Default)]
pub struct StreamPatch {
    edits: Vec<(usize, Vec<u8>)>,
}

impl StreamPatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&mut self, stream_offset: usize, bytes: impl Into<Vec<u8>>) {
        self.edits.push((stream_offset, bytes.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Adds this patch's edits to `other`.
    pub fn copy_into(&self, other: &mut StreamPatch) {
        other.edits.extend(self.edits.iter().cloned());
    }

    /// Applies the edits to `file` (a full copy of the package). Edits may
    /// straddle block boundaries. Returns the number of rewritten blocks.
    pub fn apply(&self, file: &mut [u8], map: &ChunkMap) -> UpkResult<usize> {
        let mut rewritten = 0;
        for block in map.blocks() {
            let range = block.stream_offset..block.stream_offset + block.u_size;
            let touching: Vec<&(usize, Vec<u8>)> = self
                .edits
                .iter()
                .filter(|(off, bytes)| *off < range.end && off + bytes.len() > range.start)
                .collect();
            if touching.is_empty() {
                continue;
            }
            let mut data = decompress_block(file, block)?;
            for (off, bytes) in touching {
                for (i, b) in bytes.iter().enumerate() {
                    let s = off + i;
                    if range.contains(&s) {
                        data[s - range.start] = *b;
                    }
                }
            }
            let packed = recompress_within(&data, block.c_size)?;
            file[block.file_offset..block.file_offset + block.c_size].copy_from_slice(&packed);
            rewritten += 1;
        }
        let covered = self.edits.iter().all(|(off, bytes)| {
            map.block_at(*off).is_some()
                && map.block_at(off + bytes.len().saturating_sub(1)).is_some()
        });
        if !covered {
            return Err(UpkError::Format(
                "patch outside the compressed stream".into(),
            ));
        }
        Ok(rewritten)
    }
}

/// Wraps a payload as one chunk (the format of compressed bulk data, e.g.
/// texture mips stored in a `.tfc`).
pub fn wrap_chunked(payload: &[u8]) -> UpkResult<Vec<u8>> {
    let bs = DEFAULT_BLOCK_SIZE as usize;
    let blocks: Vec<Vec<u8>> = payload
        .chunks(bs.max(1))
        .map(deflate)
        .collect::<UpkResult<_>>()?;
    let c_total: usize = blocks.iter().map(Vec::len).sum();
    let mut out = Vec::with_capacity(16 + blocks.len() * 8 + c_total);
    out.extend(PACKAGE_MAGIC.to_le_bytes());
    out.extend(DEFAULT_BLOCK_SIZE.to_le_bytes());
    out.extend((c_total as u32).to_le_bytes());
    out.extend((payload.len() as u32).to_le_bytes());
    for (i, b) in blocks.iter().enumerate() {
        let u = bs.min(payload.len() - i * bs);
        out.extend((b.len() as u32).to_le_bytes());
        out.extend((u as u32).to_le_bytes());
    }
    for b in &blocks {
        out.extend(b);
    }
    Ok(out)
}

/// Inverse of [`wrap_chunked`] (reads every chunk present).
pub fn unwrap_chunked(bytes: &[u8]) -> UpkResult<Vec<u8>> {
    let map = ChunkMap::read_from(bytes, 0)?;
    decompress_all(bytes, &map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(len: usize) -> Vec<u8> {
        (0..len).map(|i| ((i * 31) ^ (i >> 7)) as u8).collect()
    }

    #[test]
    fn wrap_unwrap_roundtrip_multi_block() {
        let payload = sample(300_000);
        let wrapped = wrap_chunked(&payload).unwrap();
        assert_eq!(unwrap_chunked(&wrapped).unwrap(), payload);
    }

    #[test]
    fn scan_finds_chunks_after_garbage() {
        let payload = sample(200_000);
        let mut file = vec![0xAAu8; 37];
        file.extend(wrap_chunked(&payload).unwrap());
        let map = ChunkMap::scan(&file).unwrap();
        assert_eq!(map.stream_len(), payload.len());
        assert_eq!(decompress_all(&file, &map).unwrap(), payload);
    }

    #[test]
    fn patch_straddling_blocks_keeps_file_size() {
        let payload = vec![0u8; 2 * DEFAULT_BLOCK_SIZE as usize];
        let mut file = wrap_chunked(&payload).unwrap();
        // Give each block slack so recompression of the edited block fits.
        let map = ChunkMap::read_from(&file, 0).unwrap();
        let size_before = file.len();
        let mut patch = StreamPatch::new();
        let edge = DEFAULT_BLOCK_SIZE as usize - 2;
        patch.put(edge, vec![0u8; 4]); // same content: always fits
        assert_eq!(patch.apply(&mut file, &map).unwrap(), 2);
        assert_eq!(file.len(), size_before);
        assert_eq!(unwrap_chunked(&file).unwrap(), payload);
    }

    #[test]
    fn recompress_pads_to_budget_or_fails() {
        let data = vec![7u8; 10_000];
        let packed = recompress_within(&data, 500).unwrap();
        assert_eq!(packed.len(), 500);
        assert_eq!(&inflate(&packed, data.len()).unwrap(), &data);
        let noisy: Vec<u8> = (0..10_000u32)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
            .collect();
        assert!(matches!(
            recompress_within(&noisy, 100),
            Err(UpkError::BudgetExceeded { .. })
        ));
    }
}
