//! `Texture2D` exports: properties, mip chain (inline or in a `.tfc` texture
//! file cache) and BC1/BC3 codecs.
//!
//! Serialized layout after the property block (cooked RL):
//! an empty `SourceArt` bulk data, `i32 NumMips`, then per mip an
//! `FByteBulkData` (`u32 flags · i32 elem_count · i32 size_on_disk ·
//! i64 offset_in_file` + payload when inline) followed by `i32 SizeX · i32 SizeY`.
//! The exact position of `NumMips` is found by validating candidate mip
//! chains rather than trusting a fixed offset (it varies across builds).

use image::RgbaImage;
use image_dds::{ImageFormat, Mipmaps, Quality, Surface, SurfaceRgba8};

use crate::upk::error::{UpkError, UpkResult};
use crate::upk::names::NameTable;
use crate::upk::props;
use crate::upk::reader::{read_i32, read_u32, read_u64};

pub const BULK_SEPARATE_FILE: u32 = 0x01;
pub const BULK_COMPRESSED_ZLIB: u32 = 0x02;
pub const BULK_COMPRESSED_LZO: u32 = 0x10;
pub const BULK_UNUSED: u32 = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    Bc5,
    A8R8G8B8,
    G8,
    Unknown,
}

impl PixelFormat {
    pub fn from_name(name: &str) -> Self {
        match name {
            "PF_DXT1" => Self::Dxt1,
            "PF_DXT3" => Self::Dxt3,
            "PF_DXT5" => Self::Dxt5,
            "PF_BC5" | "PF_ATI2" => Self::Bc5,
            "PF_A8R8G8B8" => Self::A8R8G8B8,
            "PF_G8" | "PF_L8" => Self::G8,
            _ => Self::Unknown,
        }
    }

    /// Uncompressed byte size of one mip.
    pub fn mip_bytes(self, w: u32, h: u32) -> Option<usize> {
        let blocks =
            |bpb: usize| (w.div_ceil(4).max(1) as usize) * (h.div_ceil(4).max(1) as usize) * bpb;
        Some(match self {
            Self::Dxt1 => blocks(8),
            Self::Dxt3 | Self::Dxt5 | Self::Bc5 => blocks(16),
            Self::A8R8G8B8 => (w * h * 4) as usize,
            Self::G8 => (w * h) as usize,
            Self::Unknown => return None,
        })
    }

