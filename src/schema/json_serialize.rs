use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize,
};

use crate::schema::{SchemaNode, SchemaType, SmallVecLen};

impl Serialize for SchemaNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry("name", &self.name)?;
        state.serialize_entry("type", &self.typ)?;
        state.end()
    }
}

impl SchemaType {
    fn typename(&self) -> &str {
        match self {
            SchemaType::Empty => "empty",
            SchemaType::Pubkey => "pubkey",
            SchemaType::String => "string",
            SchemaType::I8 => "i8",
            SchemaType::U8 => "u8",
            SchemaType::I16 => "i16",
            SchemaType::U16 => "u16",
            SchemaType::I32 => "i32",
            SchemaType::U32 => "u32",
            SchemaType::I64 => "i64",
            SchemaType::U64 => "u64",
            SchemaType::I128 => "i128",
            SchemaType::U128 => "u128",
            SchemaType::F32 => "f32",
            SchemaType::F64 => "f64",
            SchemaType::Bool => "bool",
            SchemaType::Option(_) => "option",
            SchemaType::Array(_, _) => "array",
            SchemaType::Tuple(_) => "tuple",
            SchemaType::Enum(_) => "enum",
            SchemaType::Vec(_) => "vec",
            SchemaType::Struct(_) => "struct",
            SchemaType::SmallVec(_, _) => "smallvec",
            SchemaType::RemainingBytes => "bytes_remaining",
        }
    }
}

impl Serialize for SchemaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self {
            SchemaType::Empty => serializer.serialize_none(),
            SchemaType::Option(inner_type) => {
                let mut state = serializer.serialize_map(Some(1))?;
                state.serialize_entry("type:option", inner_type)?;
                state.end()
            }
            SchemaType::Array(size, inner_type) => {
                let mut state = serializer.serialize_struct("type:array", 2)?;
                state.serialize_field("size", size)?;
                state.serialize_field("type", inner_type)?;
                state.end()
            }
            SchemaType::Tuple(types) => {
                let mut state = serializer.serialize_map(Some(1))?;
                state.serialize_entry("type:tuple", types)?;
                state.end()
            }
            SchemaType::Vec(inner_type) => {
                let mut state = serializer.serialize_map(Some(1))?;
                state.serialize_entry("type:vec", inner_type)?;
                state.end()
            }
            SchemaType::Struct(fields) => {
                let mut state = serializer.serialize_map(Some(fields.len()))?;
                for field in fields {
                    state.serialize_entry(&field.name, &field.typ)?;
                }
                state.end()
            }
            SchemaType::Enum(variants) => {
                let mut state = serializer.serialize_map(Some(1))?;
                state.serialize_entry("type:enum", &Variants { variants })?;
                state.end()
            }
            SchemaType::SmallVec(len_ty, elem) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                // Represent as: { "type:smallvec": { "len": "u8|u16", "elem": <SchemaType> } }
                #[derive(serde::Serialize)]
                struct SmallVecRepr<'a> {
                    len: &'a str,
                    #[serde(rename = "elem")]
                    elem: &'a SchemaType,
                }
                let len_str = match len_ty {
                    SmallVecLen::U8 => "u8",
                    SmallVecLen::U16 => "u16",
                };
                map.serialize_entry("type:smallvec", &SmallVecRepr { len: len_str, elem })?;
                map.end()
            }
            _ => Serialize::serialize(&self.typename(), serializer),
        }
    }
}

/// Helper struct for serializing enum variants
struct Variants<'a> {
    variants: &'a Vec<SchemaNode>,
}

impl Serialize for Variants<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut variants_state = serializer.serialize_map(Some(self.variants.len()))?;
        for variant in self.variants {
            variants_state.serialize_entry(&variant.name, &variant.typ)?;
        }
        variants_state.end()
    }
}
