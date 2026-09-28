//! Dev probe: colour-related properties of a package (particle colour
//! modules, material instances).
use alxs_rl_mod_lib::upk::names::NameTable;
use alxs_rl_mod_lib::upk::props;
use alxs_rl_mod_lib::upk::reader::{read_i32, read_u32};

fn f32_at(buf: &[u8], pos: usize) -> f32 {
    f32::from_bits(read_u32(buf, pos).unwrap_or(0))
}

fn dump(buf: &[u8], start: usize, names: &NameTable, depth: usize) {
    let Ok((list, _)) = props::walk(buf, start, names) else {
        println!("{}(unreadable)", "  ".repeat(depth));
        return;
    };
    for p in list {
        let pad = "  ".repeat(depth);
        let inner = p.inner.clone().unwrap_or_default();
        print!(
            "{pad}{} : {} {} [{}] @{}",
            p.name, p.type_name, inner, p.size, p.value_pos
        );
        match p.type_name.as_str() {
            "FloatProperty" => println!(" = {}", f32_at(buf, p.value_pos)),
            "IntProperty" => println!(" = {:?}", p.as_i32(buf)),
            "ByteProperty" | "NameProperty" => {
                println!(" = {:?}", p.as_name(buf, names).or(Some("?")))
            }
            "ObjectProperty" => println!(" = {:?}", p.as_i32(buf)),
            "StructProperty" if inner == "LinearColor" || inner == "Vector" || inner == "Color" => {
                let v: Vec<f32> = (0..(p.size / 4))
                    .map(|i| f32_at(buf, p.value_pos + i * 4))
                    .collect();
                println!(" = {v:?}");
            }
            "StructProperty" => {
                println!();
                dump(buf, p.value_pos, names, depth + 1);
            }
            "ArrayProperty" => {
                let n = read_i32(buf, p.value_pos).unwrap_or(0);
                if p.name == "LookupTable" {
                    let v: Vec<f32> = (0..n as usize)
                        .map(|i| f32_at(buf, p.value_pos + 4 + i * 4))
                        .collect();
                    println!(" n={n} = {v:?}");
                } else {
                    println!(" n={n}");
                    let mut at = p.value_pos + 4;
                    for _ in 0..n.min(8) {
                        match props::walk(buf, at, names) {
                            Ok((_, next)) => {
                                dump(buf, at, names, depth + 1);
                                at = next;
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
            _ => println!(),
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("package");
    let keys = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = alxs_rl_mod_lib::upk::KeyRing::from_text(&keys, None);
    let pkg = alxs_rl_mod_lib::upk::Package::open(std::fs::read(&path).unwrap(), &ring).unwrap();
    let body = pkg.body().unwrap();
    for e in &pkg.exports {
        let class = pkg.class_of(e);
        if !(class.contains("Color")
            || class.contains("Material")
            || class.contains("Distribution")
            || class == "ParameterDispenser_X"
            || [10usize, 12].contains(&e.index))
        {
            continue;
        }
        let Some(pos) = body.export_pos(e) else {
            continue;
        };
        println!(
            "== [{}] {} ({class}) size={} pos={pos}",
            e.index, e.object_name, e.serial_size
        );
        if class.contains("Distribution")
            || class == "ParameterDispenser_X"
            || class.contains("ProductAsset")
        {
            for skip in [0usize, 4, 8, 12, 16] {
                if props::walk(&body.data, pos + skip, &pkg.names).is_ok() {
                    println!("  (props at +{skip})");
                    dump(&body.data, pos + skip, &pkg.names, 1);
                    break;
                }
            }
            continue;
        }
        dump(&body.data, pos + 4, &pkg.names, 1);
    }
}
