//! Dev probe: header-only open of packages (as the catalog does) vs full open.
use std::io::Read;

use alxs_rl_mod_lib::upk::{crypto, keys::KeyRing, summary::PackageSummary};

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
        let full = std::fs::read(std::path::Path::new(&cooked).join(&name)).unwrap();
        let s = PackageSummary::parse(&full).unwrap();
        println!(
            "{name}: total_header={} name_offset={} name_count={} region_len={} file={}",
            s.total_header_size,
            s.name_offset,
            s.name_count,
            s.encrypted_region_len(),
            full.len()
        );
        println!(
            "  full open: {:?}",
            crypto::open_header(&full, &s, &ring).map(|h| (
                h.key.is_some(),
                h.nonce.is_some(),
                h.chunks.len()
            ))
        );
        match alxs_rl_mod_lib::upk::Package::open(full.clone(), &ring) {
            Ok(pkg) => match pkg.body() {
                Ok(body) => println!(
                    "  body: {} bytes, {} segments, {} textures",
                    body.data.len(),
                    body.segments.len(),
                    pkg.textures(&body).len()
                ),
                Err(e) => println!("  body error: {e}"),
            },
            Err(e) => println!("  package error: {e}"),
        }
        let mut f = std::fs::File::open(std::path::Path::new(&cooked).join(&name)).unwrap();
        let mut head = vec![0u8; 64 * 1024];
        let n = f.read(&mut head).unwrap();
        head.truncate(n);
        println!(
            "  head {n} bytes: {:?}",
            crypto::open_header(&head, &s, &ring).map(|h| h.key.is_some())
        );
        let wanted = s.total_header_size + 4096;
        if wanted > head.len() {
            let mut rest = Vec::with_capacity(wanted - head.len());
            let r = f.take((wanted - head.len()) as u64).read_to_end(&mut rest);
            println!(
                "  extra read {:?} -> {}",
                r.as_ref().map(|n| *n),
                rest.len()
            );
            head.extend(rest);
        }
        println!(
            "  catalog path: can_decrypt={} len={}",
            alxs_rl_mod_lib::upk::can_decrypt(&head, &ring),
            head.len()
        );
    }
}
