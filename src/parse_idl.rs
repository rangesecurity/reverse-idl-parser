use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use borsh::{BorshDeserialize, BorshSerialize};
use serde_json::{Map, Value};
use solana_program::hash::hash;

use crate::{
    on_chain_idl::{InstructionDecoder, OnChainIdl},
    schema::{SchemaNode, SchemaType, SmallVecLen},
};

pub fn parse_idl_file(file_path: &str) -> Result<OnChainIdl, Box<dyn std::error::Error>> {
    parse_idl(std::fs::read_to_string(file_path)?)
}

pub fn parse_idl(json_str: String) -> Result<OnChainIdl, Box<dyn std::error::Error>> {
    let json: Value = serde_json::from_str(&json_str)?;
    let root = json.as_object().ok_or("Root is not an object")?;

    let mut idl_type_map = parse_types(root)?;
    parse_accounts(root, &mut idl_type_map)?;

    let mut idl_parser = IdlParser::new(idl_type_map);
    let schema_map = idl_parser.parse()?;

    let (accounts, account_disc_len) = parse_account_schemas(root, &schema_map)?;
    let (instruction_params, instruction_disc_len) = parse_instructions(root, &mut idl_parser)?;

    let on_chain_idl = OnChainIdl {
        program_name: root
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        account_disc_len,
        instruction_disc_len,
        accounts: accounts.into_iter().collect(),
        instruction_params: instruction_params.into_iter().collect(),
    };

    validate_on_chain_idl(&on_chain_idl)?;

    Ok(on_chain_idl)
}

fn parse_types(
    root: &Map<String, Value>,
) -> Result<HashMap<String, Map<String, Value>>, Box<dyn std::error::Error>> {
    let mut idl_type_map = HashMap::new();
    let type_map_list = root
        .get("types")
        .and_then(|v| v.as_array())
        .ok_or("Types is not an array")?;
    for raw_type_map in type_map_list {
        let type_map = raw_type_map
            .as_object()
            .ok_or("Type map is not an object")?;
        let type_name = type_map
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Type name is not a string")?;
        idl_type_map.insert(type_name.to_string(), type_map.clone());
    }
    Ok(idl_type_map)
}

fn parse_accounts(
    root: &Map<String, Value>,
    idl_type_map: &mut HashMap<String, Map<String, Value>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let account_map_list = root
        .get("accounts")
        .and_then(|s| s.as_array().cloned())
        .unwrap_or_default();

    for raw_account_map in account_map_list {
        let account_map = raw_account_map
            .as_object()
            .ok_or("Account map is not an object")?;
        let account_name = account_map
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Account name is not a string")?;

        // Only merge if this account carries an inline layout (legacy style).
        if account_map.get("type").is_some() {
            // Don’t overwrite an existing proper type definition
            idl_type_map
                .entry(account_name.to_string())
                .or_insert_with(|| account_map.clone());
        }
    }
    Ok(())
}

fn parse_account_schemas(
    root: &Map<String, Value>,
    schema_map: &HashMap<String, SchemaNode>,
) -> Result<(HashMap<u64, SchemaNode>, u8), Box<dyn std::error::Error>> {
    let mut account_disc_types = HashSet::new();
    let mut accounts = HashMap::new();
    let account_map_list = root
        .get("accounts")
        .and_then(|s| s.as_array().cloned())
        .unwrap_or_default();

    for raw_account_map in account_map_list {
        let account_map = raw_account_map
            .as_object()
            .ok_or("Account map is not an object")?;
        let account_name = account_map
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Account name is not a string")?;

        // let (key, disc_type) = if let Some(disc) = account_map.get("discriminant") {
        //     parse_explicit_discriminant(disc)?
        // } else {
        //     parse_implicit_discriminant(account_name)?
        // };

        // account_disc_types.insert(disc_type);
        let (key, disc_len) = if let Some(disc) = account_map
            .get("discriminant")
            .or_else(|| account_map.get("discriminator"))
        {
            parse_any_discriminator(disc)?
        } else {
            let (key, _type_len) = parse_implicit_discriminant(account_name)?;
            (key, 8u8)
        };
        account_disc_types.insert(disc_len as u64);
        accounts.insert(
            key,
            schema_map
                .get(account_name)
                .ok_or("Account not found in schema map")?
                .clone(),
        );
    }

    if account_disc_types.len() > 1 {
        return Err(format!(
            "Multiple discriminant types found: {:?}",
            account_disc_types
        )
        .into());
    }

    let account_disc_len = *account_disc_types.iter().next().unwrap_or(&8) as u8;

    Ok((accounts, account_disc_len))
}

