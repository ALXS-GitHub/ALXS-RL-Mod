//! Dev probe: recolours a package and writes the result next to `out`.
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("package");
    let target = alxs_rl_mod_lib::upk::recolor::Target::from_hex(&args.next().expect("#rrggbb"))
        .expect("hex colour");
    let out_path = args.next().expect("out");
    let keys = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = alxs_rl_mod_lib::upk::KeyRing::from_text(&keys, None);
    let original = std::fs::read(&path).unwrap();
    let pkg = alxs_rl_mod_lib::upk::Package::open(original.clone(), &ring).unwrap();
    let body = pkg.body().unwrap();
    let (patch, stats) = alxs_rl_mod_lib::upk::recolor::recolor_patch(&pkg, &body, target);
    println!("{stats:?}");
    let mut out = pkg.header_bytes().unwrap();
    let blocks = patch.apply(&mut out, &body.map).unwrap();
    pkg.seal(&mut out).unwrap();
    println!(
        "rewritten blocks: {blocks}, same size: {}",
        out.len() == original.len()
    );
    std::fs::write(&out_path, &out).unwrap();
    let again = alxs_rl_mod_lib::upk::Package::open(out, &ring).unwrap();
    println!("reopens: {} exports", again.exports.len());
}
