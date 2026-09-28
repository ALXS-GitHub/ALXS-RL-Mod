//! Official paints applied to the shown item of a swap.
//!
//! The game paints an item at runtime from the owned item's paint: its
//! `ProductAttribute_PaintSettings_TA` names the parameter(s) to set
//! (`CustomColor` by default, `PaintColor` on some items), which colour
//! variant of the paint to use (`EPaintColorVariant`: Primary, LightAccent,
//! DarkAccent, Emissive…) and a multiplier; some items add per-paint
//! particle overrides (`ProductOverride_ParticleSystemColorParameter_TA`).
//! Paints live in `TAGame.upk` (`PaintDB`, class `PaintDatabase_TA`: index =
//! the game's PaintID; each `ProductPaint_TA` has `Label` and `Colors[12]`).
//!
//! We bake the chosen paint into the parameters' default values, which the
//! game uses when the owned item is not painted. A painted owned item still
//! gets its own paint from the game.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;

use crate::base::error::{AppError, AppResult};
use crate::game::install::RlInstall;
use crate::upk::chunks::StreamPatch;
use crate::upk::keys::KeyRing;
use crate::upk::package::{Body, Package};
use crate::upk::props::{self, Prop};
use crate::upk::reader::read_i32;

/// Highest PaintID accepted from outside (share codes): the game has 29,
/// with room for new seasons.
pub const MAX_PAINT_ID: u8 = 63;

/// `EPaintColorVariant`, in enum order.
const VARIANTS: [&str; 12] = [
    "Primary",
    "LightAccent",
    "DarkAccent",
    "Emissive",
    "DeEmissive",
    "Complementary",
    "Balanced",
    "Tertiary",
    "Additive",
    "Unused3",
    "Unused4",
    "Unused5",
];