// helper: accepts either object {"type": "...", "value": ...} or byte array [u8;N]
// Returns (u64_value, disc_len_bytes)
fn parse_any_discriminator(v: &serde_json::Value) -> Result<(u64, u8), Box<dyn std::error::Error>> {
    if let Some(obj) = v.as_object() {
        // legacy/new object form: { type: "u64"|"u8", value: <u64> }
        let typ = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("Discriminant type is not a string")?;
        let disc_type_len = match typ {
            "u8" => 1u8,
            "u64" => 8u8,
            other => return Err(format!("Unknown discriminant type: {}", other).into()),
        };
        let val = obj
            .get("value")
            .and_then(|v| v.as_u64())
            .ok_or("Discriminant value is not a u64")?;
        return Ok((val, disc_type_len));
    }

    if let Some(arr) = v.as_array() {
        // byte array form: [149, 0, 150, 1, ...]
        // we interpret as little-endian; pad/truncate to 8 bytes to fit u64 (like your current reader).
        let mut padded = [0u8; 8];
        let n = arr.len().min(8);
        for i in 0..n {
            padded[i] = arr[i]
                .as_u64()
                .ok_or("Discriminator byte is not a number")? as u8;
        }
        let val = u64::from_le_bytes(padded);
        return Ok((val, arr.len() as u8));
    }

    Err("Unsupported discriminator/discriminant value; expected object or byte array".into())
}

fn parse_implicit_discriminant(
    account_name: &str,
) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let seeds = format!("account:{}", account_name).into_bytes();
    let key = u64::from_le_bytes(
        hash(&seeds).to_bytes()[..8]
            .try_into()
            .map_err(|_| "Failed to convert hash to u64")?,
    );
    Ok((key, 8))
}

fn parse_instructions(
    root: &Map<String, Value>,
    idl_parser: &mut IdlParser,
) -> Result<(HashMap<u64, InstructionDecoder>, u8), Box<dyn std::error::Error>> {
    let mut instruction_disc_types = HashSet::new();
    let mut instruction_params = HashMap::new();

    let instruction_map_list = root
        .get("instructions")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    for raw_instruction_map in instruction_map_list {
        let instruction_map = raw_instruction_map
            .as_object()
            .ok_or("Instruction map is not an object")?;
        let instruction_name = instruction_map
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Instruction name is not a string")?;

        let accounts = parse_instruction_accounts(instruction_map)?;
        let instruction_args = parse_instruction_args(instruction_map)?;

        let instruction_args_parser = if instruction_args.is_empty() {
            SchemaNode::new(instruction_name, SchemaType::Empty)
        } else {
            idl_parser.parse_fields(instruction_name, &instruction_args)?
        };

        let instruction_decoder = InstructionDecoder {
            accounts,
            instruction_args_parser,
        };

        let (key, disc_len) = if let Some(disc) = instruction_map
            .get("discriminant")
            .or_else(|| instruction_map.get("discriminator"))
        {
            parse_any_discriminator(disc)?
        } else {
            let (key, _type_len) = parse_implicit_instruction_discriminant(instruction_name)?;
            (key, 8u8)
        };
        instruction_disc_types.insert(disc_len as u64);
        instruction_params.insert(key, instruction_decoder);
    }

    if instruction_disc_types.len() > 1 {
        return Err(format!(
            "Multiple discriminant types found: {:?}",
            instruction_disc_types
        )
        .into());
    }

    let instruction_disc_len = *instruction_disc_types.iter().next().unwrap_or(&8) as u8;

    Ok((instruction_params, instruction_disc_len))
}

fn parse_instruction_accounts(
    instruction_map: &Map<String, Value>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut accounts = Vec::new();
    let accounts_list = instruction_map
        .get("accounts")
        .and_then(|v| v.as_array())
        .ok_or("Accounts is not an array")?;
    for raw_account in accounts_list {
        let account = raw_account.as_object().ok_or("Account is not an object")?;
        let account_name = account
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Account name is not a string")?;
        accounts.push(account_name.to_string());
    }
    Ok(accounts)
}

