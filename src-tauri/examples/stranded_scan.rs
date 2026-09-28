//! Dev probe: Texture2D exports stored in a texture cache whose mip chain does
//! not parse (they would block a cache hijack).
use alxs_rl_mod_lib::upk::{keys::KeyRing, texture, Package};

fn main() {
    let mut args = std::env::args().skip(1);
    let cooked_arg = args.next().expect("CookedPCConsole dir");
    let cooked = std::path::Path::new(&cooked_arg);
    let prefixes: Vec<String> = args.map(|s| s.to_ascii_lowercase()).collect();
    let keys_txt = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = KeyRing::from_text(&keys_txt, None);
    let (mut files, mut stranded) = (0, 0);
    for entry in std::fs::read_dir(cooked).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if !name.ends_with(".upk") || !prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let Ok(pkg) = Package::open(std::fs::read(entry.path()).unwrap(), &ring) else {
            continue;
        };
        let Ok(body) = pkg.body() else { continue };
        files += 1;
        let parsed = pkg.textures(&body);
        for e in pkg
            .exports
            .iter()
            .filter(|e| pkg.class_of(e) == "Texture2D")
        {
            if parsed.iter().any(|t| t.export_index == e.index) {
                continue;
            }
            let Some(pos) = body.export_pos(e) else {
                continue;
            };
            if let Some(cache) = texture::cache_name_of(&body.data, pos, &pkg.names) {
                stranded += 1;
                println!("{name}: {} ({cache})", e.object_name);
            }
        }
    }
    println!("{files} packages, {stranded} stranded textures");
}
