use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    schema::{SchemaNode, SchemaType},
    value::{TypedValue, ValueNode},
};

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, PartialEq, Eq)]
pub struct InstructionDecoder {
    pub accounts: Vec<String>,
    pub instruction_args_parser: SchemaNode,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, PartialEq, Eq)]
pub struct OnChainIdl {
    pub program_name: String,
    pub account_disc_len: u8,
    pub instruction_disc_len: u8,
    pub accounts: Vec<(u64, SchemaNode)>,
    pub instruction_params: Vec<(u64, InstructionDecoder)>,
}

impl OnChainIdl {
    pub fn get_parsed_instruction(
        &self,
        instruction_data: Vec<u8>,
        account_keys: &[String],
        show_hidden: bool,
    ) -> anyhow::Result<ParsedInstructionResult> {
        let data = instruction_data;
        if data.len() < self.instruction_disc_len as usize {
            return Err(anyhow::anyhow!("Instruction data is too short"));
        }

        let mut padded_data = [0u8; 8];
        let slice_len = std::cmp::min(self.instruction_disc_len as usize, 8);
        padded_data[..slice_len].copy_from_slice(&data[..slice_len]);
        let discriminant = u64::from_le_bytes(padded_data);

        let instruction_decoder = self
            .instruction_params
            .iter()
            .find(|(disc, _)| *disc == discriminant)
            .map(|(_, decoder)| decoder)
            .ok_or(anyhow::anyhow!("Instruction discriminant not found"))?;

        let mut account_names = vec![];
        for (i, _) in account_keys.iter().enumerate() {
            let name = instruction_decoder
                .accounts
                .get(i)
                .cloned()
                .unwrap_or(format!("Account {}", i + 1));
            account_names.push(name);
        }

        let schema = instruction_decoder.instruction_args_parser.clone();

        let args: ValueNode = schema
            .deserialize_bytes(
                &mut &data[self.instruction_disc_len as usize..],
                show_hidden,
            )?
            .ok_or(anyhow::anyhow!(
                "is_hidden shouldn't be true in instructions"
            ))?;

        Ok(ParsedInstructionResult::new(
            schema,
            account_names,
            args.value,
        ))
    }

    pub fn get_parsed_account(
        &self,
        account_data: Vec<u8>,
        show_hidden: bool,
    ) -> anyhow::Result<ParsedAccountResult> {
        if account_data.len() < self.account_disc_len as usize {
            return Err(anyhow::anyhow!("Account data is too short"));
        }

        let discriminant = self.get_account_discriminator(&account_data);

        let account_schema = self
            .accounts
            .iter()
            .find(|(disc, _)| *disc == discriminant)
            .map(|(_, schema)| schema)
            .ok_or(anyhow::anyhow!("Account discriminant not found"))?
            .clone();

        let value: ValueNode = account_schema
            .deserialize_bytes(
                &mut &account_data[self.account_disc_len as usize..],
                show_hidden,
            )?
            .ok_or(anyhow::anyhow!("Account type shouldn't be hidden"))?;

        Ok(ParsedAccountResult::new(account_schema, value.value))
    }

    pub fn get_account_discriminator(&self, account_data: &[u8]) -> u64 {
        let mut padded_data = [0u8; 8];
        let slice_len = std::cmp::min(self.account_disc_len as usize, 8);
        padded_data[..slice_len].copy_from_slice(&account_data[..slice_len]);
        u64::from_le_bytes(padded_data)
    }

    pub fn get_instruction_discriminator(&self, data: &[u8]) -> u64 {
        let mut padded_data = [0u8; 8];
        let slice_len = std::cmp::min(self.instruction_disc_len as usize, 8);
        padded_data[..slice_len].copy_from_slice(&data[..slice_len]);
        u64::from_le_bytes(padded_data)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ParsedInstructionResult {
    pub name: String,
    pub schema: SchemaType,
    pub accounts: Vec<String>,
    pub value: TypedValue,
}

impl ParsedInstructionResult {
    pub fn new(schema: SchemaNode, accounts: Vec<String>, value: TypedValue) -> Self {
        Self {
            name: schema.name,
            schema: schema.typ,
            accounts,
            value,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ParsedAccountResult {
    pub name: String,
    pub schema: SchemaType,
    pub value: TypedValue,
}

impl ParsedAccountResult {
    pub fn new(schema: SchemaNode, value: TypedValue) -> Self {
        Self {
            name: schema.name,
            schema: schema.typ,
            value,
        }
    }
}