fn parse_instruction_args(
    instruction_map: &Map<String, Value>,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    Ok(instruction_map
        .get("args")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default())
}

fn parse_implicit_instruction_discriminant(
    instruction_name: &str,
) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let seeds = format!("global:{}", camel_to_snake_case(instruction_name)).into_bytes();
    let key = u64::from_le_bytes(
        hash(&seeds).to_bytes()[..8]
            .try_into()
            .map_err(|_| "Failed to convert hash to u64")?,
    );
    Ok((key, 8))
}

fn validate_on_chain_idl(on_chain_idl: &OnChainIdl) -> Result<(), Box<dyn std::error::Error>> {
    let serialized = on_chain_idl.try_to_vec()?;
    let deserialized = OnChainIdl::try_from_slice(&serialized)?;
    assert_eq!(deserialized, *on_chain_idl);
    Ok(())
}

fn _get_disc_from_str(s: &str) -> u64 {
    match s {
        "u8" => 1,
        "u64" => 8,
        _ => panic!("Unknown discriminant type: {}", s),
    }
}

fn camel_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_uppercase() {
            if !result.is_empty()
                && chars
                    .peek()
                    .is_some_and(|next| next.is_lowercase() || next.is_ascii_digit())
            {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    result
}

pub struct IdlParser {
    type_map: HashMap<String, Map<String, Value>>,
    parsed_cache: RefCell<HashMap<String, SchemaNode>>,
}

impl IdlParser {
    pub fn new(type_map: HashMap<String, Map<String, Value>>) -> Self {
        Self {
            type_map,
            parsed_cache: RefCell::new(HashMap::new()),
        }
    }
}

impl IdlParser {
    pub fn parse(&mut self) -> Result<HashMap<String, SchemaNode>, Box<dyn std::error::Error>> {
        let keys: Vec<_> = self.type_map.keys().cloned().collect();
        let mut types = HashMap::new();
        for type_name in keys {
            match self.parse_type(&type_name) {
                Ok(schema) => {
                    types.insert(type_name.clone(), schema);
                }
                Err(e) => {
                    println!("Failed to parse type: {}: {:?}", type_name, e);
                    continue;
                }
            }
        }
        Ok(types)
    }

    fn parse_type(&self, type_name: &str) -> Result<SchemaNode, Box<dyn std::error::Error>> {
        if let Some(schema) = self.parsed_cache.borrow().get(type_name) {
            return Ok(schema.clone());
        }
        let type_map = self
            .type_map
            .get(type_name)
            .ok_or_else(|| format!("Type {} not found in type map", type_name))?;
        let typ = type_map
            .get("type")
            .and_then(|v| v.as_object())
            .ok_or_else(|| format!("Type for {} is not an object", type_name))?;
        let kind = typ
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or("Kind is not a string")?;
        let schema = match kind {
            "struct" => {
                let fields = typ
                    .get("fields")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| format!("Fields for {} is not an array", type_name))?;
                self.parse_fields(type_name, fields)
            }
            "enum" => {
                let variants = typ
                    .get("variants")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| format!("Variants for {} is not an array", type_name))?;
                let mut nodes = vec![];
                for raw_variant in variants {
                    let variant = raw_variant.as_object().ok_or("Variant is not an object")?;
                    let variant_name = variant
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or("Variant name is not a string")?;
                    if let Some(fields) = variant.get("fields") {
                        let fields = fields.as_array().ok_or_else(|| {
                            format!("Fields for variant {} is not an array", variant_name)
                        })?;
                        let inner_schema = self.parse_fields(variant_name, fields)?.typ;
                        nodes.push(SchemaNode::new(variant_name, inner_schema));
                    } else {
                        nodes.push(SchemaNode::new(variant_name, SchemaType::Empty));
                    }
                }
                Ok(SchemaNode::new(type_name, SchemaType::Enum(nodes)))
            }
            _ => Err("Unknown type kind".into()),
        }?;
        self.parsed_cache
            .borrow_mut()
            .insert(type_name.to_string(), schema.clone());
        Ok(schema)
    }

    fn parse_fields(
        &self,
        type_name: &str,
        fields: &[Value],
    ) -> Result<SchemaNode, Box<dyn std::error::Error>> {
        let mut parsed_fields = Vec::new();
        for raw_field in fields {
            let field = self.parse_field(raw_field)?;
            parsed_fields.push(field);
        }
        Ok(SchemaNode::new(
            type_name,
            SchemaType::Struct(parsed_fields),
        ))
    }

    fn parse_field(&self, raw_field: &Value) -> Result<SchemaNode, Box<dyn std::error::Error>> {
        if !raw_field.is_object() {
            return Ok(SchemaNode::new("", self.parse_field_inner(raw_field)?));
        }

        let field = raw_field.as_object().ok_or("Field is not an object")?;

        if !field.contains_key("type") {
            return Ok(SchemaNode::new("", self.parse_field_inner(raw_field)?));
        }

        let field_name = field
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let field_type = field.get("type").ok_or("Field type not found")?.clone();
        let schema_type = self.parse_field_inner(&field_type)?;
        Ok(SchemaNode::new(field_name, schema_type))
    }

    fn parse_field_inner(
        &self,
        field_type: &Value,
    ) -> Result<SchemaType, Box<dyn std::error::Error>> {
        let schema_type = if field_type.is_object() {
            let field_type_object = field_type
                .as_object()
                .ok_or("Field type is not an object")?;
            let (key, value) = field_type_object
                .iter()
                .next()
                .ok_or("Field type object is empty")?;
            match key.as_str() {
                "vec" => SchemaType::vec(if value.is_object() {
                    self.parse_field_inner(value)?
                } else {
                    parse_raw_schema_type(value.as_str().ok_or("Vec type is not a string")?)?
                }),
                "option" => SchemaType::option(if value.is_object() {
                    self.parse_field_inner(value)?
                } else {
                    parse_raw_schema_type(value.as_str().ok_or("Option type is not a string")?)?
                }),
                "array" => {
                    let inner_array = value.as_array().ok_or("Array is not an array")?;
                    let size = inner_array
                        .get(1)
                        .and_then(|v| v.as_u64())
                        .ok_or("Array size is not a u64")? as usize;

                    let value = inner_array.first().ok_or("Array value not found")?;
                    if value.is_object() {
                        self.parse_field_inner(value)?
                    } else {
                        SchemaType::array(
                            size,
                            parse_raw_schema_type(
                                value.as_str().ok_or("Array type is not a string")?,
                            )?,
                        )
                    }
                }
                "defined" => {
                    let inner_type = value.as_str().ok_or("Defined type is not a string")?;

                    // NEW: handle inline parametrized types like SmallVec<u8,Pubkey>
                    if let Some(inner) = inner_type
                        .strip_prefix("SmallVec<")
                        .and_then(|s| s.strip_suffix('>'))
                    {
                        // inner looks like: "u8,Pubkey"  or "u16,u8"  etc.
                        let mut parts = inner.split(',').map(|s| s.trim());
                        let len_s = parts.next().ok_or("SmallVec missing len type")?;
                        let elem_s = parts.next().ok_or("SmallVec missing elem type")?;
                        if parts.next().is_some() {
                            return Err("SmallVec has more than two generic params".into());
                        }

                        let len_ty = match len_s {
                            "u8" => SmallVecLen::U8,
                            "u16" => SmallVecLen::U16,
                            other => {
                                return Err(
                                    format!("Unsupported SmallVec len type: {}", other).into()
                                )
                            }
                        };

                        // Try built-ins first (handle Pubkey/publicKey/case too)
                        let elem_ty = match elem_s {
                            s if s.eq_ignore_ascii_case("pubkey")
                                || s.eq_ignore_ascii_case("publickey") =>
                            {
                                SchemaType::Pubkey
                            }
                            "string" => SchemaType::String,
                            "i8" => SchemaType::I8,
                            "u8" => SchemaType::U8,
                            "i16" => SchemaType::I16,
                            "u16" => SchemaType::U16,
                            "i32" => SchemaType::I32,
                            "u32" => SchemaType::U32,
                            "i64" => SchemaType::I64,
                            "u64" => SchemaType::U64,
                            "i128" => SchemaType::I128,
                            "u128" => SchemaType::U128,
                            "f32" => SchemaType::F32,
                            "f64" => SchemaType::F64,
                            "bool" => SchemaType::Bool,
                            // Otherwise it's a defined user type in the IDL
                            other_defined => self.parse_type(other_defined)?.typ,
                        };

                        return Ok(SchemaType::SmallVec(len_ty, Box::new(elem_ty)));
                    }

                    self.parse_type(inner_type)?.typ
                }
                _ => {
                    return Err("Unknown field type".into());
                }
            }
        } else {
            let field_type_name = field_type.as_str().ok_or("Field type is not a string")?;
            parse_raw_schema_type(field_type_name)?
        };
        Ok(schema_type)
    }
}

