//! Dev probe: where Rocket League keeps its item paint colours.
//!
//! - `cargo run --example paint_probe -- pkg <package> [class-substring]...`
//!   dumps the tagged properties of every export whose class contains one of
//!   the substrings (default: `Paint`), plus the imports mentioning `Paint`.
//! - `cargo run --example paint_probe -- raw <package> <out.bin>` writes the
//!   whole decompressed chunk stream (works on `TAGame.upk`, whose header
//!   tables are chunked too) for offline searching.
//! - `cargo run --example paint_probe -- find <package> <needle>...` prints
//!   every offset of each ASCII needle in the decompressed stream, with the
//!   floats that follow it.
use alxs_rl_mod_lib::upk::chunks::{self, ChunkMap};
use alxs_rl_mod_lib::upk::names::NameTable;
use alxs_rl_mod_lib::upk::props;
use alxs_rl_mod_lib::upk::reader::{read_i32, read_u32};

fn f32_at(buf: &[u8], pos: usize) -> f32 {
    f32::from_bits(read_u32(buf, pos).unwrap_or(0))
}

fn dump(buf: &[u8], start: usize, names: &NameTable, depth: usize) {
    let Ok((list, _)) = props::walk(buf, start, names) else {
        println!("{}(unreadable)", "  ".repeat(depth));
        return;
    };
    for p in list {
        let pad = "  ".repeat(depth);
        let inner = p.inner.clone().unwrap_or_default();
        print!(
            "{pad}{}[{}] : {} {} ({}B)",
            p.name, p.array_index, p.type_name, inner, p.size
        );
        match p.type_name.as_str() {
            "FloatProperty" => println!(" = {}", f32_at(buf, p.value_pos)),
            "IntProperty" => println!(" = {:?}", p.as_i32(buf)),
            "BoolProperty" => println!(" = {:?}", p.as_bool(buf)),
            "ByteProperty" | "NameProperty" => {
                println!(" = {:?}", p.as_name(buf, names).or(Some("?")))
            }
            "ObjectProperty" | "ClassProperty" => println!(" = obj {:?}", p.as_i32(buf)),
            "StrProperty" => {
                let n = read_i32(buf, p.value_pos).unwrap_or(0).max(0) as usize;
                let s = buf
                    .get(p.value_pos + 4..p.value_pos + 4 + n.saturating_sub(1))
                    .map(String::from_utf8_lossy)
                    .unwrap_or_default();
                println!(" = {s:?}");
            }
            "StructProperty"
                if matches!(
                    inner.as_str(),
                    "LinearColor" | "Vector" | "Color" | "Vector4"
                ) =>
            {
                if inner == "Color" {
                    let b = &buf[p.value_pos..p.value_pos + 4];
                    println!(" = BGRA {b:?}");
                } else {
                    let v: Vec<f32> = (0..(p.size / 4))
                        .map(|i| f32_at(buf, p.value_pos + i * 4))
                        .collect();
                    println!(" = {v:?}");
                }
            }
            "StructProperty" => {
                println!();
                dump(buf, p.value_pos, names, depth + 1);
            }
            "ArrayProperty" => {
                let n = read_i32(buf, p.value_pos).unwrap_or(0);
                let elem = if n > 0 { (p.size - 4) / n as usize } else { 0 };
                println!(" n={n} elem~{elem}B");
                // Try: struct elements (tagged), else raw ints (object refs).
                let mut at = p.value_pos + 4;
                let mut tagged = true;
                for _ in 0..n.min(64) {
                    match props::walk(buf, at, names) {
                        Ok((l, next)) if !l.is_empty() => {
                            println!("{pad}  -");
                            dump(buf, at, names, depth + 2);
                            at = next;
                        }
                        _ => {
                            tagged = false;
                            break;
                        }
                    }
                }
                if !tagged {
                    let v: Vec<i32> = (0..(p.size - 4) / 4)
                        .take(64)
                        .map(|i| read_i32(buf, p.value_pos + 4 + i * 4).unwrap_or(0))
                        .collect();
                    println!("{pad}  raw i32 {v:?}");
                }
            }
            _ => println!(),
        }
    }
}

