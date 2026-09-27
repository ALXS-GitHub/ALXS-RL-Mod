//! Locates the colour-picker arrays inside `TAGame.upk` — no hard-coded
//! offsets, so a game update does not break the palette.
//!
//! Each picker is a `TArray<FLinearColor>`: `i32 count` then `count × 16`
//! bytes of `f32 R, G, B, A` with `A = 1`. RL has three of them next to each
//! other in the decompressed stream (see `docs_rl/palette.md`):
//! the shared accent grid (105 = 15 hues × 7 shades), then the blue and
//! orange primary grids (70 = 10 hues × 7 shades each). Smaller legacy
//! arrays (18/28 entries) also exist and are ignored.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::palette::store::Rgb;
use crate::upk::chunks::{self, ChunkMap};
use crate::upk::error::UpkResult;
use crate::upk::reader::{read_f32, read_i32};

pub const ACCENT_COUNT: usize = 105;
pub const PRIMARY_COUNT: usize = 70;
pub const ROWS: usize = 7;
const STRIDE: usize = 16;
/// Max distance between the accent array and the primary arrays.
const MAX_SPREAD: usize = 1 << 20;

/// Stream offsets of each array's `i32 count`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub accent: usize,
    pub blue: usize,
    pub orange: usize,
}

pub const fn array_len(count: usize) -> usize {
    4 + count * STRIDE
}

fn channel_ok(v: f32) -> bool {
    v.is_finite() && (-1e-4..=1.0001).contains(&v)
}

/// True if `buf[pos..]` holds `count` followed by `count` plausible colours.
pub fn is_color_array(buf: &[u8], pos: usize, count: usize) -> bool {
    if read_i32(buf, pos) != Some(count as i32) || buf.len() < pos + array_len(count) {
        return false;
    }
    let mut first: Option<[u32; 3]> = None;
    let mut varied = false;
    for i in 0..count {
        let p = pos + 4 + i * STRIDE;
        let (Some(r), Some(g), Some(b), Some(a)) = (
            read_f32(buf, p),
            read_f32(buf, p + 4),
            read_f32(buf, p + 8),
            read_f32(buf, p + 12),
        ) else {
            return false;
        };
        if !(channel_ok(r) && channel_ok(g) && channel_ok(b)) || (a - 1.0).abs() > 0.01 {
            return false;
        }
        let bits = [r.to_bits(), g.to_bits(), b.to_bits()];
        match first {
            None => first = Some(bits),
            Some(f) if f != bits => varied = true,
            _ => {}
        }
    }
    varied
}

/// Records `(stream_offset, count)` of every candidate array in `window`,
/// whose first byte is at stream offset `base`.
pub fn scan_window(window: &[u8], base: usize, hits: &mut BTreeSet<(usize, usize)>) {
    let accent = (ACCENT_COUNT as u32).to_le_bytes();
    let primary = (PRIMARY_COUNT as u32).to_le_bytes();
    let mut i = 0;
    while i + 4 <= window.len() {
        let w = &window[i..i + 4];
        let count = if w == accent {
            ACCENT_COUNT
        } else if w == primary {
            PRIMARY_COUNT
        } else {
            i += 1;
            continue;
        };
        if is_color_array(window, i, count) {
            hits.insert((base + i, count));
        }
        i += 1;
    }
}

/// Picks accent + blue + orange among the hits: the first accent array
/// followed by two primary arrays within [`MAX_SPREAD`].
pub fn choose(hits: &BTreeSet<(usize, usize)>) -> Option<Layout> {
    let primaries: Vec<usize> = hits
        .iter()
        .filter(|(_, c)| *c == PRIMARY_COUNT)
        .map(|(o, _)| *o)
        .collect();
    for &(accent, _) in hits.iter().filter(|(_, c)| *c == ACCENT_COUNT) {
        let after: Vec<usize> = primaries
            .iter()
            .copied()
            .filter(|&o| o > accent + array_len(ACCENT_COUNT) - 4 && o - accent <= MAX_SPREAD)
            .collect();
        if let [blue, orange, ..] = after[..] {
            if orange >= blue + array_len(PRIMARY_COUNT) {
                return Some(Layout {
                    accent,
                    blue,
                    orange,
                });
            }
        }
    }
    None
}