fn parse_raw_schema_type(name: &str) -> Result<SchemaType, Box<dyn std::error::Error>> {
    // Support bracket-array shorthand like "[u8; 3]" or "[publicKey; 2]"
    if let Some(inner) = name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let mut parts = inner.split(';');
        let ty_s = parts.next().ok_or("Array missing element type")?.trim();
        let len_s = parts.next().ok_or("Array missing length")?.trim();
        if parts.next().is_some() {
            return Err("Array syntax has extra parts".into());
        }
        let len: usize = len_s.parse::<usize>()?;
        let elem = parse_raw_schema_type(ty_s)?;
        return Ok(SchemaType::array(len, elem));
    }

    Ok(match name {
        "pubkey" | "publicKey" => SchemaType::Pubkey,
        "string" => SchemaType::String,
        "i8" => SchemaType::I8,
        "u8" => SchemaType::U8,
        "i16" => SchemaType::I16,
        "u16" => SchemaType::U16,
        "i32" => SchemaType::I32,
        "u32" => SchemaType::U32,
        "i64" => SchemaType::I64,
        "u64" => SchemaType::U64,
        "i128" => SchemaType::I128,
        "u128" => SchemaType::U128,
        "f32" => SchemaType::F32,
        "f64" => SchemaType::F64,
        "bool" => SchemaType::Bool,
        "bytes" => SchemaType::Vec(Box::new(SchemaType::U8)),
        "bytes_remaining" | "rest" => SchemaType::RemainingBytes,
        _ => panic!("Unknown type: {}", name),
    })
}

