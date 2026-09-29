//! Dev probe: the "exact colour" pipeline on a stock package, in memory
//! (nothing is written to the game): neutral colour textures, exact
//! recolour, then checks and PNG previews in `out`.
use std::path::Path;

use alxs_rl_mod_lib::swap::texture_color;
use alxs_rl_mod_lib::upk::recolor::{recolor_patch, ColorMode, Target};
use alxs_rl_mod_lib::upk::{chunks, keys::KeyRing, texture, Package};

fn main() {
    let mut args = std::env::args().skip(1);
    let cooked_arg = args.next().expect("CookedPCConsole dir");
    let cooked = Path::new(&cooked_arg);
    let package = args.next().expect("package file name");
    let target = Target::from_hex(&args.next().expect("#rrggbb")).expect("hex");
    let out_arg = args.next().expect("out dir");
    let out = Path::new(&out_arg);
    let keys = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = KeyRing::from_text(&keys, None);
    let stock = std::fs::read(cooked.join(&package)).unwrap();

    for mode in [ColorMode::Exact, ColorMode::Tint] {
        if let Some(img) = texture_color::preview(&stock, &ring, cooked, target, mode).unwrap() {
            img.save(out.join(format!("preview_{mode:?}.png"))).unwrap();
        }
    }

    let started = std::time::Instant::now();
    let n = texture_color::neutralize(&stock, &ring, cooked, &package, target).unwrap();
    println!(
        "neutralized {:?} in {} ms",
        n.textures,
        started.elapsed().as_millis()
    );
    let pkg = Package::open(n.package, &ring).unwrap();
    let body = pkg.body().unwrap();
    let (patch, stats) = recolor_patch(&pkg, &body, target, ColorMode::Exact);
    println!("recolour: {stats:?}");
    let mut bytes = pkg.header_bytes().unwrap();
    patch.apply(&mut bytes, &body.map).unwrap();
    pkg.seal(&mut bytes).unwrap();
    println!("same size: {}", bytes.len() == stock.len());

    let pkg = Package::open(bytes, &ring).unwrap();
    let body = pkg.body().unwrap();
    let Some((name, tfc)) = n.cache else {
        println!("no texture cache");
        return;
    };
    println!("cache {name}.tfc = {} bytes", tfc.len());
    let mut bad = 0;
    for t in pkg
        .textures(&body)
        .iter()
        .filter(|t| t.tfc_name.as_deref() == Some(name.as_str()))
    {
        for m in t.mips.iter().filter(|m| m.in_tfc() && !m.is_empty()) {
            if m.offset_in_file + u64::from(m.size_on_disk) > tfc.len() as u64 {
                bad += 1;
            }
        }
        if n.textures.contains(&t.name) {
            let m = t.mips.iter().find(|m| m.width == 512).unwrap();
            let raw = &tfc[m.offset_in_file as usize
                ..(m.offset_in_file + u64::from(m.size_on_disk)) as usize];
            let raw = if m.compressed() {
                chunks::unwrap_chunked(raw).unwrap()
            } else {
                raw.to_vec()
            };
            texture::decode(t.format, 512, 512, &raw)
                .unwrap()
                .save(out.join(format!("{}_neutral.png", t.name)))
                .unwrap();
        }
    }
    println!("out-of-bounds mips: {bad}");
}
