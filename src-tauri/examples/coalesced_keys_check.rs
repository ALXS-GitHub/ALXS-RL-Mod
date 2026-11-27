//! Dev probe: does the community key list alone open the game's
//! localisation archive (the catalog source)? Usage: <cooked dir>
use alxs_rl_mod_lib::catalog::game_db;
use alxs_rl_mod_lib::upk::{keys, KeyRing};

fn main() {
    let cooked = std::env::args().nth(1).expect("cooked dir");
    let cooked = std::path::Path::new(&cooked);
    let community = tauri::async_runtime::block_on(keys::download_community()).unwrap();
    let local = keys::parse_keys(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/keys/keys.txt"
        ))
        .unwrap_or_default(),
    );
    for (label, ring) in [
        ("community", KeyRing::from_keys(community.clone())),
        ("local", KeyRing::from_keys(local.clone())),
    ] {
        let n = game_db::read(cooked, &ring).map(|p| p.len());
        println!("{label}: {} keys -> products {n:?}", ring.len());
    }
    let missing: Vec<_> = local.iter().filter(|k| !community.contains(k)).collect();
    println!("local keys absent from community: {}", missing.len());
    for k in &missing {
        let ring = KeyRing::from_keys(vec![**k]);
        if game_db::read(cooked, &ring).is_some() {
            use base64::prelude::{Engine as _, BASE64_STANDARD};
            println!(
                "coalesced key (missing from community): {}",
                BASE64_STANDARD.encode(k)
            );
        }
    }
}
