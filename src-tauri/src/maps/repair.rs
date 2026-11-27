//! Map repair: re-points a custom map's references to game objects that a
//! Rocket League update moved or renamed (e.g. Mannfield's
//! `FutureTech.Materials.ForceField_Mat`, now only in `CHN_Stadium_Cage`;
//! `Proto_World01.SM.SkySphere01`, moved from `Park_P` to `Startup.upk`).
//! An import the game cannot find renders with the engine's default
//! material: the white/blue checkerboard on walls and skies.
//!
//! The repair is a pure function of (original package bytes, mapping
//! version): every player who repairs the same map with the same mapping
//! gets the exact same file, so multiplayer only compares hashes.
//!
//! Supported: uncompressed UDK packages (licensee 0), i.e. custom maps.
//! The fixed import keeps its index (exports referencing it are untouched);
//! its outer chain is rebuilt from reused or appended `Package` imports and
//! its object name re-pointed. Names and imports are appended at the end of
//! their tables, and every absolute offset after them is shifted: summary
//! offsets, export serial offsets, inline bulk data (texture mips,
//! lightmaps…, whose header stores its own file offset) and the thumbnail
//! table. Mappings are bundled (`repair_mappings/vN.json`) and all versions
//! stay available.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::base::error::{AppError, AppResult};

const TAG: u32 = 0x9E2A_83C1;
const IMPORT_SIZE: usize = 28;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// Import path as written in the map (`EuroStadium_P.futuretech.Materials.ForceField_Mat`).
    pub from: String,
    /// Where the object lives now (top package first).
    pub to: String,
    /// Class of the object (rules only apply to imports of that class).
    pub class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub version: u32,
    pub label: String,
    pub released: String,
    pub game_build: String,
    pub rules: Vec<Rule>,
}

/// Bundled mappings, oldest first.
pub fn mappings() -> &'static [Mapping] {
    static ALL: OnceLock<Vec<Mapping>> = OnceLock::new();
    ALL.get_or_init(|| {
        // Invariant: the files are checked in and covered by a test.
        let mut all: Vec<Mapping> = [include_str!("repair_mappings/v1.json")]
            .iter()
            .map(|raw| serde_json::from_str(raw).expect("valid bundled repair mapping"))
            .collect();
        all.sort_by_key(|m| m.version);
        all
    })
}

pub fn latest() -> &'static Mapping {
    // Invariant: at least one mapping is bundled.
    mappings().last().expect("a bundled repair mapping")
}

pub fn mapping(version: u32) -> AppResult<&'static Mapping> {
    mappings()
        .iter()
        .find(|m| m.version == version)
        .ok_or_else(|| AppError::NotFound(format!("repair mapping v{version}")))
}

/// One reference re-pointed by a repair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Fix {
    pub from: String,
    pub to: String,
}

fn bad(msg: impl Into<String>) -> AppError {
    AppError::Unsupported(format!("map repair: {}", msg.into()))
}

fn rd_i32(b: &[u8], pos: usize) -> AppResult<i32> {
    b.get(pos..pos + 4)
        .map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| bad("truncated package"))
}

fn wr_i32(b: &mut [u8], pos: usize, v: i32) {
    b[pos..pos + 4].copy_from_slice(&v.to_le_bytes());
}

fn to_usize(v: i32) -> AppResult<usize> {
    usize::try_from(v).map_err(|_| bad("negative offset"))
}

/// Positions of the summary fields the repair rewrites.
struct Summary {
    total_header: usize,
    counts: usize,
    depends: usize,
    import_export_guids: usize,
    thumbnails: usize,
    generations: usize,
    generation_count: usize,
    name_count: usize,
    name_offset: usize,
    export_count: usize,
    export_offset: usize,
    import_count: usize,
    import_offset: usize,
}

