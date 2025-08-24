use crate::{
    schema::{SchemaNode, SchemaType},
    value::{TypedValue, ValueNode},
};
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;

impl SchemaNode {
    pub fn deserialize_bytes(
        &self,
        bytes: &mut &[u8],
        show_hidden: bool,
    ) -> anyhow::Result<Option<ValueNode>> {
        let value = self.typ.deserialize_bytes(&mut *bytes, show_hidden)?;
        if self.is_hidden && !show_hidden {
            Ok(None)
        } else {
            Ok(Some(ValueNode::new(self.name.clone(), value)))
        }
    }
}

impl SchemaType {
    /// Ref: [Borsh Spec](https://borsh.io/#pills-specification)
    pub fn deserialize_bytes(
        &self,
        bytes: &mut &[u8],
        show_hidden: bool,
    ) -> anyhow::Result<TypedValue> {
        let value = match self {
            SchemaType::Empty => TypedValue::Empty,
            SchemaType::Pubkey => {
                TypedValue::Pubkey(Pubkey::deserialize_reader(&mut *bytes)?.to_string())
            }
            SchemaType::String => TypedValue::String(String::deserialize_reader(&mut *bytes)?),
            SchemaType::I8 => TypedValue::I8(i8::deserialize_reader(&mut *bytes)?),
            SchemaType::U8 => TypedValue::U8(u8::deserialize_reader(&mut *bytes)?),
            SchemaType::I16 => TypedValue::I16(i16::deserialize_reader(&mut *bytes)?),
            SchemaType::U16 => TypedValue::U16(u16::deserialize_reader(&mut *bytes)?),
            SchemaType::I32 => TypedValue::I32(i32::deserialize_reader(&mut *bytes)?),
            SchemaType::U32 => TypedValue::U32(u32::deserialize_reader(&mut *bytes)?),
            SchemaType::I64 => TypedValue::I64(i64::deserialize_reader(&mut *bytes)?),
            SchemaType::U64 => TypedValue::U64(u64::deserialize_reader(&mut *bytes)?),
            SchemaType::I128 => TypedValue::I128(i128::deserialize_reader(&mut *bytes)?),
            SchemaType::U128 => TypedValue::U128(u128::deserialize_reader(&mut *bytes)?),
            SchemaType::F32 => TypedValue::F32(f32::deserialize_reader(&mut *bytes)?),
            SchemaType::F64 => TypedValue::F64(f64::deserialize_reader(&mut *bytes)?),
            SchemaType::Bool => TypedValue::Bool(bool::deserialize_reader(&mut *bytes)?),
            SchemaType::Option(t) => TypedValue::Option(Box::new({
                // Option discriminant is 1 byte (u8), 0 => None, 1 => Some
                let is_some = u8::deserialize_reader(&mut *bytes)?;
                if is_some == 1 {
                    Some(t.deserialize_bytes(&mut *bytes, show_hidden)?)
                } else {
                    None
                }
            })),
            SchemaType::Array(size, t) => TypedValue::Array({
                let mut values = Vec::with_capacity(*size);
                for _ in 0..*size {
                    values.push(t.deserialize_bytes(&mut *bytes, show_hidden)?);
                }
                values
            }),
            SchemaType::Tuple(t) => TypedValue::Tuple({
                let mut values = Vec::with_capacity(t.len());
                for t in t {
                    values.push(t.deserialize_bytes(&mut *bytes, show_hidden)?);
                }
                values
            }),
            SchemaType::Vec(t) => {
                // SPECIAL CASE: bytes => Vec<u8>  (Anchor "bytes" = Borsh Vec<u8>)
                if matches!(**t, SchemaType::U8) {
                    let size = u32::deserialize_reader(&mut *bytes)? as usize;

                    if bytes.len() < size {
                        return Err(anyhow::anyhow!(
                            "Not enough bytes for Vec<u8>: need {}, have {}",
                            size,
                            bytes.len()
                        ));
                    }

                    // Efficiently take next `size` bytes
                    let (raw, rest) = bytes.split_at(size);
                    let buf = raw.to_vec();
                    *bytes = rest;

                    TypedValue::Bytes(buf)
                } else {
                    // Generic Vec<T> path (unchanged)
                    let size = u32::deserialize_reader(&mut *bytes)?;
                    let mut values = Vec::with_capacity(size as usize);
                    for _ in 0..size {
                        values.push(t.deserialize_bytes(&mut *bytes, show_hidden)?);
                    }
                    TypedValue::Vec(values)
                }
            }
            SchemaType::Struct(t) => TypedValue::Struct({
                let mut values = Vec::with_capacity(t.len());
                for t in t {
                    if let Some(val) = t.deserialize_bytes(&mut *bytes, show_hidden)? {
                        values.push(val);
                    }
                }
                values
            }),
            SchemaType::Enum(t) => TypedValue::Enum({
                // Enum discriminant is 1 byte (u8)
                let discriminant = u8::deserialize_reader(&mut *bytes)?;
                let value = t[discriminant as usize]
                    .deserialize_bytes(&mut *bytes, show_hidden)?
                    .ok_or(anyhow::anyhow!("is_hidden shouldn't appear in Enum types"))?;
                Box::new(value)
            }),
        };
        Ok(value)
    }
}
