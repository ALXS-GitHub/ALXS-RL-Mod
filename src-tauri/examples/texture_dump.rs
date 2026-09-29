//! Dev probe: writes the 512 px (or nearest) mip of every texture of a package as PNG.
use alxs_rl_mod_lib::upk::{keys::KeyRing, texture, thumbs, Package};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = std::path::PathBuf::from(args.next().expect("package"));
    let out = std::path::PathBuf::from(args.next().expect("out dir"));
    let keys = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = KeyRing::from_text(&keys, None);
    let cooked = path.parent().unwrap();
    let pkg = Package::open(std::fs::read(&path).unwrap(), &ring).unwrap();
    let body = pkg.body().unwrap();
    for t in pkg.textures(&body) {
        let Some(mip) = t
            .mips
            .iter()
            .filter(|m| !m.is_empty() && m.width <= 512)
            .max_by_key(|m| m.width)
        else {
            continue;
        };
        match thumbs::read_mip(cooked, &body.data, t.tfc_name.as_deref(), mip)
            .and_then(|raw| texture::decode(t.format, mip.width, mip.height, &raw))
        {
            Ok(img) => {
                let (mut sum, n) = ([0f64; 3], (img.width() * img.height()) as f64);
                for p in img.pixels() {
                    for c in 0..3 {
                        sum[c] += p[c] as f64;
                    }
                }
                println!(
                    "{} {}x{} avg rgb {:?}",
                    t.name,
                    mip.width,
                    mip.height,
                    sum.map(|s| (s / n).round())
                );
                img.save(out.join(format!("{}.png", t.name))).unwrap();
            }
            Err(e) => println!("{}: {e}", t.name),
        }
    }
}