fn summary(b: &[u8]) -> AppResult<Summary> {
    if rd_i32(b, 0)? as u32 != TAG {
        return Err(bad("not an Unreal package"));
    }
    let lic = u16::from_le_bytes([b[6], b[7]]);
    if lic != 0 {
        return Err(bad("only UDK custom maps can be repaired"));
    }
    let mut o = 8;
    let total_header = o;
    o += 4;
    let folder_len = rd_i32(b, o)?;
    o += 4 + if folder_len >= 0 {
        to_usize(folder_len)?
    } else {
        to_usize(-folder_len)? * 2
    };
    o += 4; // package flags
    let counts = o;
    let name_count = to_usize(rd_i32(b, o)?)?;
    let name_offset = to_usize(rd_i32(b, o + 4)?)?;
    let export_count = to_usize(rd_i32(b, o + 8)?)?;
    let export_offset = to_usize(rd_i32(b, o + 12)?)?;
    let import_count = to_usize(rd_i32(b, o + 16)?)?;
    let import_offset = to_usize(rd_i32(b, o + 20)?)?;
    o += 24;
    let depends = o;
    o += 4;
    let import_export_guids = o;
    o += 12;
    let thumbnails = o;
    o += 4 + 16;
    let generations = o;
    let generation_count = to_usize(rd_i32(b, o)?)?;
    o += 4 + generation_count * 12 + 8;
    let compression = rd_i32(b, o)?;
    let chunks = rd_i32(b, o + 4)?;
    if compression != 0 || chunks != 0 {
        return Err(bad("compressed packages are not supported"));
    }
    if !(name_offset < import_offset && import_offset < export_offset) {
        return Err(bad("unexpected table layout"));
    }
    Ok(Summary {
        total_header,
        counts,
        depends,
        import_export_guids,
        thumbnails,
        generations,
        generation_count,
        name_count,
        name_offset,
        export_count,
        export_offset,
        import_count,
        import_offset,
    })
}

/// Name table: text and the 8 flag bytes of each entry, plus where it ends.
fn read_names(b: &[u8], s: &Summary) -> AppResult<(Vec<String>, Vec<[u8; 8]>, usize)> {
    let (mut names, mut flags, mut p) =
        (Vec::with_capacity(s.name_count), Vec::new(), s.name_offset);
    for _ in 0..s.name_count {
        let len = rd_i32(b, p)?;
        p += 4;
        let text = if len >= 0 {
            let n = to_usize(len)?;
            let raw = b.get(p..p + n).ok_or_else(|| bad("truncated names"))?;
            p += n;
            raw.iter().map(|&c| c as char).collect::<String>()
        } else {
            let n = to_usize(-len)? * 2;
            let raw = b.get(p..p + n).ok_or_else(|| bad("truncated names"))?;
            p += n;
            let units: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        };
        let f = b.get(p..p + 8).ok_or_else(|| bad("truncated names"))?;
        flags.push([f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7]]);
        p += 8;
        names.push(text.trim_end_matches('\0').to_string());
    }
    Ok((names, flags, p))
}

#[derive(Debug, Clone, Copy)]
struct Import {
    class_package: i32,
    class_name: i32,
    outer: i32,
    name: i32,
    name_number: i32,
}

fn read_imports(b: &[u8], s: &Summary) -> AppResult<Vec<Import>> {
    (0..s.import_count)
        .map(|i| {
            let p = s.import_offset + i * IMPORT_SIZE;
            Ok(Import {
                class_package: rd_i32(b, p)?,
                class_name: rd_i32(b, p + 8)?,
                outer: rd_i32(b, p + 16)?,
                name: rd_i32(b, p + 20)?,
                name_number: rd_i32(b, p + 24)?,
            })
        })
        .collect()
}

/// Names + imports being edited, with path lookups.
struct Tables {
    names: Vec<String>,
    imports: Vec<Import>,
}

impl Tables {
    fn name(&self, i: i32) -> &str {
        usize::try_from(i)
            .ok()
            .and_then(|i| self.names.get(i))
            .map_or("", String::as_str)
    }

    fn path(&self, index: usize) -> String {
        let mut parts = Vec::new();
        let mut cur = -(index as i32) - 1;
        let mut guard = 0;
        while cur < 0 && guard < 64 {
            guard += 1;
            let Some(imp) = usize::try_from(-cur - 1)
                .ok()
                .and_then(|i| self.imports.get(i))
            else {
                break;
            };
            parts.push(self.name(imp.name).to_string());
            cur = imp.outer;
        }
        parts.reverse();
        parts.join(".")
    }

    fn name_index(&mut self, text: &str) -> i32 {
        match self.names.iter().position(|n| n.eq_ignore_ascii_case(text)) {
            Some(i) => i as i32,
            None => {
                self.names.push(text.to_string());
                (self.names.len() - 1) as i32
            }
        }
    }

    /// Reuses or appends a `Core.Package` import named `name` under `outer`.
    fn package(&mut self, outer: i32, name: &str) -> i32 {
        let ni = self.name_index(name);
        if let Some(i) = self.imports.iter().position(|imp| {
            imp.outer == outer && imp.name == ni && self.name(imp.class_name) == "Package"
        }) {
            return -(i as i32) - 1;
        }
        let (core, package) = (self.name_index("Core"), self.name_index("Package"));
        self.imports.push(Import {
            class_package: core,
            class_name: package,
            outer,
            name: ni,
            name_number: 0,
        });
        -(self.imports.len() as i32)
    }
}