/// Streams through every block of the file (bounded memory) and returns the
/// layout. Stops early once a layout is found and the scan moved past it.
pub fn locate(file: &[u8], map: &ChunkMap) -> UpkResult<Option<Layout>> {
    let overlap = array_len(ACCENT_COUNT);
    let mut hits = BTreeSet::new();
    let mut tail: Vec<u8> = Vec::new();
    let mut tail_start = 0usize;
    for block in map.blocks() {
        let data = chunks::decompress_block(file, block)?;
        let mut window = std::mem::take(&mut tail);
        window.extend_from_slice(&data);
        scan_window(&window, tail_start, &mut hits);
        let keep = window.len().min(overlap);
        tail = window[window.len() - keep..].to_vec();
        tail_start = block.stream_offset + block.u_size - keep;
        if let Some(layout) = choose(&hits) {
            if block.stream_offset > layout.orange + MAX_SPREAD {
                return Ok(Some(layout));
            }
        }
    }
    Ok(choose(&hits))
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Colours of an array whose bytes (count included) are in `buf`.
pub fn read_colors(buf: &[u8], count: usize) -> Vec<Rgb> {
    (0..count)
        .map(|i| {
            let p = 4 + i * STRIDE;
            let c = |o| read_f32(buf, p + o).unwrap_or(0.0);
            Rgb {
                r: to_u8(c(0)),
                g: to_u8(c(4)),
                b: to_u8(c(8)),
            }
        })
        .collect()
}

/// New bytes for an array (count included). Slots beyond `colors` keep
/// their original bytes.
pub fn encode(original: &[u8], colors: &[Rgb], count: usize) -> Vec<u8> {
    let mut out = original[..array_len(count).min(original.len())].to_vec();
    for (i, c) in colors.iter().take(count).enumerate() {
        let p = 4 + i * STRIDE;
        if p + STRIDE > out.len() {
            break;
        }
        for (k, v) in [c.r, c.g, c.b].into_iter().enumerate() {
            out[p + k * 4..p + k * 4 + 4].copy_from_slice(&(v as f32 / 255.0).to_le_bytes());
        }
        out[p + 12..p + 16].copy_from_slice(&1.0f32.to_le_bytes());
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn fake_array(count: usize, seed: u8) -> Vec<u8> {
        let mut b = (count as i32).to_le_bytes().to_vec();
        for i in 0..count {
            let v = ((i as u8).wrapping_mul(7).wrapping_add(seed)) as f32 / 255.0;
            for f in [v, 1.0 - v, 0.5, 1.0] {
                b.extend(f.to_le_bytes());
            }
        }
        b
    }

    /// Stream with legacy arrays + the three real ones at odd offsets.
    pub fn fake_stream() -> (Vec<u8>, Layout) {
        let mut s = vec![0u8; 1001];
        s.extend(fake_array(28, 3)); // legacy, ignored
        let accent = s.len();
        s.extend(fake_array(ACCENT_COUNT, 1));
        s.extend(vec![0u8; 333]);
        let blue = s.len();
        s.extend(fake_array(PRIMARY_COUNT, 2));
        s.extend(vec![0u8; 77]);
        let orange = s.len();
        s.extend(fake_array(PRIMARY_COUNT, 9));
        s.extend(vec![0u8; 500]);
        (
            s,
            Layout {
                accent,
                blue,
                orange,
            },
        )
    }

    #[test]
    fn finds_layout_in_stream() {
        let (s, expected) = fake_stream();
        let mut hits = BTreeSet::new();
        scan_window(&s, 0, &mut hits);
        assert_eq!(choose(&hits), Some(expected));
    }

    #[test]
    fn rejects_constant_or_out_of_range_arrays() {
        let mut flat = (70i32).to_le_bytes().to_vec();
        for _ in 0..70 {
            for f in [0.2f32, 0.2, 0.2, 1.0] {
                flat.extend(f.to_le_bytes());
            }
        }
        assert!(!is_color_array(&flat, 0, 70));
        let mut hdr = fake_array(70, 1);
        hdr[4..8].copy_from_slice(&3.5f32.to_le_bytes());
        assert!(!is_color_array(&hdr, 0, 70));
    }

    #[test]
    fn encode_then_read_roundtrip_keeps_unset_slots() {
        let original = fake_array(PRIMARY_COUNT, 5);
        let before = read_colors(&original, PRIMARY_COUNT);
        let new = [Rgb {
            r: 255,
            g: 0,
            b: 128,
        }];
        let bytes = encode(&original, &new, PRIMARY_COUNT);
        let after = read_colors(&bytes, PRIMARY_COUNT);
        assert_eq!(after[0], new[0]);
        assert_eq!(after[1..], before[1..]);
        assert_eq!(bytes.len(), original.len());
    }

    #[test]
    fn locate_streams_across_blocks() {
        let (s, expected) = fake_stream();
        // Tiny blocks (4 KiB) force arrays to straddle block boundaries.
        let mut file = Vec::new();
        let bs = 4096usize;
        let blocks: Vec<Vec<u8>> = s
            .chunks(bs)
            .map(|c| {
                use std::io::Write;
                let mut e =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(c).unwrap();
                e.finish().unwrap()
            })
            .collect();
        file.extend(crate::upk::summary::PACKAGE_MAGIC.to_le_bytes());
        file.extend((bs as u32).to_le_bytes());
        file.extend((blocks.iter().map(Vec::len).sum::<usize>() as u32).to_le_bytes());
        file.extend((s.len() as u32).to_le_bytes());
        for (i, b) in blocks.iter().enumerate() {
            file.extend((b.len() as u32).to_le_bytes());
            file.extend((bs.min(s.len() - i * bs) as u32).to_le_bytes());
        }
        for b in &blocks {
            file.extend(b);
        }
        let map = ChunkMap::read_from(&file, 0).unwrap();
        assert_eq!(locate(&file, &map).unwrap(), Some(expected));
    }
}
