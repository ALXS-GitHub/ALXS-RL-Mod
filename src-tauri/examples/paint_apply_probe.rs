//! Dev probe: reads the paint database and plans a paint on items.
//! `cargo run --release --example paint_apply_probe -- <CookedPCConsole> <paint id> <item.upk>...`
use alxs_rl_mod_lib::swap::paint;
use alxs_rl_mod_lib::upk::{KeyRing, Package};

fn main() {
    let mut args = std::env::args().skip(1);
    let cooked = std::path::PathBuf::from(args.next().expect("cooked dir"));
    let id: u8 = args.next().expect("paint id").parse().unwrap();
    let keys = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = KeyRing::from_text(&keys, None);
    let tagame = Package::open(std::fs::read(cooked.join("TAGame.upk")).unwrap(), &ring).unwrap();
    let db = paint::read_database(&tagame, &tagame.body().unwrap());
    for p in db.iter().filter(|p| p.id > 0) {
        println!(
            "  paint {:2} {:<14} {:<18} {} primary={:?}",
            p.id,
            p.object,
            p.label,
            p.info().hex,
            p.colors[0]
        );
    }
    let chosen = db.iter().find(|p| p.id == id).expect("paint");
    for item in args {
        let pkg = Package::open(std::fs::read(cooked.join(&item)).unwrap(), &ring).unwrap();
        let body = pkg.body().unwrap();
        let all = paint::settings(&pkg, &body);
        if all.is_empty() {
            println!("{item}: not paintable");
            continue;
        }
        let (_, count) = paint::apply(&pkg, &body, &all, chosen, &db);
        println!("{item}: {} settings -> {count} values", all.len());
        for s in &all {
            let plan = paint::plan(s, chosen, &db);
            println!(
                "    param={} variant={} particles={} accepts={} materials={:?} additional={:?} overrides={}\n      -> materials={:?} particles={:?}",
                s.parameter,
                s.variant,
                s.particles,
                s.accepts(chosen),
                s.materials,
                s.additional.iter().map(|a| &a.parameter).collect::<Vec<_>>(),
                s.overrides.len(),
                plan.materials,
                plan.particles
            );
        }
    }
}
