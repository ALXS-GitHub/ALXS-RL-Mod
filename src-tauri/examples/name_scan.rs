//! Dev probe: which cooked packages mention given names in their name table.
//! `cargo run --example name_scan -- <CookedPCConsole> <needle>...`

use std::io::Read;

use alxs_rl_mod_lib::upk::{keys::KeyRing, summary::PackageSummary, Package};

fn main() {
    let mut args = std::env::args().skip(1);
    let cooked = args.next().expect("cooked dir");
    let needles: Vec<String> = args.collect();
    let keys_txt = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = KeyRing::from_text(&keys_txt, None);
    let mut entries: Vec<_> = std::fs::read_dir(&cooked)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    if let Ok(filter) = std::env::var("CLASSES") {
        let wanted: Vec<String> = filter.split(',').map(|f| f.to_lowercase()).collect();
        entries
            .retain(|p| wanted.contains(&p.file_name().unwrap().to_string_lossy().to_lowercase()));
    }
    for path in entries {
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("upk"))
            != Some(true)
        {
            continue;
        }
        let Ok(mut f) = std::fs::File::open(&path) else {
            continue;
        };
        let mut buf = vec![0u8; 64 * 1024];
        let n = f.read(&mut buf).unwrap_or(0);
        buf.truncate(n);
        let Ok(summary) = PackageSummary::parse(&buf) else {
            continue;
        };
        if summary.total_header_size > buf.len() {
            let mut rest = vec![0u8; summary.total_header_size - buf.len()];
            if f.read_exact(&mut rest).is_err() {
                continue;
            }
            buf.extend(rest);
        }
        let Ok(pkg) = Package::open(buf, &ring) else {
            continue;
        };
        if let Ok(filter) = std::env::var("CLASSES") {
            let fname = path.file_name().unwrap().to_string_lossy().to_lowercase();
            if !filter.split(',').any(|f| fname == f.to_lowercase()) {
                continue;
            }
            let classes: Vec<String> = pkg
                .exports
                .iter()
                .map(|e| format!("{}:{}", e.object_name, pkg.class_of(e)))
                .collect();
            let imports: Vec<String> = pkg
                .imports
                .iter()
                .filter(|i| i.class_name == "Product_TA")
                .map(|i| i.object_name.clone())
                .collect();
            println!(
                "{fname}
  exports {:?}
  product imports {:?}",
                classes, imports
            );
            continue;
        }
        if std::env::var("DEFINERS").is_ok() {
            let n = pkg
                .exports
                .iter()
                .filter(|e| pkg.class_of(e) == "Product_TA")
                .count();
            if n > 0 {
                println!(
                    "DEFINES {} Product_TA x{} ({} bytes)",
                    path.file_name().unwrap().to_string_lossy(),
                    n,
                    std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                );
            }
            continue;
        }
        let hits: Vec<&String> = needles
            .iter()
            .filter(|nd| {
                pkg.names
                    .entries
                    .iter()
                    .any(|e| e.name.eq_ignore_ascii_case(nd))
            })
            .collect();
        if !hits.is_empty() {
            println!(
                "{} exports={} names={} hits={:?}",
                path.file_name().unwrap().to_string_lossy(),
                pkg.exports.len(),
                pkg.names.entries.len(),
                hits
            );
        }
    }
}
