//! Import and export tables (inside the decrypted header region).
//!
//! RL export entries use a 64-bit `SerialOffset`; entry size is
//! `52 + 4 × NetObjectCount + 16 (GUID) + 4 (package flags)`.

use crate::upk::crypto::HeaderRegion;
use crate::upk::error::{UpkError, UpkResult};
use crate::upk::names::NameTable;
use crate::upk::reader::Reader;
use crate::upk::summary::PackageSummary;

/// Import entry: class package (FName), class name (FName), outer, object name (FName).
const IMPORT_SIZE: usize = 28;
/// Export entry up to and including `NetObjectCount`.
const EXPORT_FIXED: usize = 52;

#[derive(Debug, Clone)]
pub struct Import {
    pub index: usize,
    pub class_package: String,
    pub class_name: String,
    pub outer: i32,
    pub object_name: String,
}

#[derive(Debug, Clone)]
pub struct Export {
    /// 0-based; the positive ObjectIndex referencing it is `index + 1`.
    pub index: usize,
    pub class_index: i32,
    pub super_index: i32,
    pub outer: i32,
    pub object_name: String,
    pub object_name_index: i32,
    pub serial_size: usize,
    pub serial_offset: u64,
}

pub fn parse_imports(
    region: &HeaderRegion,
    summary: &PackageSummary,
    names: &NameTable,
) -> Vec<Import> {
    let Some(start) = region.rel(summary.import_offset) else {
        return Vec::new();
    };
    let mut r = Reader::at(&region.plain, start);
    let mut out = Vec::with_capacity(summary.import_count.min(65_536));
    for index in 0..summary.import_count {
        let entry = (|| {
            let class_package = r.i32("import")?;
            r.skip(4, "import")?;
            let class_name = r.i32("import")?;
            r.skip(4, "import")?;
            let outer = r.i32("import")?;
            let object_name = r.i32("import")?;
            r.skip(4, "import")?;
            Ok::<_, crate::upk::UpkError>(Import {
                index,
                class_package: names.get(class_package).to_string(),
                class_name: names.get(class_name).to_string(),
                outer,
                object_name: names.get(object_name).to_string(),
            })
        })();
        match entry {
            Ok(i) => out.push(i),
            Err(_) => break,
        }
    }
    out
}

pub fn parse_exports(
    region: &HeaderRegion,
    summary: &PackageSummary,
    names: &NameTable,
) -> Vec<Export> {
    let Some(start) = region.rel(summary.export_offset) else {
        return Vec::new();
    };
    let mut r = Reader::at(&region.plain, start);
    let mut out = Vec::with_capacity(summary.export_count.min(65_536));
    for index in 0..summary.export_count {
        let entry = (|| {
            let class_index = r.i32("export")?;
            let super_index = r.i32("export")?;
            let outer = r.i32("export")?;
            let object_name_index = r.i32("export")?;
            r.skip(4, "export")?; // name number
            r.skip(4, "export")?; // archetype
            r.skip(8, "export")?; // object flags
            let serial_size = r.i32("export")?.max(0) as usize;
            let serial_offset = r.u64("export")?;
            r.skip(4, "export")?; // export flags
            let net_count = r.i32("export")?;
            if !(0..=65_536).contains(&net_count) {
                return Err(crate::upk::UpkError::Format("bad net object count".into()));
            }
            r.skip(net_count as usize * 4 + 16 + 4, "export")?;
            Ok::<_, crate::upk::UpkError>(Export {
                index,
                class_index,
                super_index,
                outer,
                object_name: names.get(object_name_index).to_string(),
                object_name_index,
                serial_size,
                serial_offset,
            })
        })();
        match entry {
            Ok(e) => out.push(e),
            Err(_) => break,
        }
    }
    out
}

/// Offsets (inside `region.plain`) of every name-index field of the import
/// and export tables: an import's class package, class name and object
/// name, an export's object name. These are the only header fields that
/// hold FName indices. Strict: fails unless both tables parse completely.
pub fn name_ref_offsets(region: &HeaderRegion, summary: &PackageSummary) -> UpkResult<Vec<usize>> {
    let mut out = Vec::with_capacity(summary.import_count * 3 + summary.export_count);
    if summary.import_count > 0 {
        let start = region
            .rel(summary.import_offset)
            .ok_or(UpkError::Truncated("import table"))?;
        for i in 0..summary.import_count {
            let entry = start + i * IMPORT_SIZE;
            if entry + IMPORT_SIZE > region.plain.len() {
                return Err(UpkError::Truncated("import table"));
            }
            out.extend([entry, entry + 8, entry + 20]);
        }
    }
    if summary.export_count > 0 {
        let start = region
            .rel(summary.export_offset)
            .ok_or(UpkError::Truncated("export table"))?;
        let mut r = Reader::at(&region.plain, start);
        for _ in 0..summary.export_count {
            let entry = r.pos();
            r.skip(EXPORT_FIXED - 4, "export table")?;
            let net_count = r.i32("export table")?;
            if !(0..=65_536).contains(&net_count) {
                return Err(UpkError::Format("bad net object count".into()));
            }
            r.skip(net_count as usize * 4 + 16 + 4, "export table")?;
            out.push(entry + 12);
        }
    }
    Ok(out)
}

/// Class name of an export (`class_index` < 0 → import, > 0 → export, 0 → UClass).
pub fn class_name<'a>(export: &Export, imports: &'a [Import], exports: &'a [Export]) -> &'a str {
    match export.class_index {
        i if i < 0 => imports
            .get((-i - 1) as usize)
            .map_or("", |imp| imp.object_name.as_str()),
        i if i > 0 => exports
            .get((i - 1) as usize)
            .map_or("", |e| e.object_name.as_str()),
        _ => "Class",
    }
}

/// Name of whatever an ObjectIndex points to.
pub fn object_name<'a>(index: i32, imports: &'a [Import], exports: &'a [Export]) -> &'a str {
    match index {
        i if i < 0 => imports
            .get((-i - 1) as usize)
            .map_or("", |imp| imp.object_name.as_str()),
        i if i > 0 => exports
            .get((i - 1) as usize)
            .map_or("", |e| e.object_name.as_str()),
        _ => "",
    }
}
