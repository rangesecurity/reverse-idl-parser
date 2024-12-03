mod bytes_deserialize;
mod json_serialize;
mod on_chain_serialization;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
#[repr(C)]
pub struct SchemaNode {
    pub name: String,
    pub typ: SchemaType,
    pub is_hidden: bool,
}

impl SchemaNode {
    pub fn new(name: impl Into<String>, typ: SchemaType) -> Self {
        Self {
            name: name.into(),
            typ,
            is_hidden: false,
        }
    }
    pub fn new_struct(
        name: impl Into<String>,
        fields: Vec<(impl Into<String>, SchemaType)>,
    ) -> Self {
        let types = SchemaType::Struct(
            fields
                .into_iter()
                .map(|(name, typ)| SchemaNode::new(name, typ))
                .collect(),
        );
        Self::new(name, types)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum SchemaType {
    Empty,
    Pubkey,
    String,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    I128,
    U128,
    F32,
    F64,
    Bool,
    Option(Box<SchemaType>),
    Array(usize, Box<SchemaType>),
    Tuple(Vec<SchemaType>),
    Vec(Box<SchemaType>),
    Struct(Vec<SchemaNode>),
    Enum(Vec<SchemaNode>),
}

impl SchemaType {
    pub fn option(typ: SchemaType) -> Self {
        Self::Option(Box::new(typ))
    }

    pub fn vec(typ: SchemaType) -> Self {
        Self::Vec(Box::new(typ))
    }

    pub fn array(len: usize, typ: SchemaType) -> Self {
        Self::Array(len, Box::new(typ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        schema::{SchemaNode, SchemaType},
        value::{TypedValue, ValueNode},
    };
    use borsh::{BorshDeserialize, BorshSerialize};

    #[test]
    fn identical_after_serialization() {
        let market_size_params_struct = SchemaNode::new_struct(
            "MarketSizeParams",
            vec![
                ("bidsSize", SchemaType::U64),
                ("asksSize", SchemaType::U64),
                ("numSeats", SchemaType::U64),
            ],
        );

        let schema = SchemaNode::new_struct(
            "InitializeParams",
            vec![
                ("marketSizeParams", market_size_params_struct.typ),
                ("numQuoteLotsPerQuoteUnit", SchemaType::U64),
                ("tickSizeInQuoteLotsPerBaseUnit", SchemaType::U64),
                ("numBaseLotsPerBaseUnit", SchemaType::U64),
                ("takerFeeBps", SchemaType::U16),
                ("feeCollector", SchemaType::Pubkey),
                (
                    "rawBaseUnitsPerBaseUnit",
                    SchemaType::option(SchemaType::U32),
                ),
            ],
        );
        let serialized = schema.try_to_vec().unwrap();
        let deserialized = SchemaNode::deserialize(&mut serialized.as_slice()).unwrap();
        assert_eq!(schema, deserialized);
    }

    #[repr(C)]
    #[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
    pub struct MarketSizeParams {
        pub bids_size: u64,
        pub asks_size: u64,
        pub num_seats: u64,
    }

    #[test]
    fn deserialize_market_size_params() {
        let market_size_params = MarketSizeParams {
            bids_size: 100,
            asks_size: 50,
            num_seats: 10,
        };

        let market_size_params_struct = SchemaNode::new_struct(
            "MarketSizeParams",
            vec![
                ("bidsSize", SchemaType::U64),
                ("asksSize", SchemaType::U64),
                ("numSeats", SchemaType::U64),
            ],
        );

        let serialized_data = market_size_params.try_to_vec().unwrap();
        let deserialized_data = market_size_params_struct
            .deserialize_bytes(&mut serialized_data.as_slice(), true)
            .unwrap()
            .unwrap();

        assert_eq!(
            deserialized_data,
            ValueNode::new_struct(
                "MarketSizeParams",
                vec![
                    ("bidsSize", TypedValue::U64(100)),
                    ("asksSize", TypedValue::U64(50)),
                    ("numSeats", TypedValue::U64(10)),
                ],
            )
        );

        assert_eq!(
            serde_json::to_string(&deserialized_data).unwrap(),
            r#"{"name":"MarketSizeParams","value":{"bidsSize":"100","asksSize":"50","numSeats":"10"}}"#
        );
    }

    #[test]
    fn json_serialization() {
        let market_size_params_struct = SchemaNode::new_struct(
            "MarketSizeParams",
            vec![
                ("bidsSize", SchemaType::U64),
                ("asksSize", SchemaType::U64),
                ("numSeats", SchemaType::U64),
            ],
        );

        let schema = SchemaNode::new_struct(
            "InitializeParams",
            vec![
                ("marketSizeParams", market_size_params_struct.typ.clone()),
                ("numQuoteLotsPerQuoteUnit", SchemaType::U64),
                ("tickSizeInQuoteLotsPerBaseUnit", SchemaType::U64),
                ("numBaseLotsPerBaseUnit", SchemaType::U64),
                ("takerFeeBps", SchemaType::U16),
                ("feeCollector", SchemaType::Pubkey),
                (
                    "rawBaseUnitsPerBaseUnit",
                    SchemaType::option(SchemaType::U32),
                ),
                (
                    "test_vec",
                    SchemaType::vec(market_size_params_struct.typ.clone()),
                ),
                (
                    "test_tuple",
                    SchemaType::Tuple(vec![
                        market_size_params_struct.typ.clone(),
                        SchemaType::Pubkey,
                        SchemaType::U32,
                    ]),
                ),
                (
                    "test_enum",
                    SchemaType::Enum(vec![
                        SchemaNode::new("Variant1", SchemaType::U64),
                        SchemaNode::new("Variant2", SchemaType::Pubkey),
                        SchemaNode::new("Variant3", SchemaType::Empty),
                    ]),
                ),
            ],
        );
        let serialized = serde_json::to_string_pretty(&schema).unwrap();
        println!("{}", serialized);
    }
}
