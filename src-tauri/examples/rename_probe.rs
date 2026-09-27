//! Dev probe: rename a package like the swap engine does and report which
//! header fields changed (names, imports, exports) — used to chase the
//! "Bad export index" crash.
//!
//! `cargo run --example rename_probe -- <cooked dir> <source.upk> <target.upk>`
use alxs_rl_mod_lib::upk::{
    crypto, keys::KeyRing, names::NameTable, summary::PackageSummary, tables,
};

fn dump(
    bytes: &[u8],
    ring: &KeyRing,
) -> (
    NameTable,
    Vec<tables::Import>,
    Vec<tables::Export>,
    Vec<u8>,
    PackageSummary,
) {
    let s = PackageSummary::parse(bytes).unwrap();
    let h = crypto::open_header(bytes, &s, ring).unwrap();
    let names = NameTable::parse(&h.plain, s.name_count);
    let imports = tables::parse_imports(&h, &s, &names);
    let exports = tables::parse_exports(&h, &s, &names);
    (names, imports, exports, h.plain, s)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cooked = std::path::PathBuf::from(args.next().expect("cooked dir"));
    let source = args.next().expect("source package");
    let target = args.next().expect("target package");
    let keys_txt = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap();
    let ring = KeyRing::from_text(&keys_txt, None);
    let bytes = std::fs::read(cooked.join(&source)).unwrap();
    let rules = alxs_rl_mod_lib::swap::rules::derive_rules(
        alxs_rl_mod_lib::catalog::Slot::Wheels,
        &source,
        &target,
    );
    println!("rules: {rules:?}");
    let out = alxs_rl_mod_lib::upk::rename_package(&bytes, &rules, &ring, None).unwrap();
    let (n0, i0, e0, p0, s) = dump(&bytes, &ring);
    let (n1, i1, e1, p1, _) = dump(&out, &ring);
    println!("names={} imports={} exports={} name_end={} import_off={} export_off={} depends_off={} region_start={}",
        s.name_count, s.import_count, s.export_count, n0.end(), s.import_offset, s.export_offset, s.depends_offset, s.name_offset);
    for (a, b) in n0.entries.iter().zip(&n1.entries) {
        if a.name != b.name {
            println!("name[{}]: {} -> {}", a.index, a.name, b.name);
        }
    }
    for (a, b) in i0.iter().zip(&i1) {
        if a.object_name != b.object_name
            || a.class_name != b.class_name
            || a.class_package != b.class_package
            || a.outer != b.outer
        {
            println!(
                "import[{}]: {}.{} ({}) outer {} -> {}.{} ({}) outer {}",
                a.index,
                a.class_package,
                a.class_name,
                a.object_name,
                a.outer,
                b.class_package,
                b.class_name,
                b.object_name,
                b.outer
            );
        }
    }
    for (a, b) in e0.iter().zip(&e1) {
        if a.object_name != b.object_name
            || a.class_index != b.class_index
            || a.super_index != b.super_index
            || a.outer != b.outer
            || a.serial_offset != b.serial_offset
            || a.serial_size != b.serial_size
        {
            println!("export[{}]: {} class {} super {} outer {} off {} size {} -> {} class {} super {} outer {} off {} size {}",
                a.index, a.object_name, a.class_index, a.super_index, a.outer, a.serial_offset, a.serial_size,
                b.object_name, b.class_index, b.super_index, b.outer, b.serial_offset, b.serial_size);
        }
    }
    let rs = s.name_offset;
    for (i, (x, y)) in p0.iter().zip(&p1).enumerate() {
        if x != y && i >= n0.end() {
            println!("byte @file {} (rel {}): {:02x} -> {:02x}", rs + i, i, x, y);
        }
    }
    let n_imp = i1.len() as i32;
    let n_exp = e1.len() as i32;
    let ok = |i: i32| i >= -n_imp && i <= n_exp;
    let bad: Vec<_> = e1
        .iter()
        .filter(|e| !ok(e.class_index) || !ok(e.super_index) || !ok(e.outer))
        .map(|e| e.index)
        .collect();
    println!("out-of-range object indices in exports: {bad:?}");
    println!(
        "parsed imports {} / exports {} (after: {} / {})",
        i0.len(),
        e0.len(),
        i1.len(),
        e1.len()
    );
}
