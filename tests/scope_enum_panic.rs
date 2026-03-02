use atlas_idl_schema::{parse_idl, schema::SchemaType};

/// Minimal Scope IDL containing only the types needed for
/// the `updateMappingAndMetadata` instruction that triggered the crash.
fn scope_idl_fragment() -> String {
    r#"{
        "version": "0.26.1",
        "name": "scope",
        "instructions": [
            {
                "name": "updateMappingAndMetadata",
                "accounts": [
                    { "name": "admin", "isMut": false, "isSigner": true },
                    { "name": "configuration", "isMut": false, "isSigner": false },
                    { "name": "oracleMappings", "isMut": true, "isSigner": false },
                    { "name": "tokensMetadata", "isMut": true, "isSigner": false },
                    { "name": "oraclePrices", "isMut": true, "isSigner": false },
                    { "name": "oracleTwaps", "isMut": true, "isSigner": false }
                ],
                "args": [
                    { "name": "feedName", "type": "string" },
                    {
                        "name": "updates",
                        "type": { "vec": { "defined": "UpdateOracleMappingAndMetadataEntriesWithId" } }
                    }
                ]
            }
        ],
        "types": [
            {
                "name": "UpdateOracleMappingAndMetadataEntriesWithId",
                "type": {
                    "kind": "struct",
                    "fields": [
                        { "name": "entryId", "type": "u16" },
                        {
                            "name": "updates",
                            "type": { "vec": { "defined": "UpdateOracleMappingAndMetadataEntry" } }
                        }
                    ]
                }
            },
            {
                "name": "UpdateOracleMappingAndMetadataEntry",
                "type": {
                    "kind": "enum",
                    "variants": [
                        { "name": "RemoveEntry" },
                        {
                            "name": "MappingConfig",
                            "fields": [
                                { "name": "priceType", "type": { "defined": "OracleType" } },
                                { "name": "genericData", "type": { "array": ["u8", 20] } }
                            ]
                        },
                        {
                            "name": "MappingTwapEntry",
                            "fields": [
                                { "name": "priceType", "type": { "defined": "OracleType" } },
                                { "name": "twapSource", "type": "u16" }
                            ]
                        },
                        { "name": "MappingTwapEnabledBitmask", "fields": ["u8"] },
                        {
                            "name": "MappingRefPrice",
                            "fields": [
                                { "name": "refPriceIndex", "type": { "option": "u16" } },
                                { "name": "refPriceToleranceBps", "type": { "option": "u16" } }
                            ]
                        },
                        { "name": "MetadataName", "fields": ["string"] },
                        { "name": "MetadataMaxPriceAgeSlots", "fields": ["u64"] },
                        { "name": "MetadataGroupIdsBitset", "fields": ["u64"] }
                    ]
                }
            },
            {
                "name": "OracleType",
                "type": {
                    "kind": "enum",
                    "variants": [
                        { "name": "Unused" },
                        { "name": "DeprecatedPlaceholder1" },
                        { "name": "DeprecatedPlaceholder2" },
                        { "name": "DeprecatedPlaceholder3" },
                        { "name": "DeprecatedPlaceholder4" },
                        { "name": "SplStake" },
                        { "name": "KToken" },
                        { "name": "DeprecatedPlaceholder5" },
                        { "name": "MsolStake" },
                        { "name": "KTokenToTokenA" },
                        { "name": "KTokenToTokenB" },
                        { "name": "JupiterLpFetch" },
                        { "name": "ScopeTwap1h" },
                        { "name": "OrcaWhirlpoolAtoB" },
                        { "name": "OrcaWhirlpoolBtoA" },
                        { "name": "RaydiumAmmV3AtoB" },
                        { "name": "RaydiumAmmV3BtoA" },
                        { "name": "DeprecatedPlaceholder6" },
                        { "name": "MeteoraDlmmAtoB" },
                        { "name": "MeteoraDlmmBtoA" },
                        { "name": "DeprecatedPlaceholder7" },
                        { "name": "PythPull" },
                        { "name": "PythPullEMA" },
                        { "name": "FixedPrice" },
                        { "name": "SwitchboardOnDemand" },
                        { "name": "JitoRestaking" },
                        { "name": "Chainlink" },
                        { "name": "DiscountToMaturity" },
                        { "name": "MostRecentOf" },
                        { "name": "PythLazer" },
                        { "name": "RedStone" },
                        { "name": "AdrenaLp" },
                        { "name": "Securitize" },
                        { "name": "CappedFloored" },
                        { "name": "ChainlinkRWA" },
                        { "name": "ChainlinkNAV" },
                        { "name": "FlashtradeLp" },
                        { "name": "ChainlinkX" },
                        { "name": "ChainlinkExchangeRate" },
                        { "name": "CappedMostRecentOf" },
                        { "name": "ScopeTwap8h" },
                        { "name": "ScopeTwap24h" }
                    ]
                }
            }
        ]
    }"#
    .to_string()
}

