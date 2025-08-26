use crate::schema::SmallVecLen;

use super::{SchemaNode, SchemaType};
use borsh::{BorshDeserialize, BorshSerialize};

impl BorshDeserialize for SchemaType {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let tag = u16::deserialize_reader(reader)?;
        let output = match tag {
            0 => SchemaType::Empty,
            1 => SchemaType::Pubkey,
            2 => SchemaType::String,
            3 => SchemaType::I8,
            4 => SchemaType::U8,
            5 => SchemaType::I16,
            6 => SchemaType::U16,
            7 => SchemaType::I32,
            8 => SchemaType::U32,
            9 => SchemaType::I64,
            10 => SchemaType::U64,
            11 => SchemaType::I128,
            12 => SchemaType::U128,
            13 => SchemaType::F32,
            14 => SchemaType::F64,
            15 => SchemaType::Bool,
            16 => SchemaType::Option(Box::new(SchemaType::deserialize_reader(reader)?)),
            17 => SchemaType::Array(
                usize::deserialize_reader(reader)?,
                Box::new(SchemaType::deserialize_reader(reader)?),
            ),
            18 => {
                let len = usize::deserialize_reader(reader)?;
                let mut types = Vec::with_capacity(len);
                for _ in 0..len {
                    types.push(SchemaType::deserialize_reader(reader)?);
                }
                SchemaType::Tuple(types)
            }
            19 => SchemaType::Vec(Box::new(SchemaType::deserialize_reader(reader)?)),
            20 | 21 => {
                let len = usize::deserialize_reader(reader)?;
                let mut nodes = Vec::with_capacity(len);
                for _ in 0..len {
                    nodes.push(SchemaNode::deserialize_reader(reader)?);
                }
                match tag {
                    20 => SchemaType::Struct(nodes),
                    21 => SchemaType::Enum(nodes),
                    _ => unreachable!(),
                }
            }
            22 => {
                let len_ty = SmallVecLen::deserialize_reader(reader)?;
                let elem = SchemaType::deserialize_reader(reader)?;
                SchemaType::SmallVec(len_ty, Box::new(elem))
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid tag: {}", tag),
                ))
            }
        };
        Ok(output)
    }
}

impl BorshSerialize for SchemaType {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // write tag first
        let tag: u16 = match self {
            SchemaType::Empty => 0,
            SchemaType::Pubkey => 1,
            SchemaType::String => 2,
            SchemaType::I8 => 3,
            SchemaType::U8 => 4,
            SchemaType::I16 => 5,
            SchemaType::U16 => 6,
            SchemaType::I32 => 7,
            SchemaType::U32 => 8,
            SchemaType::I64 => 9,
            SchemaType::U64 => 10,
            SchemaType::I128 => 11,
            SchemaType::U128 => 12,
            SchemaType::F32 => 13,
            SchemaType::F64 => 14,
            SchemaType::Bool => 15,
            SchemaType::Option(_) => 16,
            SchemaType::Array(_, _) => 17,
            SchemaType::Tuple(_) => 18,
            SchemaType::Vec(_) => 19,
            SchemaType::Struct(_) => 20,
            SchemaType::Enum(_) => 21,
            SchemaType::SmallVec(_, _) => 22,
        };
        BorshSerialize::serialize(&tag, writer)?;
        match self {
            SchemaType::Option(typ) => {
                BorshSerialize::serialize(typ, writer)?;
            }
            SchemaType::Array(len, typ) => {
                BorshSerialize::serialize(&len, writer)?;
                BorshSerialize::serialize(typ, writer)?;
            }
            SchemaType::Vec(typ) => {
                BorshSerialize::serialize(&typ, writer)?;
            }
            SchemaType::Struct(nodes) => {
                BorshSerialize::serialize(&nodes.len(), writer)?;
                for node in nodes {
                    BorshSerialize::serialize(node, writer)?;
                }
            }
            SchemaType::Enum(variants) => {
                BorshSerialize::serialize(&variants.len(), writer)?;
                for variant in variants {
                    BorshSerialize::serialize(variant, writer)?;
                }
            }
            SchemaType::Tuple(types) => {
                BorshSerialize::serialize(&types.len(), writer)?;
                for typ in types {
                    BorshSerialize::serialize(typ, writer)?;
                }
            }
            SchemaType::SmallVec(len_ty, typ) => {
                borsh::BorshSerialize::serialize(len_ty, writer)?;
                borsh::BorshSerialize::serialize(&**typ, writer)?;
            }
            _ => (),
        }
        Ok(())
    }
}
