//! Dev tool: writes a hybrid test decal pack (Octane) that shows at a glance
//! which parts of the body take the primary colour, the accent colour or
//! the art's own colours.
//! `cargo run --example decal_test_pack -- <library root>`
//!
//! - `1_Diffuse_Skin`: a rainbow (hue follows X) with a darker 128 px checker.
//! - `2_Diffuse_Skin_Mask`: repeating horizontal bands of 160 px:
//!   red = primary colour, green = accent colour, black = rainbow as is.

use image::{Rgba, RgbaImage};

const SIZE: u32 = 2048;
const BAND: u32 = 160;

fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
    let (r, g, b) = match i as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("library root"));
    let dir = root.join("ALXS Hybrid Test").join("Octane");
    std::fs::create_dir_all(&dir).unwrap();

    let art = RgbaImage::from_fn(SIZE, SIZE, |x, y| {
        let dark = ((x / 128) + (y / 128)) % 2 == 1;
        let [r, g, b] = hsv(x as f32 / SIZE as f32, 1.0, if dark { 0.6 } else { 1.0 });
        // Alpha is the material's highlight mask: none, like the donor.
        Rgba([r, g, b, 0])
    });
    let zones = RgbaImage::from_fn(SIZE, SIZE, |_, y| match (y / BAND) % 3 {
        0 => Rgba([255, 0, 0, 255]),
        1 => Rgba([0, 255, 0, 255]),
        _ => Rgba([0, 0, 0, 255]),
    });
    art.save(dir.join("art.png")).unwrap();
    zones.save(dir.join("zones.png")).unwrap();
    let template = serde_json::json!({
        "ALXS Hybrid Test (Octane)": {
            "Group": "ALXS",
            "BodyID": 23,
            "SkinID": 0,
            "Body": { "1_Diffuse_Skin": "art.png", "2_Diffuse_Skin_Mask": "zones.png" },
            "Chassis": {}
        }
    });
    std::fs::write(
        dir.join("Template.json"),
        serde_json::to_string_pretty(&template).unwrap(),
    )
    .unwrap();
    println!("{}", dir.display());
}
