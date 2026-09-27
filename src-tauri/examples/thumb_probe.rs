//! Dev probe: prints what the thumbnail extractor sees in real packages.
//! `cargo run --example thumb_probe -- <CookedPCConsole> <package.upk>...`

use std::path::Path;

use alxs_rl_mod_lib::upk::{keys::KeyRing, texture, thumbs, Package};

fn main() {
    let mut args = std::env::args().skip(1);
    let cooked = args.next().expect("cooked dir");
    let keys_txt = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = KeyRing::from_text(&keys_txt, None);
    for name in args {
        println!("== {name}");
        let bytes = std::fs::read(Path::new(&cooked).join(&name)).expect("read");
        let pkg = match Package::open(bytes, &ring) {
            Ok(p) => p,
            Err(e) => {
                println!("   open failed: {e}");
                continue;
            }
        };
        let body = match pkg.body() {
            Ok(b) => b,
            Err(e) => {
                println!("   body failed: {e}");
                continue;
            }
        };
        println!(
            "   origin={} body_len={} preamble={} chunks={:?}",
            body.origin,
            body.data.len(),
            body.preamble,
            pkg.summary.compression_flags
        );
        for e in &pkg.exports {
            println!(
                "   ex {} off={} size={}",
                e.object_name, e.serial_offset, e.serial_size
            );
            let class = pkg.class_of(e);
            if class == "Texture2D" {
                let res = body.export_pos(e).map(|pos| {
                    texture::parse_texture(
                        &body.data,
                        pos,
                        e.serial_size,
                        e.index,
                        &e.object_name,
                        &pkg.names,
                    )
                    .map(|t| t.mips.len())
                });
                println!(
                    "   export {} [{}] size={} pos={:?} -> {:?}",
                    e.object_name,
                    class,
                    e.serial_size,
                    body.export_pos(e),
                    res.map(|r| r.map_err(|x| x.to_string()))
                );
            }
        }
        let textures = pkg.textures(&body);
        println!("   {} textures", textures.len());
        for t in &textures {
            println!(
                "   - {} {}x{} {:?} ({}) tfc={:?} mips={}",
                t.name,
                t.width,
                t.height,
                t.format,
                t.format_name,
                t.tfc_name,
                t.mips.len()
            );
            for m in t.mips.iter().take(12) {
                let data =
                    thumbs::read_mip(Path::new(&cooked), &body.data, t.tfc_name.as_deref(), m);
                let decoded = data.as_ref().map_err(|e| e.to_string()).and_then(|d| {
                    texture::decode(t.format, m.width, m.height, d)
                        .map(|i| i.width())
                        .map_err(|e| e.to_string())
                });
                println!(
                    "       mip {}x{} flags={:#x} size={} inline={} -> {:?}",
                    m.width,
                    m.height,
                    m.flags,
                    m.size_on_disk,
                    m.data_pos.is_some(),
                    decoded
                );
            }
        }
    }
}