    fn dds(self) -> Option<ImageFormat> {
        match self {
            Self::Dxt1 => Some(ImageFormat::BC1RgbaUnorm),
            Self::Dxt3 => Some(ImageFormat::BC2RgbaUnorm),
            Self::Dxt5 => Some(ImageFormat::BC3RgbaUnorm),
            Self::Bc5 => Some(ImageFormat::BC5RgUnorm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mip {
    pub flags: u32,
    pub elem_count: u32,
    pub size_on_disk: u32,
    pub offset_in_file: u64,
    pub width: u32,
    pub height: u32,
    /// Buffer offset of `flags` (stream offset when parsed from a body).
    pub header_pos: usize,
    /// Inline payload position (only for mips stored in the package).
    pub data_pos: Option<usize>,
}

impl Mip {
    pub fn in_tfc(&self) -> bool {
        self.flags & BULK_SEPARATE_FILE != 0
    }
    pub fn compressed(&self) -> bool {
        self.flags & BULK_COMPRESSED_ZLIB != 0
    }
    pub fn is_empty(&self) -> bool {
        self.flags & BULK_UNUSED != 0 || self.elem_count == 0
    }
    /// Positions of the patchable fields (TFC mips only).
    pub fn elem_field(&self) -> usize {
        self.header_pos + 4
    }
    pub fn size_field(&self) -> usize {
        self.header_pos + 8
    }
    pub fn offset_field(&self) -> usize {
        self.header_pos + 12
    }
}

#[derive(Debug, Clone)]
pub struct Texture2D {
    pub export_index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub format_name: String,
    /// `TextureFileCacheName` property, if any.
    pub tfc_name: Option<String>,
    pub mips: Vec<Mip>,
}

/// Parses one bulk-data entry + trailing size. `with_offset` toggles the
/// presence of the 8-byte file offset (absent on some empty entries).
fn parse_mip(buf: &[u8], pos: usize, end: usize, with_offset: bool) -> Option<(Mip, usize)> {
    let flags = read_u32(buf, pos)?;
    if flags & !0x000F_00FF != 0 || flags & BULK_COMPRESSED_LZO != 0 {
        return None;
    }
    let elem_count = read_i32(buf, pos + 4)?;
    // Unused mips (e.g. a 4K top level the cook dropped) store -1 as size.
    let unused = flags & BULK_UNUSED != 0;
    let size_on_disk = match read_i32(buf, pos + 8)? {
        -1 if unused => 0,
        s => s,
    };
    if elem_count < 0 || size_on_disk < 0 {
        return None;
    }
    let mut p = pos + 12;
    let offset_in_file = if with_offset {
        let o = read_u64(buf, p)?;
        p += 8;
        o
    } else {
        0
    };
    let mut data_pos = None;
    if flags & BULK_SEPARATE_FILE == 0 && size_on_disk > 0 {
        data_pos = Some(p);
        p = p.checked_add(size_on_disk as usize)?;
    }
    let width = read_i32(buf, p)?;
    let height = read_i32(buf, p + 4)?;
    p += 8;
    if p > end || !(1..=16_384).contains(&width) || !(1..=16_384).contains(&height) {
        return None;
    }
    Some((
        Mip {
            flags,
            elem_count: elem_count as u32,
            size_on_disk: size_on_disk as u32,
            offset_in_file: if flags & BULK_SEPARATE_FILE != 0 && !unused {
                offset_in_file
            } else {
                0
            },
            width: width as u32,
            height: height as u32,
            header_pos: pos,
            data_pos,
        },
        p,
    ))
}

fn parse_chain(
    buf: &[u8],
    start: usize,
    end: usize,
    count: usize,
    format: PixelFormat,
) -> Option<Vec<Mip>> {
    let mut mips = Vec::with_capacity(count);
    let mut pos = start;
    for _ in 0..count {
        let (mip, next) =
            parse_mip(buf, pos, end, true).or_else(|| parse_mip(buf, pos, end, false))?;
        if let (Some(expected), false) = (format.mip_bytes(mip.width, mip.height), mip.is_empty()) {
            if mip.elem_count as usize != expected {
                return None;
            }
        }
        if let Some(prev) = mips.last() {
            let prev: &Mip = prev;
            if mip.width > prev.width || mip.height > prev.height {
                return None;
            }
        }
        mips.push(mip);
        pos = next;
    }
    mips.iter().any(|m| !m.is_empty()).then_some(mips)
}

/// Locates `NumMips` after the property block by validating candidates.
pub fn find_mips(
    buf: &[u8],
    props_end: usize,
    serial_end: usize,
    format: PixelFormat,
) -> Option<Vec<Mip>> {
    for k in 0..=96usize {
        let at = props_end + k;
        let Some(count) = read_i32(buf, at) else {
            break;
        };
        if !(1..=16).contains(&count) {
            continue;
        }
        if let Some(chain) = parse_chain(
            buf,
            at + 4,
            serial_end.min(buf.len()),
            count as usize,
            format,
        ) {
            return Some(chain);
        }
    }
    None
}

/// `TextureFileCacheName` of a Texture2D export, read from its properties
/// only (works for exports whose mip chain does not parse).
pub fn cache_name_of(buf: &[u8], pos: usize, names: &NameTable) -> Option<String> {
    let (tags, _) = props::walk(buf, pos + 4, names).ok()?;
    props::find(&tags, "TextureFileCacheName")
        .and_then(|p| p.as_name(buf, names))
        .map(str::to_string)
}

/// Parses a Texture2D export whose serial data starts at `pos` (NetIndex).
pub fn parse_texture(
    buf: &[u8],
    pos: usize,
    serial_size: usize,
    export_index: usize,
    name: &str,
    names: &NameTable,
) -> UpkResult<Texture2D> {
    if read_i32(buf, pos) != Some(-1) {
        return Err(UpkError::Texture(format!(
            "{name}: no NetIndex at serial start"
        )));
    }
    let (tags, props_end) = props::walk(buf, pos + 4, names)?;
    let int = |n: &str| props::find(&tags, n).and_then(|p| p.as_i32(buf));
    let width = int("SizeX").unwrap_or(0).max(0) as u32;
    let height = int("SizeY").unwrap_or(0).max(0) as u32;
    let format_name = props::find(&tags, "Format")
        .and_then(|p| p.as_name(buf, names))
        .unwrap_or("PF_DXT1");
    let format = PixelFormat::from_name(format_name);
    let tfc_name = props::find(&tags, "TextureFileCacheName")
        .and_then(|p| p.as_name(buf, names))
        .filter(|n| !n.is_empty() && *n != "None")
        .map(str::to_string);
    // Some exports under-report their serial size; allow a little slack.
    let serial_end = (pos + serial_size + 64).min(buf.len());
    let mips = find_mips(buf, props_end, serial_end, format)
        .ok_or_else(|| UpkError::Texture(format!("{name}: mip chain not found")))?;
    Ok(Texture2D {
        export_index,
        name: name.to_string(),
        width,
        height,
        format,
        format_name: format_name.to_string(),
        tfc_name,
        mips,
    })
}

// ── Codecs ───────────────────────────────────────────────────────────────

/// Decodes one mip to RGBA8.
pub fn decode(format: PixelFormat, width: u32, height: u32, data: &[u8]) -> UpkResult<RgbaImage> {
    let expected = format
        .mip_bytes(width, height)
        .ok_or_else(|| UpkError::Texture(format!("unsupported pixel format {format:?}")))?;
    if data.len() < expected {
        return Err(UpkError::Texture(format!(
            "mip data too short ({} < {expected})",
            data.len()
        )));
    }
    let data = &data[..expected];
    let rgba: Vec<u8> = match format {
        PixelFormat::A8R8G8B8 => data
            .chunks_exact(4)
            .flat_map(|p| [p[2], p[1], p[0], p[3]])
            .collect(),
        PixelFormat::G8 => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        other => {
            let fmt = other
                .dds()
                .ok_or_else(|| UpkError::Texture(format!("unsupported pixel format {other:?}")))?;
            let surface = Surface {
                width,
                height,
                depth: 1,
                layers: 1,
                mipmaps: 1,
                image_format: fmt,
                data,
            };
            surface
                .decode_rgba8()
                .map_err(|e| UpkError::Texture(e.to_string()))?
                .data
        }
    };
    RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| UpkError::Texture("decoded buffer size mismatch".into()))
}

/// Encodes an RGBA image to BC1 (DXT1), BC3 (DXT5) or raw A8R8G8B8.
pub fn encode(img: &RgbaImage, format: PixelFormat) -> UpkResult<Vec<u8>> {
    if format == PixelFormat::A8R8G8B8 {
        // Stored as B, G, R, A bytes (see `decode`).
        return Ok(img
            .as_raw()
            .chunks_exact(4)
            .flat_map(|p| [p[2], p[1], p[0], p[3]])
            .collect());
    }
    let fmt = match format {
        PixelFormat::Dxt1 => ImageFormat::BC1RgbaUnorm,
        PixelFormat::Dxt5 => ImageFormat::BC3RgbaUnorm,
        other => {
            return Err(UpkError::Texture(format!(
                "encoding to {other:?} is not supported"
            )))
        }
    };
    let surface = SurfaceRgba8 {
        width: img.width(),
        height: img.height(),
        depth: 1,
        layers: 1,
        mipmaps: 1,
        data: img.as_raw().as_slice(),
    };
    // Slow = best quality: masks rely on crisp channel values.
    Ok(surface
        .encode(fmt, Quality::Slow, Mipmaps::Disabled)
        .map_err(|e| UpkError::Texture(e.to_string()))?
        .data)
}

/// `img` at each `(w, h)` of a mip chain (largest first). Each level is
/// filtered down from the previous one, not from the source: a proper mip
/// pyramid, and far cheaper than resampling the full image every time.
pub fn mip_images(img: &RgbaImage, dims: &[(u32, u32)]) -> Vec<RgbaImage> {
    let mut out: Vec<RgbaImage> = Vec::with_capacity(dims.len());
    for &(w, h) in dims {
        let from = out.last().unwrap_or(img);
        let level = if from.dimensions() == (w, h) {
            from.clone()
        } else {
            let filter = if from.width() >= w && from.height() >= h {
                image::imageops::FilterType::Triangle
            } else {
                image::imageops::FilterType::Lanczos3
            };
            image::imageops::resize(from, w, h, filter)
        };
        out.push(level);
    }
    out
}

/// Resizes `img` to each `(w, h)` of a mip chain and encodes it.
pub fn encode_chain(
    img: &RgbaImage,
    dims: &[(u32, u32)],
    format: PixelFormat,
) -> UpkResult<Vec<Vec<u8>>> {
    mip_images(img, dims)
        .iter()
        .map(|level| encode(level, format))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upk::props::tests::{names, texture_props};

    fn mip_bytes_tfc(elem: u32, size: u32, offset: u64, dim: i32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend((BULK_SEPARATE_FILE | BULK_COMPRESSED_ZLIB | 0x10000).to_le_bytes());
        b.extend((elem as i32).to_le_bytes());
        b.extend((size as i32).to_le_bytes());
        b.extend(offset.to_le_bytes());
        b.extend(dim.to_le_bytes());
        b.extend(dim.to_le_bytes());
        b
    }

    fn mip_bytes_inline(payload: &[u8], dim: i32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(0x10000u32.to_le_bytes());
        b.extend((payload.len() as i32).to_le_bytes());
        b.extend((payload.len() as i32).to_le_bytes());
        b.extend(0u64.to_le_bytes());
        b.extend(payload);
        b.extend(dim.to_le_bytes());
        b.extend(dim.to_le_bytes());
        b
    }

    #[test]
    fn parses_texture_with_tfc_and_inline_mips() {
        let n = names();
        let mut buf = (-1i32).to_le_bytes().to_vec();
        buf.extend(texture_props(256));
        // Empty SourceArt (12 bytes, no offset) then NumMips.
        buf.extend([0x00, 0x00, 0x01, 0x00]);
        buf.extend([0u8; 8]);
        buf.extend(3i32.to_le_bytes());
        buf.extend(mip_bytes_tfc(256 * 256, 1000, 4096, 256));
        buf.extend(mip_bytes_tfc(128 * 128, 500, 8192, 128));
        buf.extend(mip_bytes_inline(&vec![0u8; 64 * 64], 64));
        let t = parse_texture(&buf, 0, buf.len(), 0, "Skin_RGB", &n).unwrap();
        assert_eq!(t.format, PixelFormat::Dxt5);
        assert_eq!(t.tfc_name.as_deref(), Some("Textures7"));
        assert_eq!(t.mips.len(), 3);
        assert!(t.mips[0].in_tfc() && t.mips[0].compressed());
        assert_eq!(t.mips[1].offset_in_file, 8192);
        assert!(!t.mips[2].in_tfc());
        assert_eq!(t.mips[2].width, 64);
        // Patchable field positions point at the right bytes.
        assert_eq!(read_u64(&buf, t.mips[0].offset_field()), Some(4096));
    }

    #[test]
    fn parses_chain_with_unused_top_mip() {
        let n = names();
        let mut buf = (-1i32).to_le_bytes().to_vec();
        buf.extend(texture_props(256));
        buf.extend([0x00, 0x00, 0x01, 0x00]);
        buf.extend([0u8; 8]);
        buf.extend(2i32.to_le_bytes());
        // Unused 512 mip: elem 0, size -1, offset -1 (as cooked by RL).
        buf.extend((BULK_SEPARATE_FILE | BULK_UNUSED).to_le_bytes());
        buf.extend(0i32.to_le_bytes());
        buf.extend((-1i32).to_le_bytes());
        buf.extend((-1i64).to_le_bytes());
        buf.extend(512i32.to_le_bytes());
        buf.extend(512i32.to_le_bytes());
        buf.extend(mip_bytes_tfc(256 * 256, 1000, 4096, 256));
        let t = parse_texture(&buf, 0, buf.len(), 0, "Ball_N", &n).unwrap();
        assert_eq!(t.mips.len(), 2);
        assert!(t.mips[0].is_empty());
        assert_eq!((t.mips[0].size_on_disk, t.mips[0].offset_in_file), (0, 0));
        assert_eq!(t.mips[1].offset_in_file, 4096);
        assert_eq!(cache_name_of(&buf, 0, &n).as_deref(), Some("Textures7"));
    }

    #[test]
    fn bc3_roundtrip_keeps_solid_colour() {
        let img = RgbaImage::from_pixel(16, 16, image::Rgba([200, 30, 10, 255]));
        let enc = encode(&img, PixelFormat::Dxt5).unwrap();
        assert_eq!(enc.len(), PixelFormat::Dxt5.mip_bytes(16, 16).unwrap());
        let dec = decode(PixelFormat::Dxt5, 16, 16, &enc).unwrap();
        let px = dec.get_pixel(5, 5).0;
        assert!((px[0] as i32 - 200).abs() < 8 && px[3] == 255);
    }

    #[test]
    fn argb_roundtrip_is_exact() {
        let img = RgbaImage::from_pixel(4, 2, image::Rgba([1, 2, 3, 4]));
        let enc = encode(&img, PixelFormat::A8R8G8B8).unwrap();
        assert_eq!(&enc[..4], &[3, 2, 1, 4]);
        assert_eq!(decode(PixelFormat::A8R8G8B8, 4, 2, &enc).unwrap(), img);
    }

    #[test]
    fn mip_sizes() {
        assert_eq!(
            PixelFormat::Dxt1.mip_bytes(2048, 2048),
            Some(2048 * 2048 / 2)
        );
        assert_eq!(PixelFormat::Dxt5.mip_bytes(2, 2), Some(16));
        assert_eq!(PixelFormat::Unknown.mip_bytes(4, 4), None);
    }
}