fn variant_index(enum_value: &str) -> usize {
    let short = enum_value.rsplit('_').next().unwrap_or(enum_value);
    VARIANTS.iter().position(|v| *v == short).unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paint {
    /// The game's PaintID.
    pub id: u8,
    /// Object name in `ProductPaint.Paints` (`Red_00`…): how items refer to it.
    pub object: String,
    pub label: String,
    /// `Colors[EPaintColorVariant]`, linear RGBA.
    pub colors: [[f32; 4]; 12],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaintInfo {
    pub id: u8,
    pub label: String,
    /// Display colour (sRGB of the primary variant).
    pub hex: String,
}

impl Paint {
    pub fn info(&self) -> PaintInfo {
        let to_srgb = |c: f32| (c.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0).round() as u8;
        let [r, g, b, _] = self.colors[0];
        PaintInfo {
            id: self.id,
            label: self.label.clone(),
            hex: format!("#{:02x}{:02x}{:02x}", to_srgb(r), to_srgb(g), to_srgb(b)),
        }
    }
}

/// Where an export's property block starts (4 for most objects, 8 for some).
fn property_block(data: &[u8], pos: usize, pkg: &Package) -> Option<Vec<Prop>> {
    [4usize, 8].into_iter().find_map(|skip| {
        props::walk(data, pos + skip, &pkg.names)
            .ok()
            .map(|(p, _)| p)
    })
}

fn object_array(data: &[u8], prop: &Prop) -> Vec<i32> {
    let n = read_i32(data, prop.value_pos).unwrap_or(0).max(0) as usize;
    (0..n)
        .filter_map(|k| read_i32(data, prop.value_pos + 4 + k * 4))
        .collect()
}

/// Elements of an array of structs (each a property block).
fn struct_array(data: &[u8], prop: &Prop, pkg: &Package) -> Vec<Vec<Prop>> {
    let n = read_i32(data, prop.value_pos).unwrap_or(0).max(0);
    let mut at = prop.value_pos + 4;
    let mut out = Vec::new();
    for _ in 0..n {
        let Ok((fields, next)) = props::walk(data, at, &pkg.names) else {
            break;
        };
        out.push(fields);
        at = next;
    }
    out
}

fn linear_color(data: &[u8], prop: &Prop) -> Option<[f32; 4]> {
    let f =
        |k: usize| crate::upk::reader::read_u32(data, prop.value_pos + k * 4).map(f32::from_bits);
    Some([f(0)?, f(1)?, f(2)?, f(3)?])
}

fn export_by_object(pkg: &Package, object: i32) -> Option<&crate::upk::tables::Export> {
    usize::try_from(object - 1)
        .ok()
        .and_then(|i| pkg.exports.get(i))
}

/// The paint database of a `TAGame.upk`.
pub fn read_database(pkg: &Package, body: &Body) -> Vec<Paint> {
    let data = &body.data;
    let Some(db) = pkg
        .exports
        .iter()
        .find(|e| e.object_name == "PaintDB" && pkg.class_of(e) == "PaintDatabase_TA")
    else {
        return Vec::new();
    };
    let Some(fields) = body
        .export_pos(db)
        .and_then(|p| property_block(data, p, pkg))
    else {
        return Vec::new();
    };
    let Some(list) = props::find(&fields, "Paints") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, object) in object_array(data, list).into_iter().enumerate() {
        let (Ok(id), Some(export)) = (u8::try_from(id), export_by_object(pkg, object)) else {
            continue;
        };
        let Some(paint_fields) = body
            .export_pos(export)
            .and_then(|p| property_block(data, p, pkg))
        else {
            continue;
        };
        let mut colors = [[0.0, 0.0, 0.0, 1.0]; 12];
        for p in paint_fields.iter().filter(|p| p.name == "Colors") {
            if let (Some(slot), Some(c)) = (
                usize::try_from(p.array_index).ok().filter(|&i| i < 12),
                linear_color(data, p),
            ) {
                colors[slot] = c;
            }
        }
        let label = props::find(&paint_fields, "Label")
            .and_then(|p| p.as_str(data))
            .unwrap_or_else(|| export.object_name.clone());
        out.push(Paint {
            id,
            object: export.object_name.clone(),
            label,
            colors,
        });
    }
    out
}

type DbCache = Mutex<Option<(String, Arc<Vec<Paint>>)>>;

fn cache() -> &'static DbCache {
    static C: OnceLock<DbCache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

/// The installed game's paints (cached per game build).
pub fn database(install: &RlInstall, build: &str, ring: &KeyRing) -> AppResult<Arc<Vec<Paint>>> {
    if let Ok(guard) = cache().lock() {
        if let Some((_, db)) = guard.as_ref().filter(|(b, _)| b == build) {
            return Ok(Arc::clone(db));
        }
    }
    let path = install.cooked_dir.join("TAGame.upk");
    let bytes = crate::game::writer::stock_bytes(install, &path)?;
    let pkg = Package::open(bytes, ring)?;
    let body = pkg.body()?;
    let db = Arc::new(read_database(&pkg, &body));
    if db.is_empty() {
        return Err(AppError::Unsupported(
            "paint database not found in TAGame.upk".into(),
        ));
    }
    if let Ok(mut guard) = cache().lock() {
        *guard = Some((build.to_string(), Arc::clone(&db)));
    }
    Ok(db)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Additional {
    pub parameter: String,
    pub variant: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Override {
    pub parameter: String,
    pub variant: usize,
    /// A fixed paint's colour, else `custom`.
    pub paint: Option<String>,
    pub custom: [f32; 4],
}

/// An item's `ProductAttribute_PaintSettings_TA`.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintSettings {
    pub parameter: String,
    pub variant: usize,
    pub multiplier: f32,
    pub particles: bool,
    /// PaintIDs the item accepts.
    pub include: Vec<u8>,
    /// Paint object names it refuses (metals…).
    pub unsupported: Vec<String>,
    pub additional: Vec<Additional>,
    /// Per paint object name.
    pub overrides: HashMap<String, Vec<Override>>,
    /// Material instances it paints (object refs, `MaterialGroups`); empty =
    /// every material of the package.
    pub materials: Vec<i32>,
}

impl PaintSettings {
    pub fn accepts(&self, paint: &Paint) -> bool {
        paint.id > 0
            && (self.include.is_empty() || self.include.contains(&paint.id))
            && !self.unsupported.contains(&paint.object)
    }
}

fn object_name(pkg: &Package, object: i32) -> String {
    crate::upk::tables::object_name(object, &pkg.imports, &pkg.exports).to_string()
}

/// Every paint settings of the package (a product may paint several parts,
/// each with its own settings); empty when the item is not paintable.
pub fn settings(pkg: &Package, body: &Body) -> Vec<PaintSettings> {
    pkg.exports
        .iter()
        .filter(|e| {
            e.serial_size > 0
                && !e.object_name.starts_with("Default__")
                && pkg.class_of(e) == "ProductAttribute_PaintSettings_TA"
        })
        .filter_map(|e| read_settings(pkg, body, e))
        .collect()
}

fn read_settings(
    pkg: &Package,
    body: &Body,
    export: &crate::upk::tables::Export,
) -> Option<PaintSettings> {
    let data = &body.data;
    let fields = property_block(data, body.export_pos(export)?, pkg)?;
    let name =
        |f: &[Prop], key: &str| props::find(f, key).and_then(|p| p.as_full_name(data, &pkg.names));
    let variant = |f: &[Prop], key: &str| {
        props::find(f, key)
            .and_then(|p| p.as_name(data, &pkg.names))
            .map(variant_index)
            .unwrap_or(0)
    };
    let mut s = PaintSettings {
        parameter: name(&fields, "PaintParameterName").unwrap_or_else(|| "CustomColor".into()),
        variant: variant(&fields, "PaintType"),
        multiplier: props::find(&fields, "PaintEmissiveMultiplier")
            .and_then(|p| p.as_f32(data))
            .unwrap_or(1.0),
        particles: props::find(&fields, "bPaintParticles")
            .and_then(|p| p.as_bool(data))
            .unwrap_or(false),
        include: props::find(&fields, "IncludePaintIDs")
            .map(|p| {
                object_array(data, p)
                    .into_iter()
                    .filter_map(|i| u8::try_from(i).ok())
                    .collect()
            })
            .unwrap_or_default(),
        unsupported: props::find(&fields, "UnsupportedPaints")
            .map(|p| {
                object_array(data, p)
                    .into_iter()
                    .map(|o| object_name(pkg, o))
                    .collect()
            })
            .unwrap_or_default(),
        additional: Vec::new(),
        overrides: HashMap::new(),
        materials: fields
            .iter()
            .filter(|p| p.name == "MaterialGroups")
            .filter_map(|p| props::walk(data, p.value_pos, &pkg.names).ok())
            .filter_map(|(group, _)| {
                props::find(&group, "Materials").map(|m| object_array(data, m))
            })
            .flatten()
            .collect(),
    };
    if let Some(list) = props::find(&fields, "PaintAdditionalParameters") {
        for f in struct_array(data, list, pkg) {
            let enabled = props::find(&f, "bEnabled")
                .and_then(|p| p.as_bool(data))
                .unwrap_or(false);
            if let (true, Some(parameter)) = (enabled, name(&f, "ParameterName")) {
                s.additional.push(Additional {
                    parameter,
                    variant: variant(&f, "PaintVariant"),
                });
            }
        }
    }
    if let Some(list) = props::find(&fields, "PaintsToOverride") {
        for f in struct_array(data, list, pkg) {
            let Some(paint) = props::find(&f, "PaintToOverride")
                .and_then(|p| p.as_i32(data))
                .map(|o| object_name(pkg, o))
            else {
                continue;
            };
            let objects = props::find(&f, "Overrides")
                .map(|p| object_array(data, p))
                .unwrap_or_default();
            let entries = s.overrides.entry(paint).or_default();
            for object in objects {
                let Some(fields) = export_by_object(pkg, object)
                    .and_then(|e| body.export_pos(e))
                    .and_then(|p| property_block(data, p, pkg))
                else {
                    continue;
                };
                let Some(list) = props::find(&fields, "ParameterOverrides") else {
                    continue;
                };
                for o in struct_array(data, list, pkg) {
                    let Some(parameter) = name(&o, "PaintParameterName") else {
                        continue;
                    };
                    entries.push(Override {
                        parameter,
                        variant: variant(&o, "PaintType"),
                        paint: props::find(&o, "Paint")
                            .and_then(|p| p.as_i32(data))
                            .filter(|&o| o != 0)
                            .map(|o| object_name(pkg, o)),
                        custom: props::find(&o, "CustomColor")
                            .and_then(|p| linear_color(data, p))
                            .unwrap_or([1.0, 1.0, 1.0, 1.0]),
                    });
                }
            }
        }
    }
    Some(s)
}

/// What to write: material parameters (MIC values and parameter
/// expressions) and particle parameters, by full parameter name.
#[derive(Debug, Default, PartialEq)]
pub struct PaintPlan {
    pub materials: HashMap<String, [f32; 3]>,
    pub particles: HashMap<String, [f32; 3]>,
}

fn rgb(c: [f32; 4], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

/// The parameter values the game would set for `paint` on this item.
pub fn plan(settings: &PaintSettings, paint: &Paint, db: &[Paint]) -> PaintPlan {
    let mut plan = PaintPlan::default();
    let both = |name: &str, color: [f32; 3], plan: &mut PaintPlan| {
        plan.materials.insert(name.to_string(), color);
        if settings.particles {
            plan.particles.insert(name.to_string(), color);
        }
    };
    let main = rgb(paint.colors[settings.variant], settings.multiplier);
    both(&settings.parameter, main, &mut plan);
    for a in &settings.additional {
        both(&a.parameter, rgb(paint.colors[a.variant], 1.0), &mut plan);
    }
    for o in settings.overrides.get(&paint.object).into_iter().flatten() {
        let color = match &o.paint {
            Some(name) => db
                .iter()
                .find(|p| &p.object == name)
                .map(|p| rgb(p.colors[o.variant], 1.0))
                .unwrap_or_else(|| rgb(o.custom, 1.0)),
            None => rgb(o.custom, 1.0),
        };
        plan.particles.insert(o.parameter.clone(), color);
    }
    plan
}

/// Stream edits painting the package with `paint`; the number of values
/// set. Each settings paints its own materials; particles and material
/// parameter expressions take every settings' values.
pub fn apply(
    pkg: &Package,
    body: &Body,
    all: &[PaintSettings],
    paint: &Paint,
    db: &[Paint],
) -> (StreamPatch, usize) {
    let plans: Vec<(&PaintSettings, PaintPlan)> = all
        .iter()
        .filter(|s| s.accepts(paint))
        .map(|s| (s, plan(s, paint, db)))
        .collect();
    let mut particles: HashMap<String, [f32; 3]> = HashMap::new();
    let mut expressions: HashMap<String, [f32; 3]> = HashMap::new();
    for (_, p) in &plans {
        particles.extend(p.particles.iter().map(|(k, v)| (k.clone(), *v)));
        expressions.extend(p.materials.iter().map(|(k, v)| (k.clone(), *v)));
    }
    let data = &body.data;
    let mut patch = StreamPatch::new();
    let mut count = 0;
    let mut put = |pos: usize, c: [f32; 3], patch: &mut StreamPatch| {
        let mut bytes = Vec::with_capacity(12);
        for v in c {
            bytes.extend(v.to_le_bytes());
        }
        patch.put(pos, bytes);
        count += 1;
    };
    for e in pkg.exports.iter().filter(|e| e.serial_size > 0) {
        let class = pkg.class_of(e);
        let Some(fields) = body
            .export_pos(e)
            .and_then(|p| property_block(data, p, pkg))
        else {
            continue;
        };
        let full = |f: &[Prop]| {
            props::find(f, "ParameterName").and_then(|p| p.as_full_name(data, &pkg.names))
        };
        match class {
            "MaterialInstanceConstant" => {
                let object = i32::try_from(e.index + 1).unwrap_or(0);
                let maps: Vec<&HashMap<String, [f32; 3]>> = plans
                    .iter()
                    .filter(|(s, _)| s.materials.is_empty() || s.materials.contains(&object))
                    .map(|(_, p)| &p.materials)
                    .collect();
                if maps.is_empty() {
                    continue;
                }
                let Some(list) = props::find(&fields, "VectorParameterValues") else {
                    continue;
                };
                for param in struct_array(data, list, pkg) {
                    let (Some(name), Some(value)) =
                        (full(&param), props::find(&param, "ParameterValue"))
                    else {
                        continue;
                    };
                    if let Some(c) = maps
                        .iter()
                        .find_map(|m| m.get(&name))
                        .filter(|_| value.size == 16)
                    {
                        put(value.value_pos, *c, &mut patch);
                    }
                }
            }
            "MaterialExpressionVectorParameter" => {
                if let (Some(name), Some(value)) =
                    (full(&fields), props::find(&fields, "DefaultValue"))
                {
                    if let Some(c) = expressions.get(&name).filter(|_| value.size == 16) {
                        put(value.value_pos, *c, &mut patch);
                    }
                }
            }
            c if c.starts_with("DistributionVectorParticleParameter") => {
                if let (Some(name), Some(value)) = (full(&fields), props::find(&fields, "Constant"))
                {
                    if let Some(c) = particles.get(&name).filter(|_| value.size == 12) {
                        put(value.value_pos, *c, &mut patch);
                    }
                }
            }
            _ => {}
        }
    }
    (patch, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paint(id: u8, object: &str, primary: [f32; 4]) -> Paint {
        let mut colors = [[0.0, 0.0, 0.0, 1.0]; 12];
        colors[0] = primary;
        colors[2] = [primary[0] / 2.0, primary[1] / 2.0, primary[2] / 2.0, 1.0];
        Paint {
            id,
            object: object.into(),
            label: object.into(),
            colors,
        }
    }

    fn settings() -> PaintSettings {
        PaintSettings {
            parameter: "CustomColor".into(),
            variant: 0,
            multiplier: 1.0,
            particles: true,
            include: vec![1, 2],
            unsupported: vec!["GoldMetal_00".into()],
            additional: vec![Additional {
                parameter: "DarkAccentColor".into(),
                variant: 2,
            }],
            overrides: HashMap::new(),
            materials: Vec::new(),
        }
    }

    #[test]
    fn plans_the_parameters_the_game_would_set() {
        let red = paint(1, "Red_00", [0.6, 0.0, 0.0, 1.0]);
        let db = vec![red.clone()];
        let mut s = settings();
        s.overrides.insert(
            "Red_00".into(),
            vec![Override {
                parameter: "Inf".into(),
                variant: 0,
                paint: None,
                custom: [1.0, 0.1, 0.1, 1.0],
            }],
        );
        let p = plan(&s, &red, &db);
        assert_eq!(p.materials["CustomColor"], [0.6, 0.0, 0.0]);
        assert_eq!(p.materials["DarkAccentColor"], [0.3, 0.0, 0.0]);
        assert_eq!(p.particles["CustomColor"], [0.6, 0.0, 0.0]);
        assert_eq!(p.particles["Inf"], [1.0, 0.1, 0.1]);

        s.particles = false;
        s.overrides.clear();
        assert!(plan(&s, &red, &db).particles.is_empty());
    }

    #[test]
    fn filters_accepted_paints() {
        let s = settings();
        assert!(s.accepts(&paint(1, "Red_00", [1.0; 4])));
        assert!(!s.accepts(&paint(3, "Black_00", [1.0; 4])), "not included");
        assert!(
            !s.accepts(&paint(2, "GoldMetal_00", [1.0; 4])),
            "unsupported"
        );
        assert!(!s.accepts(&paint(0, "None", [1.0; 4])));
    }

    #[test]
    fn maps_variants_and_display_colours() {
        assert_eq!(variant_index("PaintColorVariant_DarkAccent"), 2);
        assert_eq!(variant_index("PaintColorVariant_Emissive"), 3);
        assert_eq!(variant_index("weird"), 0);
        assert_eq!(
            paint(1, "Red_00", [1.0, 0.0, 0.0, 1.0]).info().hex,
            "#ff0000"
        );
        assert_eq!(
            paint(12, "White_00", [0.8, 0.8, 0.8, 1.0]).info().hex,
            "#e6e6e6"
        );
    }
}
