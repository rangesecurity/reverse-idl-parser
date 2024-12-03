use serde::{ser::SerializeMap, Serialize, Serializer};
use solana_program::pubkey::Pubkey;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[repr(C)]
pub struct ValueNode {
    pub name: String,
    pub value: TypedValue,
}

impl ValueNode {
    pub fn new(name: impl Into<String>, value: TypedValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }

    pub fn new_struct(
        name: impl Into<String>,
        fields: Vec<(impl Into<String>, TypedValue)>,
    ) -> Self {
        Self::new(name, TypedValue::new_struct(fields))
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub enum TypedValue {
    Empty,
    Pubkey(String),
    String(String),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    I128(i128),
    U128(u128),
    F32(f32),
    F64(f64),
    Bool(bool),
    Option(Box<Option<TypedValue>>),
    /// the length is enforced by the schema, and the inner type should be the same
    Array(Vec<TypedValue>),
    /// the length is enforced by the schema, and the inner type can be different
    Tuple(Vec<TypedValue>),
    /// the inner value would be a variant of the enum
    Enum(Box<ValueNode>),
    /// variable length, while the inner type should be the same
    Vec(Vec<TypedValue>),
    /// list of fields, the name of struct is stored in the outer ValueNode
    Struct(Vec<ValueNode>),
}

impl TypedValue {
    pub fn new_struct(fields: Vec<(impl Into<String>, TypedValue)>) -> Self {
        let nodes = fields
            .into_iter()
            .map(|(name, value)| ValueNode::new(name.into(), value))
            .collect();
        Self::Struct(nodes)
    }
}

macro_rules! type_conversion {
    ($($t:ty => $v:ident)*) => ($(
        impl From<$t> for TypedValue {
            fn from(value: $t) -> Self {
                TypedValue::$v(value)
            }
        }
    )*)
}

type_conversion!(
    String => String
    i8 => I8
    u8 => U8
    i16 => I16
    u16 => U16
    i32 => I32
    u32 => U32
    i64 => I64
    u64 => U64
    i128 => I128
    u128 => U128
    f32 => F32
    f64 => F64
    bool => Bool
);

impl From<Pubkey> for TypedValue {
    fn from(value: Pubkey) -> Self {
        TypedValue::Pubkey(value.to_string())
    }
}

impl<T: Into<TypedValue>> From<Option<T>> for TypedValue {
    fn from(value: Option<T>) -> Self {
        TypedValue::Option(Box::new(value.map(|v| v.into())))
    }
}

impl<T: Into<TypedValue>> From<Vec<T>> for TypedValue {
    fn from(value: Vec<T>) -> Self {
        TypedValue::Vec(value.into_iter().map(|v| v.into()).collect())
    }
}

impl Serialize for TypedValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self {
            TypedValue::Empty => serializer.serialize_str(""),
            TypedValue::Pubkey(v) => v.serialize(serializer),
            TypedValue::String(v) => v.serialize(serializer),
            TypedValue::I8(v) => v.serialize(serializer),
            TypedValue::U8(v) => v.serialize(serializer),
            TypedValue::I16(v) => v.serialize(serializer),
            TypedValue::U16(v) => v.serialize(serializer),
            TypedValue::I32(v) => v.serialize(serializer),
            TypedValue::U32(v) => v.serialize(serializer),
            TypedValue::I64(v) => v.to_string().serialize(serializer),
            TypedValue::U64(v) => v.to_string().serialize(serializer),
            TypedValue::I128(v) => v.to_string().serialize(serializer),
            TypedValue::U128(v) => v.to_string().serialize(serializer),
            TypedValue::F32(v) => v.to_string().serialize(serializer),
            TypedValue::F64(v) => v.to_string().serialize(serializer),
            TypedValue::Bool(v) => v.serialize(serializer),
            TypedValue::Option(v) => v.serialize(serializer),
            TypedValue::Array(v) => v.serialize(serializer),
            TypedValue::Tuple(v) => v.serialize(serializer),
            TypedValue::Enum(v) => {
                if matches!(v.value, TypedValue::Empty) {
                    v.name.serialize(serializer)
                } else {
                    v.serialize(serializer)
                }
            }
            TypedValue::Vec(v) => v.serialize(serializer),
            TypedValue::Struct(v) => {
                let mut state = serializer.serialize_map(Some(v.len()))?;
                for field in v {
                    state.serialize_entry(&field.name, &field.value)?;
                }
                state.end()
            }
        }
    }
}