/// Rules of `mapping` that apply to the package (import path and class match).
fn matching(tables: &Tables, mapping: &Mapping) -> Vec<(usize, Rule)> {
    let mut out = Vec::new();
    for rule in &mapping.rules {
        if let Some(i) = (0..tables.imports.len()).find(|&i| {
            tables.path(i).eq_ignore_ascii_case(&rule.from)
                && tables.name(tables.imports[i].class_name) == rule.class
        }) {
            if !out.iter().any(|(j, _)| *j == i) {
                out.push((i, rule.clone()));
            }
        }
    }
    out
}

/// Fixes `mapping` would apply, reading only the package header.
pub fn preview(header: &[u8], mapping: &Mapping) -> AppResult<Vec<Fix>> {
    let s = summary(header)?;
    let (names, _, _) = read_names(header, &s)?;
    let tables = Tables {
        names,
        imports: read_imports(header, &s)?,
    };
    Ok(matching(&tables, mapping)
        .into_iter()
        .map(|(_, r)| Fix {
            from: r.from,
            to: r.to,
        })
        .collect())
}

/// Bytes to read for [`preview`]: the whole header (`TotalHeaderSize`).
pub fn header_len(first_bytes: &[u8]) -> AppResult<usize> {
    summary(first_bytes)?;
    to_usize(rd_i32(first_bytes, 8)?)
}

pub struct Repaired {
    pub bytes: Vec<u8>,
    pub fixes: Vec<Fix>,
}