fn stream(path: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).expect("read package");
    let map = ChunkMap::scan(&bytes).expect("chunk scan");
    chunks::decompress_all(&bytes, &map).expect("decompress")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("mode: pkg | raw | find");
    let path = args.next().expect("package path");
    let rest: Vec<String> = args.collect();
    match mode.as_str() {
        "raw" => {
            let data = stream(&path);
            let out = rest.first().expect("out path");
            std::fs::write(out, &data).expect("write");
            println!("{} bytes -> {out}", data.len());
        }
        "find" => {
            let data = stream(&path);
            for needle in &rest {
                let n = needle.as_bytes();
                let hits: Vec<usize> = data
                    .windows(n.len())
                    .enumerate()
                    .filter(|(_, w)| *w == n)
                    .map(|(i, _)| i)
                    .collect();
                println!("{needle}: {} hits", hits.len());
                for h in hits.iter().take(20) {
                    let v: Vec<f32> = (0..8)
                        .map(|i| f32_at(&data, h + n.len() + 1 + i * 4))
                        .collect();
                    println!("  @{h} next floats {v:?}");
                }
            }
        }
        "definers" => {
            // `path` is the CookedPCConsole dir: packages that EXPORT an
            // object of class ProductPaint_TA (or named Paints/ProductPaint).
            let keys = std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/keys/keys.txt"
            ))
            .unwrap_or_default();
            let ring = alxs_rl_mod_lib::upk::KeyRing::from_text(&keys, None);
            let mut entries: Vec<_> = std::fs::read_dir(&path)
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case("upk"))
                })
                .collect();
            entries.sort();
            for p in entries {
                let Ok(bytes) = std::fs::read(&p) else {
                    continue;
                };
                let Ok(pkg) = alxs_rl_mod_lib::upk::Package::open(bytes, &ring) else {
                    println!("(unopenable) {}", p.display());
                    continue;
                };
                let hits: Vec<String> = pkg
                    .exports
                    .iter()
                    .filter(|e| {
                        let c = pkg.class_of(e);
                        c == "ProductPaint_TA"
                            || e.object_name == "Paints"
                            || e.object_name == "ProductPaint"
                            || c.contains("PaintDatabase")
                    })
                    .map(|e| format!("{}:{}", e.object_name, pkg.class_of(e)))
                    .collect();
                if !hits.is_empty() {
                    println!("{} {:?}", p.display(), hits);
                }
            }
        }
        "pkg" => {
            let keys = std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/keys/keys.txt"
            ))
            .unwrap_or_default();
            let ring = alxs_rl_mod_lib::upk::KeyRing::from_text(&keys, None);
            let pkg = alxs_rl_mod_lib::upk::Package::open(std::fs::read(&path).unwrap(), &ring)
                .expect("open");
            if rest.first().map(String::as_str) == Some("--who") {
                // Where exports with these exact names live (class + outer).
                // `in:<Name>` lists the children of exports named <Name>
                // (e.g. the properties of a class or struct).
                let outer_of = |e: &alxs_rl_mod_lib::upk::tables::Export| {
                    usize::try_from(e.outer - 1)
                        .ok()
                        .and_then(|i| pkg.exports.get(i))
                        .map(|o| o.object_name.as_str())
                        .unwrap_or("-")
                };
                for e in pkg.exports.iter().filter(|e| {
                    rest[1..].iter().any(|w| match w.strip_prefix("in:") {
                        Some(parent) => outer_of(e) == parent,
                        None => *w == e.object_name,
                    })
                }) {
                    let outer = outer_of(e);
                    println!(
                        "[{}] {} : {} in {outer}",
                        e.index + 1,
                        e.object_name,
                        pkg.class_of(e)
                    );
                }
                return;
            }
            if rest.first().map(String::as_str) == Some("--raw") {
                // `--raw <out_dir> <export#>...`: serial bytes of exports
                // (1-based, as printed) + names.txt / exports.txt for decoding.
                let out = std::path::PathBuf::from(&rest[1]);
                std::fs::create_dir_all(&out).unwrap();
                let names: Vec<&str> = pkg.names.entries.iter().map(|n| n.name.as_str()).collect();
                std::fs::write(out.join("names.txt"), names.join("\n")).unwrap();
                let exports: Vec<String> = pkg
                    .exports
                    .iter()
                    .map(|e| format!("{}\t{}\t{}", e.object_name, pkg.class_of(e), e.outer))
                    .collect();
                std::fs::write(out.join("exports.txt"), exports.join("\n")).unwrap();
                let imports: Vec<String> =
                    pkg.imports.iter().map(|i| i.object_name.clone()).collect();
                std::fs::write(out.join("imports.txt"), imports.join("\n")).unwrap();
                let body = pkg.body().expect("body");
                for idx in &rest[2..] {
                    let i: usize = idx.parse().unwrap();
                    let e = &pkg.exports[i - 1];
                    let pos = body.export_pos(e).unwrap();
                    std::fs::write(
                        out.join(format!("{i}.bin")),
                        &body.data[pos..pos + e.serial_size],
                    )
                    .unwrap();
                    println!("{i} {} {}B", e.object_name, e.serial_size);
                }
                return;
            }
            if rest.first().map(String::as_str) == Some("--enum") {
                // Enum exports whose name contains the substring: UE3 serialises
                // `props (None) · Next · i32 count · FName[count]` at the end.
                let needle = rest.get(1).cloned().unwrap_or_default();
                let body = pkg.body().expect("body");
                for e in pkg.exports.iter().filter(|e| {
                    pkg.class_of(e) == "Enum" && e.object_name.contains(needle.as_str())
                }) {
                    let Some(pos) = body.export_pos(e) else {
                        continue;
                    };
                    let raw = &body.data[pos..pos + e.serial_size];
                    // Find the count whose FNames fill the tail exactly.
                    let mut members = Vec::new();
                    for start in (0..raw.len().saturating_sub(4)).rev() {
                        let n = read_i32(raw, start).unwrap_or(-1);
                        if n > 0 && start + 4 + n as usize * 8 == raw.len() {
                            members = (0..n as usize)
                                .map(|k| {
                                    pkg.names.get(read_i32(raw, start + 4 + k * 8).unwrap_or(0))
                                })
                                .collect();
                            break;
                        }
                    }
                    println!("enum {} = {:?}", e.object_name, members);
                }
                return;
            }
            let filters: Vec<String> = if rest.is_empty() {
                vec!["Paint".into()]
            } else {
                rest
            };
            for i in pkg
                .imports
                .iter()
                .filter(|i| i.object_name.contains("Paint") || i.class_name.contains("Paint"))
            {
                println!(
                    "import [-{}] {} ({}.{}) outer={}",
                    i.index + 1,
                    i.object_name,
                    i.class_package,
                    i.class_name,
                    i.outer
                );
            }
            let body = pkg.body().expect("body");
            for e in &pkg.exports {
                let class = pkg.class_of(e);
                if !filters.iter().any(|f| class.contains(f.as_str())) {
                    continue;
                }
                let Some(pos) = body.export_pos(e) else {
                    continue;
                };
                println!(
                    "== [{}] {} ({class}) size={} outer={}",
                    e.index + 1,
                    e.object_name,
                    e.serial_size,
                    e.outer
                );
                for skip in [4usize, 0, 8, 12, 16] {
                    if let Ok((l, _)) = props::walk(&body.data, pos + skip, &pkg.names) {
                        if l.is_empty() && skip != 16 {
                            continue;
                        }
                        println!("  (props at +{skip})");
                        dump(&body.data, pos + skip, &pkg.names, 1);
                        break;
                    }
                }
            }
        }
        _ => panic!("unknown mode {mode}"),
    }
}
