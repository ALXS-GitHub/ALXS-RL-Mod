//! Dev probe: builds the custom-ball swap in memory and checks that every
//! texture pointing at our cache stays inside it.
use std::path::Path;

use alxs_rl_mod_lib::decals::pipeline::{self, Select, SwapJob, SwapSpec};
use alxs_rl_mod_lib::upk::{keys::KeyRing, Package};

fn main() {
    let cooked_arg = std::env::args().nth(1).expect("CookedPCConsole dir");
    let cooked = Path::new(&cooked_arg);
    let keys_txt = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = KeyRing::from_text(&keys_txt, None);
    let donor = std::fs::read(cooked.join("GameInfo_Soccar_SF.upk")).unwrap();
    let image = image::RgbaImage::from_pixel(512, 512, image::Rgba([220, 30, 30, 255]));
    let read = |name: &str, offset: u64, len: usize| {
        alxs_rl_mod_lib::game::tfc::read_range(cooked, name, offset, len)
    };
    let swap = pipeline::build_swap(
        donor,
        &ring,
        &SwapSpec {
            renames: &[],
            target_key: None,
            cache_prefix: "AlxsBall",
            jobs: vec![SwapJob {
                select: Select::Name("Ball_Default00_D".into()),
                image: &image,
            }],
        },
        &read,
    )
    .unwrap();
    println!(
        "tfc {} = {} bytes, relocated {:?}",
        swap.tfc_name,
        swap.tfc.len(),
        swap.relocated
    );
    let pkg = Package::open(swap.package, &ring).unwrap();
    let body = pkg.body().unwrap();
    let mut bad = 0;
    for t in pkg.textures(&body) {
        if t.tfc_name.as_deref() != Some(swap.tfc_name.as_str()) {
            continue;
        }
        for m in t.mips.iter().filter(|m| m.in_tfc() && !m.is_empty()) {
            let end = m.offset_in_file + m.size_on_disk as u64;
            if end > swap.tfc.len() as u64 {
                bad += 1;
                println!(
                    "OUT OF BOUNDS {} {}x{} @{} +{}",
                    t.name, m.width, m.height, m.offset_in_file, m.size_on_disk
                );
            }
        }
    }
    println!("out-of-bounds mips: {bad}");
}