/// Repairs `original` with `mapping`. Deterministic. No fix to apply returns
/// the original bytes and an empty list.
pub fn repair(original: &[u8], mapping: &Mapping) -> AppResult<Repaired> {
    let s = summary(original)?;
    let (names, flags, names_end) = read_names(original, &s)?;
    let mut t = Tables {
        names,
        imports: read_imports(original, &s)?,
    };
    let rules = matching(&t, mapping);
    if rules.is_empty() {
        return Ok(Repaired {
            bytes: original.to_vec(),
            fixes: Vec::new(),
        });
    }
    let (name_count, import_count) = (t.names.len(), t.imports.len());
    let mut fixes = Vec::new();
    for (i, rule) in rules {
        let parts: Vec<&str> = rule.to.split('.').collect();
        let Some((leaf, groups)) = parts.split_last() else {
            continue;
        };
        let mut outer = 0;
        for g in groups {
            outer = t.package(outer, g);
        }
        let leaf = t.name_index(leaf);
        let imp = &mut t.imports[i];
        imp.outer = outer;
        imp.name = leaf;
        imp.name_number = 0;
        fixes.push(Fix {
            from: rule.from,
            to: rule.to,
        });
    }

    // New names: same flags as an existing package name.
    let name_flags = t
        .names
        .iter()
        .position(|n| n == "Core")
        .filter(|&i| i < flags.len())
        .map_or([0, 0, 0, 0, 0x10, 0, 0x07, 0], |i| flags[i]);
    let mut name_blob = Vec::new();
    for n in &t.names[name_count..] {
        let mut text: Vec<u8> = n.chars().map(|c| c as u8).collect();
        text.push(0);
        name_blob.extend_from_slice(&(text.len() as i32).to_le_bytes());
        name_blob.extend_from_slice(&text);
        name_blob.extend_from_slice(&name_flags);
    }
    let mut import_blob = Vec::with_capacity(t.imports.len() * IMPORT_SIZE);
    for imp in &t.imports {
        for v in [
            imp.class_package,
            0,
            imp.class_name,
            0,
            imp.outer,
            imp.name,
            imp.name_number,
        ] {
            import_blob.extend_from_slice(&v.to_le_bytes());
        }
    }
    let imports_end = s.import_offset + import_count * IMPORT_SIZE;
    let d1 = name_blob.len();
    let delta = d1 + (t.imports.len() - import_count) * IMPORT_SIZE;
    let shift = |v: usize| {
        if v >= imports_end {
            v + delta
        } else if v >= names_end {
            v + d1
        } else {
            v
        }
    };

    let mut out = Vec::with_capacity(original.len() + delta);
    out.extend_from_slice(&original[..names_end]);
    out.extend_from_slice(&name_blob);
    out.extend_from_slice(&original[names_end..s.import_offset]);
    out.extend_from_slice(&import_blob);
    out.extend_from_slice(&original[imports_end..]);

    // Summary (before the name table: positions unchanged).
    wr_i32(&mut out, s.counts, t.names.len() as i32);
    wr_i32(&mut out, s.counts + 16, t.imports.len() as i32);
    for pos in [
        s.total_header,
        s.counts + 12,
        s.counts + 20,
        s.depends,
        s.import_export_guids,
        s.thumbnails,
    ] {
        let v = rd_i32(&out, pos)?;
        if v > 0 {
            wr_i32(&mut out, pos, shift(to_usize(v)?) as i32);
        }
    }
    if s.generation_count > 0 {
        let last = s.generations + 4 + (s.generation_count - 1) * 12;
        wr_i32(&mut out, last + 4, t.names.len() as i32);
    }

    // Exports: serial offsets and the inline bulk data they contain.
    let mut p = shift(s.export_offset);
    for _ in 0..s.export_count {
        let size = rd_i32(&out, p + 32)?;
        let off = rd_i32(&out, p + 36)?;
        if size > 0 && off > 0 {
            let new_off = shift(to_usize(off)?);
            wr_i32(&mut out, p + 36, new_off as i32);
            let end = new_off + to_usize(size)?;
            let mut q = new_off;
            while q + 16 <= end.min(out.len()) {
                let size_on_disk = rd_i32(&out, q + 8)?;
                let offset_in_file = rd_i32(&out, q + 12)?;
                // Inline bulk data points right after its own header (in
                // the original file).
                if usize::try_from(offset_in_file).ok() == Some(q - delta + 16)
                    && size_on_disk >= 0
                    && (size_on_disk as usize) <= end - (q + 16)
                {
                    wr_i32(&mut out, q + 12, offset_in_file + delta as i32);
                    q += 16 + size_on_disk as usize;
                    continue;
                }
                q += 1;
            }
        }
        let net = to_usize(rd_i32(&out, p + 44)?)?;
        p += 48 + 4 * net + 20;
    }

    // Thumbnail table: (class FString, path FString, file offset).
    let thumbs = rd_i32(&out, s.thumbnails)?;
    if thumbs > 0 {
        let mut q = to_usize(thumbs)?;
        let n = to_usize(rd_i32(&out, q)?)?;
        q += 4;
        for _ in 0..n {
            for _ in 0..2 {
                let l = rd_i32(&out, q)?;
                q += 4 + if l >= 0 {
                    to_usize(l)?
                } else {
                    to_usize(-l)? * 2
                };
            }
            let v = rd_i32(&out, q)?;
            if v > 0 {
                wr_i32(&mut out, q, shift(to_usize(v)?) as i32);
            }
            q += 4;
        }
    }
    Ok(Repaired { bytes: out, fixes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fstring(s: &str) -> Vec<u8> {
        let mut v = ((s.len() + 1) as i32).to_le_bytes().to_vec();
        v.extend_from_slice(s.as_bytes());
        v.push(0);
        v
    }

    /// Minimal UDK package: names, 4 imports (Old_P.grp.Thing [Material]),
    /// one export whose data holds an inline bulk block.
    fn package() -> Vec<u8> {
        let names = [
            "Core", "Package", "Material", "Old_P", "grp", "Thing", "Mine",
        ];
        let mut summary = Vec::new();
        summary.extend_from_slice(&TAG.to_le_bytes());
        summary.extend_from_slice(&867u16.to_le_bytes());
        summary.extend_from_slice(&0u16.to_le_bytes());
        let total_header_pos = summary.len();
        summary.extend_from_slice(&0i32.to_le_bytes()); // total header (patched)
        summary.extend_from_slice(&fstring("None"));
        summary.extend_from_slice(&0u32.to_le_bytes()); // flags
        let counts = summary.len();
        summary.extend_from_slice(&[0u8; 24]); // counts/offsets (patched)
        summary.extend_from_slice(&0i32.to_le_bytes()); // depends
        summary.extend_from_slice(&[0u8; 12]); // import/export guids
        summary.extend_from_slice(&0i32.to_le_bytes()); // thumbnails
        summary.extend_from_slice(&[0u8; 16]); // guid
        summary.extend_from_slice(&1i32.to_le_bytes()); // generations
        summary.extend_from_slice(&[0u8; 12]);
        summary.extend_from_slice(&[0u8; 8]); // engine/cooker
        summary.extend_from_slice(&[0u8; 8]); // compression, chunks

        let name_off = summary.len();
        let mut body = summary;
        for n in names {
            body.extend_from_slice(&fstring(n));
            body.extend_from_slice(&[0, 0, 0, 0, 0x10, 0, 0x07, 0]);
        }
        let imp_off = body.len();
        // -1 Old_P (Package), -2 grp (Package in -1), -3 Thing (Material in -2), -4 Mine? no: 3 imports
        for (class, outer, name) in [(1, 0, 3), (1, -1, 4), (2, -2, 5)] {
            for v in [0i32, 0, class, 0, outer, name, 0] {
                body.extend_from_slice(&v.to_le_bytes());
            }
        }
        let exp_off = body.len();
        let exp_entry_pos = body.len();
        body.extend_from_slice(&[0u8; 48 + 16 + 4]); // one export, NetObjectCount 0
        let total_header = body.len();
        let data_pos = body.len();
        let mut data = vec![0xAA; 6];
        let bulk_hdr = data_pos + data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // flags
        data.extend_from_slice(&4i32.to_le_bytes()); // count
        data.extend_from_slice(&4i32.to_le_bytes()); // size on disk
        data.extend_from_slice(&((bulk_hdr + 16) as i32).to_le_bytes());
        data.extend_from_slice(&[1, 2, 3, 4, 0xBB]);
        body.extend_from_slice(&data);
        wr_i32(&mut body, total_header_pos, total_header as i32);
        for (k, v) in [names.len(), name_off, 1, exp_off, 3, imp_off]
            .iter()
            .enumerate()
        {
            wr_i32(&mut body, counts + 4 * k, *v as i32);
        }
        wr_i32(&mut body, exp_entry_pos + 12, 6); // object name "Mine"
        wr_i32(&mut body, exp_entry_pos + 32, data.len() as i32);
        wr_i32(&mut body, exp_entry_pos + 36, data_pos as i32);
        body
    }

    fn make_mapping(rules: Vec<Rule>) -> Mapping {
        Mapping {
            version: 9,
            label: String::new(),
            released: String::new(),
            game_build: String::new(),
            rules,
        }
    }

    #[test]
    fn bundled_mappings_load() {
        assert!(!mappings().is_empty());
        assert_eq!(latest().version, mappings().last().unwrap().version);
        assert!(super::mapping(1)
            .unwrap()
            .rules
            .iter()
            .any(|r| r.to.ends_with("SkySphere01")));
    }

    #[test]
    fn repairs_and_shifts_everything() {
        let original = package();
        let m = make_mapping(vec![Rule {
            from: "Old_P.grp.Thing".into(),
            to: "New_P.grp2.Thing2".into(),
            class: "Material".into(),
        }]);
        assert_eq!(preview(&original, &m).unwrap().len(), 1);
        let r = repair(&original, &m).unwrap();
        assert_eq!(r.fixes.len(), 1);
        let s = summary(&r.bytes).unwrap();
        let (names, _, _) = read_names(&r.bytes, &s).unwrap();
        let t = Tables {
            names,
            imports: read_imports(&r.bytes, &s).unwrap(),
        };
        assert_eq!(t.path(2), "New_P.grp2.Thing2");
        // Export data moved and its bulk block still points right after its header.
        let exp = s.export_offset;
        let size = rd_i32(&r.bytes, exp + 32).unwrap() as usize;
        let off = rd_i32(&r.bytes, exp + 36).unwrap() as usize;
        let data = &r.bytes[off..off + size];
        assert_eq!(&data[..6], &[0xAA; 6]);
        let bulk_off = rd_i32(data, 6 + 12).unwrap() as usize;
        assert_eq!(bulk_off, off + 6 + 16);
        assert_eq!(&r.bytes[bulk_off..bulk_off + 4], &[1, 2, 3, 4]);
        assert_eq!(rd_i32(&r.bytes, 8).unwrap() as usize, off); // total header
                                                                // Deterministic, and nothing left to fix afterwards.
        assert_eq!(repair(&original, &m).unwrap().bytes, r.bytes);
        assert!(preview(&r.bytes, &m).unwrap().is_empty());
    }

    #[test]
    fn rules_need_the_right_class_and_skip_cooked_packages() {
        let original = package();
        let m = make_mapping(vec![Rule {
            from: "Old_P.grp.Thing".into(),
            to: "X.Thing".into(),
            class: "StaticMesh".into(),
        }]);
        let r = repair(&original, &m).unwrap();
        assert!(r.fixes.is_empty());
        assert_eq!(r.bytes, original);
        let mut cooked = original.clone();
        cooked[6] = 34; // licensee 34 (Rocket League cooked)
        assert!(repair(&cooked, &m).is_err());
    }
}