#[test]
fn scope_update_mapping_and_metadata_deserializes_correctly() {
    // Regression test for the crash:
    //   index out of bounds: the len is 42 but the index is 244
    // at bytes_deserialize.rs:126
    //
    // The instruction data is from a real Scope/Hubble transaction that
    // calls updateMappingAndMetadata with 5 MappingRefPrice entries.
    let idl = parse_idl::parse_idl(scope_idl_fragment())
        .expect("parse_idl should succeed for scope IDL");

    // 86-byte instruction data (8-byte Anchor disc + 78 bytes of args)
    let instruction_data: Vec<u8> = vec![
        // Anchor discriminator for updateMappingAndMetadata
        0x9e, 0x51, 0x95, 0x92, 0xce, 0x9a, 0x5b, 0x38,
        // feedName = "klend" (length-prefixed string)
        0x05, 0x00, 0x00, 0x00, 0x6b, 0x6c, 0x65, 0x6e, 0x64,
        // updates vec length = 5
        0x05, 0x00, 0x00, 0x00,
        // Entry 0: entryId=135, 1 update, MappingRefPrice(Some(17), Some(500))
        0x87, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x01, 0x11, 0x00, 0x01, 0xf4, 0x01,
        // Entry 1: entryId=140, 1 update, MappingRefPrice(Some(137), Some(500))
        0x8c, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x01, 0x89, 0x00, 0x01, 0xf4, 0x01,
        // Entry 2: entryId=141, 1 update, MappingRefPrice(Some(137), Some(500))
        0x8d, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x01, 0x89, 0x00, 0x01, 0xf4, 0x01,
        // Entry 3: entryId=146, 1 update, MappingRefPrice(Some(143), Some(500))
        0x92, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x01, 0x8f, 0x00, 0x01, 0xf4, 0x01,
        // Entry 4: entryId=147, 1 update, MappingRefPrice(Some(143), Some(500))
        0x93, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x01, 0x8f, 0x00, 0x01, 0xf4, 0x01,
    ];

    let account_keys: Vec<String> = vec![
        "CzwQ3dFHekGbHcGYNwUHAjShX9KmhFdWsfJBmYFMHoh7".into(),
        "6cMwdbrJ95D7v5655Zsoe7oXmjQJMnagWK8EcdG6qmGM".into(),
        "4zh6bmb77qX2CL7t5AJYCqa6YqFafbz3QJNeFvZjLowg".into(),
        "3wHxoHowen78mskgqKQmaYVQV8Mqd5PUFXja2xcfviSV".into(),
        "3t4JZcueEzTbVP6kLxXrL3VpWx45jDer4eqysweBchNH".into(),
        "6L6vUts9tYqxHVUCEFVc2mzZw6yxMn8C6a44cp5ga7e9".into(),
    ];

    let result = idl.get_parsed_instruction(instruction_data, &account_keys, false);
    assert!(
        result.is_ok(),
        "Deserialization should succeed but got: {:?}",
        result.err()
    );

    let parsed = result.unwrap();
    assert_eq!(parsed.name, "updateMappingAndMetadata");

    let json = serde_json::to_string(&parsed.value).unwrap();
    assert!(json.contains("klend"), "Should contain feedName 'klend'");
}

#[test]
fn enum_out_of_bounds_returns_error_not_panic() {
    // The enum discriminant bounds check should return an error,
    // not panic with index out of bounds.
    use atlas_idl_schema::schema::SchemaNode;

    let schema = SchemaType::Enum(vec![
        SchemaNode::new("A", SchemaType::Empty),
        SchemaNode::new("B", SchemaType::U8),
    ]);

    // discriminant = 255, but only 2 variants exist
    let mut data: &[u8] = &[255];
    let result = schema.deserialize_bytes(&mut data, false);
    assert!(
        result.is_err(),
        "Should return error for out-of-bounds discriminant"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("out of bounds"),
        "Error should mention 'out of bounds', got: {}",
        err_msg
    );
}

#[test]
fn array_of_defined_type_preserves_array_wrapper() {
    // Regression test: {"array": [{"defined": "SomeType"}, N]} must produce
    // Array(N, <resolved type>), not just <resolved type>.
    let json = r#"{
        "version": "1.0.0",
        "name": "arr_test",
        "instructions": [
            {
                "name": "doStuff",
                "accounts": [],
                "args": [
                    {
                        "name": "items",
                        "type": { "array": [{ "defined": "MyItem" }, 3] }
                    }
                ]
            }
        ],
        "types": [
            {
                "name": "MyItem",
                "type": {
                    "kind": "struct",
                    "fields": [
                        { "name": "val", "type": "u16" }
                    ]
                }
            }
        ]
    }"#;

    let idl = parse_idl::parse_idl(json.to_string()).expect("parse_idl ok");
    let (_disc, dec) = &idl.instruction_params[0];

    match &dec.instruction_args_parser.typ {
        SchemaType::Struct(fields) => {
            let items = fields.iter().find(|f| f.name == "items").unwrap();
            match &items.typ {
                SchemaType::Array(3, inner) => {
                    assert!(
                        matches!(&**inner, SchemaType::Struct(_)),
                        "Expected Struct inside Array, got {:?}",
                        inner
                    );
                }
                other => panic!("Expected Array(3, Struct), got {:?}", other),
            }
        }
        other => panic!("Expected Struct, got {:?}", other),
    }

    // Verify deserialization works: 3 items of u16
    let mut data = Vec::new();
    data.extend_from_slice(&100u16.to_le_bytes());
    data.extend_from_slice(&200u16.to_le_bytes());
    data.extend_from_slice(&300u16.to_le_bytes());

    let result = dec
        .instruction_args_parser
        .deserialize_bytes(&mut data.as_slice(), false)
        .expect("deserialization should succeed");

    assert!(result.is_some());
}