#[cfg(test)]
mod test {
    use super::camel_to_snake_case;
    use crate::{
        parse_idl::parse_idl,
        schema::{SchemaType, SmallVecLen},
    };
    use solana_program::hash::hash;

    #[test]
    fn test_print_hash() {
        let b = hash(&format!("account:{}", "SpotMarket").into_bytes()).to_bytes();
        println!("{:x}", u64::from_be_bytes(b[..8].try_into().unwrap()));
        let a = hash(&format!("account:{}", "State").into_bytes()).to_bytes();
        println!("{:x}", u64::from_be_bytes(a[..8].try_into().unwrap()));
    }

    #[test]
    fn test_camel_to_snake_case() {
        assert_eq!(camel_to_snake_case("mintV1"), "mint_v1");
        assert_eq!(camel_to_snake_case("CreateTree"), "create_tree");
        assert_eq!(camel_to_snake_case("CancelRedeem"), "cancel_redeem");
        assert_eq!(
            camel_to_snake_case("NFTMetadataUpdate"),
            "nft_metadata_update"
        );
        assert_eq!(camel_to_snake_case("doAbc123"), "do_abc123");
    }

    // Minimal IDL that:
    //  - uses SmallVec<u8, Pubkey> directly in an instruction arg
    //  - defines a CompiledInstruction type that itself uses SmallVec<u8,u8> and SmallVec<u16,u8>
    //  - uses SmallVec<u8, CompiledInstruction> in an instruction arg (nested SmallVec)
    fn smallvec_idl() -> String {
        r#"{
          "version": "1.0.0",
          "name": "test_prog",
          "instructions": [
            {
              "name": "foo",
              "accounts": [{"name": "a", "isMut": false, "isSigner": false}],
              "args": [
                { "name": "msg", "type": { "defined": "SmallVec<u8,Pubkey>" } },
                { "name": "ixs", "type": { "defined": "SmallVec<u8,CompiledInstruction>" } }
              ]
            }
          ],
          "types": [
            {
              "name": "CompiledInstruction",
              "type": {
                "kind": "struct",
                "fields": [
                  { "name": "programIdIndex", "type": "u8" },
                  { "name": "accountIndexes", "type": { "defined": "SmallVec<u8,u8>" } },
                  { "name": "data", "type": { "defined": "SmallVec<u16,u8>" } }
                ]
              }
            }
          ]
        }"#
        .to_string()
    }

    #[test]
    fn parses_smallvec_definitions() {
        let idl = parse_idl(smallvec_idl()).expect("parse_idl ok");

        // one instruction, grab its schema
        assert_eq!(idl.instruction_params.len(), 1);
        let (_disc, dec) = &idl.instruction_params[0];
        match &dec.instruction_args_parser.typ {
            SchemaType::Struct(fields) => {
                // msg: SmallVec<u8, Pubkey>
                let msg = fields
                    .iter()
                    .find(|f| f.name == "msg")
                    .expect("msg present");
                match &msg.typ {
                    SchemaType::SmallVec(len_ty, elem) => {
                        assert!(matches!(len_ty, SmallVecLen::U8));
                        assert!(matches!(&**elem, SchemaType::Pubkey));
                    }
                    other => panic!("msg wrong schema: {:?}", other),
                }

                // ixs: SmallVec<u8, CompiledInstruction>
                let ixs = fields
                    .iter()
                    .find(|f| f.name == "ixs")
                    .expect("ixs present");
                match &ixs.typ {
                    SchemaType::SmallVec(len_ty, elem) => {
                        assert!(matches!(len_ty, SmallVecLen::U8));
                        match &**elem {
                            SchemaType::Struct(ci_fields) => {
                                // inside CompiledInstruction, check its SmallVec fields too
                                let acc_idx = ci_fields
                                    .iter()
                                    .find(|f| f.name == "accountIndexes")
                                    .unwrap();
                                match &acc_idx.typ {
                                    SchemaType::SmallVec(len_ty, elem) => {
                                        assert!(matches!(len_ty, SmallVecLen::U8));
                                        assert!(matches!(&**elem, SchemaType::U8));
                                    }
                                    other => panic!("accountIndexes wrong schema: {:?}", other),
                                }
                                let data = ci_fields.iter().find(|f| f.name == "data").unwrap();
                                match &data.typ {
                                    SchemaType::SmallVec(len_ty, elem) => {
                                        assert!(matches!(len_ty, SmallVecLen::U16));
                                        assert!(matches!(&**elem, SchemaType::U8));
                                    }
                                    other => panic!("data wrong schema: {:?}", other),
                                }
                            }
                            other => {
                                panic!("ixs elem is not struct CompiledInstruction: {:?}", other)
                            }
                        }
                    }
                    other => panic!("ixs wrong schema: {:?}", other),
                }
            }
            other => panic!("args not a struct: {:?}", other),
        }
    }

    #[test]
    fn parses_bracket_array_args() {
        // Instruction args use the bracket shorthand
        let json = r#"{
      "version": "1.0.0",
      "name": "arr_prog",
      "instructions": [
        {
          "name": "bar",
          "accounts": [],
          "args": [
            { "name": "three", "type": "[u8; 3]" },
            { "name": "twoKeys", "type": "[publicKey; 2]" }
          ]
        }
      ],
      "types": []
    }"#;

        let idl = parse_idl(json.to_string()).expect("parse_idl ok");
        assert_eq!(idl.instruction_params.len(), 1);

        let (_disc, dec) = &idl.instruction_params[0];
        match &dec.instruction_args_parser.typ {
            SchemaType::Struct(fields) => {
                let f_three = fields.iter().find(|f| f.name == "three").unwrap();
                match &f_three.typ {
                    SchemaType::Array(3, inner) => assert!(matches!(**inner, SchemaType::U8)),
                    other => panic!("three wrong schema: {:?}", other),
                }

                let f_keys = fields.iter().find(|f| f.name == "twoKeys").unwrap();
                match &f_keys.typ {
                    SchemaType::Array(2, inner) => assert!(matches!(**inner, SchemaType::Pubkey)),
                    other => panic!("twoKeys wrong schema: {:?}", other),
                }
            }
            other => panic!("args not a struct: {:?}", other),
        }
    }
}
