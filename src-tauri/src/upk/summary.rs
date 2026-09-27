//! `FPackageFileSummary` — the plain-text header at the start of every
//! cooked package. Only the fields up to the table offsets are required; the
//! tail (generations, versions, compression info) is parsed best-effort.

use crate::upk::error::{UpkError, UpkResult};
use crate::upk::reader::Reader;

pub const PACKAGE_MAGIC: u32 = 0x9E2A_83C1;

/// Engine version from which the summary carries import/export GUID and
/// thumbnail table offsets (UE3 VER_ADDED_... = 623). RL files are 868.
const VER_GUIDS_AND_THUMBNAILS: u16 = 623;

/// Package flag of Rocket League's "fully encrypted" packages (licensee
/// version 33+, from late August 2026): header *and* compressed chunks are
/// AES-256-CTR, each with its own 12-byte nonce.
pub const FULL_ENCRYPTED_FLAG: u32 = 0x0800;
const LICENSEE_CTR: u16 = 33;

#[derive(Debug, Clone, Default)]
pub struct Generation {
    pub export_count: u32,
    pub name_count: u32,
    pub net_object_count: u32,
}

#[derive(Debug, Clone)]
pub struct PackageSummary {
    pub file_version: u16,
    pub licensee_version: u16,
    pub total_header_size: usize,
    pub folder_name: String,
    pub package_flags: u32,
    pub name_count: usize,
    pub name_offset: usize,
    pub export_count: usize,
    pub export_offset: usize,
    pub import_count: usize,
    pub import_offset: usize,
    pub depends_offset: usize,
    pub guid: Option<[u8; 16]>,
    pub generations: Vec<Generation>,
    pub engine_version: Option<u32>,
    pub cooker_version: Option<u32>,
    pub compression_flags: Option<u32>,
    /// Nonce of the AES-CTR header region (fully encrypted packages only):
    /// the last field of the summary, right before the name table.
    pub header_nonce: Option<[u8; 12]>,
}

impl PackageSummary {
    pub fn parse(buf: &[u8]) -> UpkResult<Self> {
        let mut r = Reader::new(buf);
        if r.u32("magic")? != PACKAGE_MAGIC {
            return Err(UpkError::BadMagic);
        }
        let file_version = r.u16("file version")?;
        let licensee_version = r.u16("licensee version")?;
        let total_header_size = r.u32("total header size")? as usize;
        let folder_name = r.fstring("folder name")?;
        let package_flags = r.u32("package flags")?;
        let name_count = r.u32("name count")? as usize;
        let name_offset = r.u32("name offset")? as usize;
        let export_count = r.u32("export count")? as usize;
        let export_offset = r.u32("export offset")? as usize;
        let import_count = r.u32("import count")? as usize;
        let import_offset = r.u32("import offset")? as usize;
        let depends_offset = r.u32("depends offset")? as usize;

        if name_offset > total_header_size || total_header_size > buf.len() {
            return Err(UpkError::Format(format!(
                "header size {total_header_size} / name offset {name_offset} inconsistent with file size {}",
                buf.len()
            )));
        }

        let mut summary = Self {
            file_version,
            licensee_version,
            total_header_size,
            folder_name,
            package_flags,
            name_count,
            name_offset,
            export_count,
            export_offset,
            import_count,
            import_offset,
            depends_offset,
            guid: None,
            generations: Vec::new(),
            engine_version: None,
            cooker_version: None,
            compression_flags: None,
            header_nonce: None,
        };
        if licensee_version >= LICENSEE_CTR && package_flags & FULL_ENCRYPTED_FLAG != 0 {
            let nonce = name_offset
                .checked_sub(12)
                .and_then(|at| buf.get(at..name_offset))
                .ok_or(UpkError::Truncated("header nonce"))?;
            let mut n = [0u8; 12];
            n.copy_from_slice(nonce);
            summary.header_nonce = Some(n);
        }
        // Optional tail — never fail the parse because of it.
        let _ = summary.parse_tail(&mut r);
        Ok(summary)
    }

    fn parse_tail(&mut self, r: &mut Reader<'_>) -> UpkResult<()> {
        if self.file_version >= VER_GUIDS_AND_THUMBNAILS {
            // import/export guids offset, import guid count, export guid count, thumbnail table offset
            r.skip(16, "guid offsets")?;
        }
        let mut guid = [0u8; 16];
        guid.copy_from_slice(r.bytes(16, "guid")?);
        self.guid = Some(guid);
        let gen_count = r.u32("generation count")? as usize;
        if gen_count > 1024 {
            return Err(UpkError::Format("absurd generation count".into()));
        }
        for _ in 0..gen_count {
            self.generations.push(Generation {
                export_count: r.u32("generation")?,
                name_count: r.u32("generation")?,
                net_object_count: r.u32("generation")?,
            });
        }
        self.engine_version = Some(r.u32("engine version")?);
        self.cooker_version = Some(r.u32("cooker version")?);
        self.compression_flags = Some(r.u32("compression flags")?);
        Ok(())
    }

    /// Byte length of the AES-encrypted region starting at `name_offset`
    /// (the header tables), rounded down to the AES block size.
    pub fn encrypted_region_len(&self) -> usize {
        (self.total_header_size - self.name_offset) & !15
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Minimal synthetic summary used by other module tests.
    pub fn synthetic_summary(name_count: u32, name_offset: u32, total_header: u32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(PACKAGE_MAGIC.to_le_bytes());
        b.extend(868u16.to_le_bytes());
        b.extend(32u16.to_le_bytes());
        b.extend(total_header.to_le_bytes());
        b.extend(5i32.to_le_bytes());
        b.extend(b"None\0");
        b.extend(0u32.to_le_bytes()); // flags
        b.extend(name_count.to_le_bytes());
        b.extend(name_offset.to_le_bytes());
        b.extend(0u32.to_le_bytes()); // export count
        b.extend(name_offset.to_le_bytes());
        b.extend(0u32.to_le_bytes()); // import count
        b.extend(name_offset.to_le_bytes());
        b.extend(name_offset.to_le_bytes()); // depends
        b
    }

    #[test]
    fn parses_required_fields() {
        let mut b = synthetic_summary(3, 64, 128);
        b.resize(128, 0);
        let s = PackageSummary::parse(&b).unwrap();
        assert_eq!(s.file_version, 868);
        assert_eq!(s.folder_name, "None");
        assert_eq!(s.name_count, 3);
        assert_eq!(s.name_offset, 64);
        assert_eq!(s.encrypted_region_len(), 64);
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(matches!(
            PackageSummary::parse(&[0u8; 64]),
            Err(UpkError::BadMagic)
        ));
    }
}
