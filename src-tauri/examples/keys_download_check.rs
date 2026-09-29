//! Dev probe: downloads the community key list with the app's validation and
//! compares it with the local keys.txt.
use alxs_rl_mod_lib::upk::keys;

fn main() {
    let incoming = tauri::async_runtime::block_on(keys::download_community()).unwrap();
    let local = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let local = keys::parse_keys(&local);
    let (merged, added) = keys::merge(&local, &incoming);
    println!(
        "downloaded {} keys · local {} · new {added} · merged {}",
        incoming.len(),
        local.len(),
        merged.len()
    );
}
