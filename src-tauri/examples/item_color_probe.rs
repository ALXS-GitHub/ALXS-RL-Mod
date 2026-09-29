//! Dev probe: what carries the colour of an item (textures, material
//! parameters, parents) — to judge what a recolour can reach.
use alxs_rl_mod_lib::upk::{keys::KeyRing, material, props, Package};

fn main() {
    let path = std::env::args().nth(1).expect("package");
    let keys = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/keys/keys.txt"
    ))
    .unwrap_or_default();
    let ring = KeyRing::from_text(&keys, None);
    let pkg = Package::open(std::fs::read(&path).unwrap(), &ring).unwrap();
    let body = pkg.body().unwrap();
    println!("== textures");
    for t in pkg.textures(&body) {
        println!(
            "  {} {}x{} {} tfc={:?}",
            t.name, t.width, t.height, t.format_name, t.tfc_name
        );
    }
    println!("== texture params");
    for p in material::texture_params(&pkg, &body) {
        println!("  {} . {} = {}", p.material, p.parameter, p.object_name);
    }
    println!("== MIC parents / vector + scalar params");
    for e in pkg
        .exports
        .iter()
        .filter(|e| pkg.class_of(e) == "MaterialInstanceConstant")
    {
        let Some(pos) = body.export_pos(e) else {
            continue;
        };
        let Ok((tags, _)) = props::walk(&body.data, pos + 4, &pkg.names) else {
            continue;
        };
        let parent = props::find(&tags, "Parent")
            .and_then(|p| p.as_i32(&body.data))
            .map(|i| {
                alxs_rl_mod_lib::upk::tables::object_name(i, &pkg.imports, &pkg.exports).to_string()
            });
        println!("  MIC {} parent={:?}", e.object_name, parent);
        for list in ["VectorParameterValues", "ScalarParameterValues"] {
            let Some(arr) = props::find(&tags, list) else {
                continue;
            };
            let n = alxs_rl_mod_lib::upk::reader::read_i32(&body.data, arr.value_pos).unwrap_or(0);
            let mut at = arr.value_pos + 4;
            for _ in 0..n {
                let Ok((param, next)) = props::walk(&body.data, at, &pkg.names) else {
                    break;
                };
                at = next;
                let name = props::find(&param, "ParameterName")
                    .and_then(|p| p.as_full_name(&body.data, &pkg.names))
                    .unwrap_or_default();
                let v = props::find(&param, "ParameterValue").unwrap();
                let vals: Vec<f32> = (0..(v.size / 4).min(4))
                    .map(|i| {
                        f32::from_le_bytes(
                            body.data[v.value_pos + 4 * i..v.value_pos + 4 * i + 4]
                                .try_into()
                                .unwrap(),
                        )
                    })
                    .collect();
                println!("     {list:>22} {name} = {vals:?}");
            }
        }
    }
}
