//! `MaterialInstanceConstant` exports: which texture each named parameter
//! binds (`TextureParameterValues`).
//!
//! The array is a tagged `ArrayProperty`: `i32 count`, then per element a
//! property block (`ParameterName`, `ParameterValue`, `ExpressionGUID`)
//! ending with `None`.

use crate::upk::package::{Body, Package};
use crate::upk::props;
use crate::upk::reader::read_i32;
use crate::upk::tables;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureParam {
    /// Name of the MIC export that declares the binding.
    pub material: String,
    pub parameter: String,
    /// ObjectIndex of the bound texture (> 0 export, < 0 import).
    pub object: i32,
    pub object_name: String,
}

/// Every texture parameter binding of every MIC in the package.
pub fn texture_params(pkg: &Package, body: &Body) -> Vec<TextureParam> {
    let mut out = Vec::new();
    for e in pkg
        .exports
        .iter()
        .filter(|e| e.serial_size > 0 && pkg.class_of(e) == "MaterialInstanceConstant")
    {
        let Some(pos) = body.export_pos(e) else {
            continue;
        };
        let Ok((list, _)) = props::walk(&body.data, pos + 4, &pkg.names) else {
            tracing::debug!(material = %e.object_name, "MIC properties unreadable");
            continue;
        };
        let Some(array) = props::find(&list, "TextureParameterValues") else {
            continue;
        };
        let buf = &body.data;
        let count = read_i32(buf, array.value_pos).unwrap_or(0).max(0);
        let mut at = array.value_pos + 4;
        for _ in 0..count {
            let Ok((fields, next)) = props::walk(buf, at, &pkg.names) else {
                break;
            };
            at = next;
            let parameter =
                props::find(&fields, "ParameterName").and_then(|p| p.as_name(buf, &pkg.names));
            let object = props::find(&fields, "ParameterValue").and_then(|p| p.as_i32(buf));
            if let (Some(parameter), Some(object)) = (parameter, object) {
                out.push(TextureParam {
                    material: e.object_name.clone(),
                    parameter: parameter.to_string(),
                    object,
                    object_name: tables::object_name(object, &pkg.imports, &pkg.exports)
                        .to_string(),
                });
            }
        }
    }
    out
}
