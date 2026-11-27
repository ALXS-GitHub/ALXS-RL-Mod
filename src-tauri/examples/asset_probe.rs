//! Dev probe: list imports / exports / properties / textures of packages,
//! or index every object of the game (`index`, used to build map repair
//! mappings, see docs_rl/map_repair.md). `GREP=a,b` filters lines.
//! `cargo run --release --example asset_probe -- <keys.txt> imports|exports|props|tex <pkg>...`
//! `cargo run --release --example asset_probe -- <keys.txt> index <CookedPCConsole> <out.tsv>`

use alxs_rl_mod_lib::upk::{keys::KeyRing, texture, thumbs, Package};

fn path_of(pkg: &Package, mut index: i32) -> String {
    let mut parts = Vec::new();
    let mut guard = 0;
    while index != 0 && guard < 64 {
        guard += 1;
        if index < 0 {
            let Some(i) = pkg.imports.get((-index - 1) as usize) else {
                parts.push("?".into());
                break;
            };
            parts.push(i.object_name.clone());
            index = i.outer;
        } else {
            let Some(e) = pkg.exports.get((index - 1) as usize) else {
                parts.push("?".into());
                break;
            };
            parts.push(e.object_name.clone());
            index = e.outer;
        }
    }
    parts.reverse();
    parts.join(".")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let keys = std::fs::read_to_string(args.next().expect("keys")).unwrap_or_default();
    let mode = args.next().expect("mode");
    let ring = KeyRing::from_text(&keys, None);
    let needles: Vec<String> = std::env::var("GREP")
        .map(|g| g.split(',').map(|s| s.to_lowercase()).collect())
        .unwrap_or_default();
    if mode == "index" {
        // args: <cooked dir> <out tsv>; header-only open of every .upk.
        use std::io::{Read, Write};
        let dir = args.next().expect("dir");
        let out = args.next().expect("out");
        let mut w = std::io::BufWriter::new(std::fs::File::create(out).unwrap());
        let keep = [
            "Material",
            "MaterialInstanceConstant",
            "StaticMesh",
            "Texture2D",
            "Package",
            "SkeletalMesh",
            "ParticleSystem",
            "PhysicalMaterial",
            "TextureCube",
        ];
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("upk")))
            .collect();
        files.sort();
        let (mut ok, mut failed) = (0, 0);
        for path in files {
            let Ok(mut f) = std::fs::File::open(&path) else {
                continue;
            };
            let mut buf = vec![0u8; 64 * 1024];
            let n = f.read(&mut buf).unwrap_or(0);
            buf.truncate(n);
            let summary = match alxs_rl_mod_lib::upk::summary::PackageSummary::parse(&buf) {
                Ok(s) => s,
                // Large summaries (many chunks) do not fit in 64 KB: read it all.
                Err(_) => {
                    let Ok(full) = std::fs::read(&path) else {
                        failed += 1;
                        continue;
                    };
                    match alxs_rl_mod_lib::upk::summary::PackageSummary::parse(&full) {
                        Ok(s) => {
                            buf = full;
                            s
                        }
                        Err(_) => {
                            failed += 1;
                            if failed <= 10 {
                                eprintln!("summary failed: {}", path.display());
                            }
                            continue;
                        }
                    }
                }
            };
            if summary.total_header_size > buf.len() {
                let mut rest = vec![0u8; summary.total_header_size - buf.len()];
                if f.read_exact(&mut rest).is_err() {
                    failed += 1;
                    continue;
                }
                buf.extend(rest);
            }
            let pkg = match Package::open(buf, &ring) {
                Ok(pkg) => pkg,
                // Some packages need the whole file (chunk tables past the header).
                Err(_) => match std::fs::read(&path)
                    .ok()
                    .and_then(|full| Package::open(full, &ring).ok())
                {
                    Some(pkg) => pkg,
                    None => {
                        failed += 1;
                        if failed <= 15 {
                            eprintln!("failed: {}", path.display());
                        }
                        continue;
                    }
                },
            };
            ok += 1;
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            for e in &pkg.exports {
                let c = pkg.class_of(e);
                if keep.contains(&c) {
                    let _ = writeln!(w, "{}	{}	{}", path_of(&pkg, e.index as i32 + 1), c, fname);
                }
            }
        }
        eprintln!("indexed {ok} packages, {failed} failed");
        return;
    }
    for p in args {
        let bytes = std::fs::read(&p).expect("read");
        let pkg = match Package::open(bytes, &ring) {
            Ok(pkg) => pkg,
            Err(e) => {
                println!("{p}: open failed: {e}");
                continue;
            }
        };
        println!(
            "== {p}: {} imports, {} exports",
            pkg.imports.len(),
            pkg.exports.len()
        );
        if mode == "tex" {
            let want: Vec<String> = std::env::var("EXPORT")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.to_lowercase())
                .collect();
            let out = std::path::PathBuf::from(std::env::var("OUT").expect("OUT"));
            let cooked = std::path::Path::new(&p).parent().unwrap();
            let body = pkg.body().expect("body");
            for t in pkg.textures(&body) {
                if !want.contains(&t.name.to_lowercase()) {
                    continue;
                }
                let Some(mip) = t
                    .mips
                    .iter()
                    .filter(|m| !m.is_empty() && m.width <= 256)
                    .max_by_key(|m| m.width)
                else {
                    println!("{}: no mip", t.name);
                    continue;
                };
                match thumbs::read_mip(cooked, &body.data, t.tfc_name.as_deref(), mip)
                    .and_then(|raw| texture::decode(t.format, mip.width, mip.height, &raw))
                {
                    Ok(img) => {
                        println!("{} {}x{}", t.name, mip.width, mip.height);
                        img.save(out.join(format!("{}.png", t.name))).unwrap();
                    }
                    Err(e) => println!("{}: {e}", t.name),
                }
            }
            continue;
        }
        if mode == "props" {
            let target = std::env::var("EXPORT").unwrap_or_default().to_lowercase();
            let body = pkg.body().expect("body");
            for e in pkg
                .exports
                .iter()
                .filter(|e| e.object_name.to_lowercase() == target)
            {
                let Some(pos) = body.export_pos(e) else {
                    continue;
                };
                let data = &body.data[pos..(pos + e.serial_size).min(body.data.len())];
                println!(
                    "-- {} ({}) size {}",
                    path_of(&pkg, e.index as i32 + 1),
                    pkg.class_of(e),
                    e.serial_size
                );
                // Tagged properties after NetIndex (try offsets 4 then 0).
                for start in [4usize, 0, 8] {
                    let mut o = start;
                    let mut out = Vec::new();
                    let rd = |o: usize| i32::from_le_bytes(data[o..o + 4].try_into().unwrap());
                    let mut ok = true;
                    while o + 24 <= data.len() {
                        let name = pkg.names.get(rd(o));
                        if name.is_empty() {
                            ok = false;
                            break;
                        }
                        if name == "None" {
                            break;
                        }
                        let ty = pkg.names.get(rd(o + 8)).to_string();
                        let size = rd(o + 16) as usize;
                        o += 24;
                        let mut val = String::new();
                        match ty.as_str() {
                            "BoolProperty" => {
                                val = format!("{}", data[o]);
                                o += 1;
                            }
                            "StructProperty" => {
                                val = pkg.names.get(rd(o)).to_string();
                                o += 8;
                            }
                            "ByteProperty" => {
                                val = pkg.names.get(rd(o)).to_string();
                                o += 8;
                            }
                            "" => {
                                ok = false;
                                break;
                            }
                            "NameProperty" => {
                                val = pkg.names.get(rd(o)).to_string();
                            }
                            "ObjectProperty" => {
                                val = path_of(&pkg, rd(o));
                            }
                            _ => {}
                        }
                        out.push(format!("{name}:{ty}={val}"));
                        o += size;
                    }
                    if ok && !out.is_empty() {
                        println!("   @{start}: {}", out.join(" | "));
                        break;
                    }
                }
            }
            continue;
        }
        let lines: Vec<String> = if mode == "imports" {
            (0..pkg.imports.len())
                .map(|i| {
                    let imp = &pkg.imports[i];
                    format!("{} [{}]", path_of(&pkg, -(i as i32) - 1), imp.class_name)
                })
                .collect()
        } else {
            (0..pkg.exports.len())
                .map(|i| {
                    let e = &pkg.exports[i];
                    format!("{} [{}]", path_of(&pkg, i as i32 + 1), pkg.class_of(e))
                })
                .collect()
        };
        for l in lines {
            if needles.is_empty() || needles.iter().any(|n| l.to_lowercase().contains(n)) {
                println!("{l}");
            }
        }
    }
}
