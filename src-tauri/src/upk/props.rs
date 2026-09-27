//! UE3 tagged properties (the property block at the start of most exports).
//!
//! Tag layout: `FName name (8) · FName type (8) · i32 size · i32 array_index`
//! followed by type-specific extras:
//! - `StructProperty`: `FName struct_name (8)` then `size` bytes,
//! - `ByteProperty`: `FName enum_name (8)` then `size` bytes (an FName when 8),
//! - `BoolProperty`: one value byte (`size` is 0),
//! - everything else: `size` bytes.
//!
//! The block ends with the name `None`.

use crate::upk::error::{UpkError, UpkResult};
use crate::upk::names::NameTable;
use crate::upk::reader::{read_i32, read_u32};

#[derive(Debug, Clone)]
pub struct Prop {
    pub name: String,
    pub type_name: String,
    pub array_index: i32,
    /// Struct name (StructProperty) or enum name (ByteProperty).
    pub inner: Option<String>,
    /// Offset of the tag in the buffer.
    pub tag_pos: usize,
    /// Offset and size of the value.
    pub value_pos: usize,
    pub size: usize,
}

impl Prop {
    pub fn as_i32(&self, buf: &[u8]) -> Option<i32> {
        (self.size >= 4)
            .then(|| read_i32(buf, self.value_pos))
            .flatten()
    }

    /// Value of a NameProperty / enum ByteProperty.
    pub fn as_name<'a>(&self, buf: &[u8], names: &'a NameTable) -> Option<&'a str> {
        if self.size == 8 {
            read_i32(buf, self.value_pos).map(|i| names.get(i))
        } else {
            None
        }
    }

    pub fn as_bool(&self, buf: &[u8]) -> Option<bool> {
        buf.get(self.value_pos).map(|&b| b != 0)
    }
}

const MAX_PROPS: usize = 512;

/// Walks tags from `start` until `None`. Returns the tags and the offset
/// just past the terminating `None` (8 bytes).
pub fn walk(buf: &[u8], start: usize, names: &NameTable) -> UpkResult<(Vec<Prop>, usize)> {
    let mut pos = start;
    let mut props = Vec::new();
    for _ in 0..MAX_PROPS {
        let name_idx = read_i32(buf, pos).ok_or(UpkError::Truncated("property tag"))?;
        let name = names.get(name_idx);
        if name_idx < 0 || name.is_empty() {
            return Err(UpkError::Format(format!(
                "invalid property name index {name_idx} at {pos}"
            )));
        }
        if name == "None" {
            return Ok((props, pos + 8));
        }
        let type_idx = read_i32(buf, pos + 8).ok_or(UpkError::Truncated("property type"))?;
        let type_name = names.get(type_idx);
        if !type_name.ends_with("Property") {
            return Err(UpkError::Format(format!(
                "{name}: {type_name:?} is not a property type"
            )));
        }
        let size = read_u32(buf, pos + 16).ok_or(UpkError::Truncated("property size"))? as usize;
        let array_index = read_i32(buf, pos + 20).ok_or(UpkError::Truncated("property index"))?;
        let mut value_pos = pos + 24;
        let mut inner = None;
        let mut value_len = size;
        match type_name {
            "StructProperty" | "ByteProperty" => {
                let idx =
                    read_i32(buf, value_pos).ok_or(UpkError::Truncated("property inner name"))?;
                inner = Some(names.get(idx).to_string());
                value_pos += 8;
            }
            "BoolProperty" => value_len = 1,
            _ => {}
        }
        if size > buf.len() || value_pos + value_len > buf.len() {
            return Err(UpkError::Truncated("property value"));
        }
        props.push(Prop {
            name: name.to_string(),
            type_name: type_name.to_string(),
            array_index,
            inner,
            tag_pos: pos,
            value_pos,
            size: value_len,
        });
        pos = value_pos + value_len;
    }
    Err(UpkError::Format("property block does not terminate".into()))
}

pub fn find<'a>(props: &'a [Prop], name: &str) -> Option<&'a Prop> {
    props.iter().find(|p| p.name == name)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::upk::names::tests::build;

    pub const NAMES: &[&str] = &[
        "None",
        "SizeX",
        "IntProperty",
        "Format",
        "ByteProperty",
        "EPixelFormat",
        "PF_DXT5",
        "SRGB",
        "BoolProperty",
        "TextureFileCacheName",
        "NameProperty",
        "Textures7",
        "SizeY",
    ];

    pub fn names() -> NameTable {
        NameTable::parse(&build(NAMES), NAMES.len())
    }

    fn idx(n: &str) -> i32 {
        NAMES.iter().position(|x| *x == n).unwrap() as i32
    }

    pub fn tag(out: &mut Vec<u8>, name: &str, ty: &str, size: u32) {
        out.extend(idx(name).to_le_bytes());
        out.extend(0i32.to_le_bytes());
        out.extend(idx(ty).to_le_bytes());
        out.extend(0i32.to_le_bytes());
        out.extend(size.to_le_bytes());
        out.extend(0i32.to_le_bytes());
    }

    pub fn fname(out: &mut Vec<u8>, name: &str) {
        out.extend(idx(name).to_le_bytes());
        out.extend(0i32.to_le_bytes());
    }

    /// SizeX=SizeY=`dim`, Format=PF_DXT5, SRGB=true, TFC=Textures7, None.
    pub fn texture_props(dim: i32) -> Vec<u8> {
        let mut b = Vec::new();
        tag(&mut b, "SizeX", "IntProperty", 4);
        b.extend(dim.to_le_bytes());
        tag(&mut b, "SizeY", "IntProperty", 4);
        b.extend(dim.to_le_bytes());
        tag(&mut b, "Format", "ByteProperty", 8);
        fname(&mut b, "EPixelFormat");
        fname(&mut b, "PF_DXT5");
        tag(&mut b, "SRGB", "BoolProperty", 0);
        b.push(1);
        tag(&mut b, "TextureFileCacheName", "NameProperty", 8);
        fname(&mut b, "Textures7");
        fname(&mut b, "None");
        b
    }

    #[test]
    fn walks_all_property_kinds() {
        let n = names();
        let buf = texture_props(256);
        let (props, end) = walk(&buf, 0, &n).unwrap();
        assert_eq!(end, buf.len());
        assert_eq!(find(&props, "SizeX").unwrap().as_i32(&buf), Some(256));
        assert_eq!(
            find(&props, "Format").unwrap().as_name(&buf, &n),
            Some("PF_DXT5")
        );
        assert_eq!(find(&props, "SRGB").unwrap().as_bool(&buf), Some(true));
        assert_eq!(
            find(&props, "TextureFileCacheName")
                .unwrap()
                .as_name(&buf, &n),
            Some("Textures7")
        );
    }

    #[test]
    fn rejects_garbage() {
        let n = names();
        assert!(walk(&[0xff; 32], 0, &n).is_err());
    }
}
