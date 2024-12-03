# Atlas IDL Schema

This repo contains a library to transform JSON [IDL](https://en.wikipedia.org/wiki/Interface_description_language) files for Solana programs into normal Rust objects.

### How It Works
The Solana Program IDL pipeline is extremely convoluted. One might even argue that it creates a programming anti-pattern. However, given the prevalence of IDLs for SVM programs, developers should adapt to the situation and develop tooling to make it easier to work with what is available

```mermaid
flowchart TD
    A[SVM Program] -->|*Generates via* IDL Parser| B[IDL JSON File]
    B  --> |*Feeds into*| C[Revese IDL Parser]
    E[Account Data] -.-> |*Queries with* Raw Account Data| D 
    G[Instruction Data] -.-> |*Queries with* Raw Instruction Data| D
    C -->|*Generates via* Schema Parser| D[Schema Nodes] -.-> |*Generates via* Value Parser| F[Value Nodes] -.-> |*Derives*| H[Formatted JSON Data]  
```

This library can convert a JSON IDL string into an intermediate Rust `SchemaNode` type. The `SchemaNode` is a recursively defined type that can receive opaque byte object as input and decode those bytes into a `ValueNode` (another recursively defined type). Serializing the `ValueNode` will yield the parsed byte object as human-readable JSON.

### Example

Here's an example of how to read a Drift V2 State account from Solana Mainnet given the Drift IDL file.

```rust
use atlas_idl_schema::parse_idl;
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    let drift_idl = parse_idl::parse_idl_file("tests/idls/drift.json")
        .map_err(|e| anyhow::anyhow!("Failed to parse IDL file: {}", e))?;

    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());

    // State
    let account_data = client.get_account_data(
        &solana_sdk::pubkey::Pubkey::from_str("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN")
            .unwrap(),
    )?;
    let parsed_account = drift_idl.get_parsed_account(account_data, true)?;
    println!("{}\n", serde_json::to_string(&parsed_account).unwrap());
}
```

Output:
```
{"name":"State","schema":{"admin":"pubkey","whitelistMint":"pubkey","discountMint":"pubkey","signer":"pubkey","srmVault":"pubkey","perpFeeStructure":{"feeTiers":{"feeNumerator":"u32","feeDenominator":"u32","makerRebateNumerator":"u32","makerRebateDenominator":"u32","referrerRewardNumerator":"u32","referrerRewardDenominator":"u32","refereeFeeNumerator":"u32","refereeFeeDenominator":"u32"},"fillerRewardStructure":{"rewardNumerator":"u32","rewardDenominator":"u32","timeBasedRewardLowerBound":"u128"},"referrerRewardEpochUpperBound":"u64","flatFillerFee":"u64"},"spotFeeStructure":{"feeTiers":{"feeNumerator":"u32","feeDenominator":"u32","makerRebateNumerator":"u32","makerRebateDenominator":"u32","referrerRewardNumerator":"u32","referrerRewardDenominator":"u32","refereeFeeNumerator":"u32","refereeFeeDenominator":"u32"},"fillerRewardStructure":{"rewardNumerator":"u32","rewardDenominator":"u32","timeBasedRewardLowerBound":"u128"},"referrerRewardEpochUpperBound":"u64","flatFillerFee":"u64"},"oracleGuardRails":{"priceDivergence":{"markOraclePercentDivergence":"u64","oracleTwap5minPercentDivergence":"u64"},"validity":{"slotsBeforeStaleForAmm":"i64","slotsBeforeStaleForMargin":"i64","confidenceIntervalMaxSize":"u64","tooVolatileRatio":"i64"}},"numberOfAuthorities":"u64","numberOfSubAccounts":"u64","lpCooldownTime":"u64","liquidationMarginBufferRatio":"u32","settlementDuration":"u16","numberOfMarkets":"u16","numberOfSpotMarkets":"u16","signerNonce":"u8","minPerpAuctionDuration":"u8","defaultMarketOrderTimeInForce":"u8","defaultSpotAuctionDuration":"u8","exchangeStatus":"u8","liquidationDuration":"u8","initialPctToLiquidate":"u16","maxNumberOfSubAccounts":"u16","maxInitializeUserFee":"u16","padding":{"size":10,"type":"u8"}},"value":{"admin":"E1admb4tW2Y6bpbnpE5jYZsc4TE2NArG7siZqDsafnob","whitelistMint":"11111111111111111111111111111111","discountMint":"11111111111111111111111111111111","signer":"JCNCMFXo5M5qwUPg2Utu1u6YWp3MbygxqBsBeXXJfrw","srmVault":"11111111111111111111111111111111","perpFeeStructure":{"feeTiers":{"feeNumerator":100,"feeDenominator":100000,"makerRebateNumerator":10,"makerRebateDenominator":100000,"referrerRewardNumerator":15,"referrerRewardDenominator":100,"refereeFeeNumerator":5,"refereeFeeDenominator":100},"fillerRewardStructure":{"rewardNumerator":90,"rewardDenominator":100000,"timeBasedRewardLowerBound":"7922816251703135349956767907850"},"referrerRewardEpochUpperBound":"429496729605","flatFillerFee":"429496729600080"},"spotFeeStructure":{"feeTiers":{"feeNumerator":10,"feeDenominator":100000,"makerRebateNumerator":15,"makerRebateDenominator":100,"referrerRewardNumerator":5,"referrerRewardDenominator":100,"refereeFeeNumerator":70,"refereeFeeDenominator":100000},"fillerRewardStructure":{"rewardNumerator":10,"rewardDenominator":100000,"timeBasedRewardLowerBound":"7922816251518667480152439521295"},"referrerRewardEpochUpperBound":"429496729600060","flatFillerFee":"429496729600010"},"oracleGuardRails":{"priceDivergence":{"markOraclePercentDivergence":"429496729615","oracleTwap5minPercentDivergence":"429496729605"},"validity":{"slotsBeforeStaleForAmm":"429496729600030","slotsBeforeStaleForMargin":"429496729600010","confidenceIntervalMaxSize":"429496729615","tooVolatileRatio":"429496729605"}},"numberOfAuthorities":"429496729600000","numberOfSubAccounts":"429496729600000","lpCooldownTime":"429496729600","liquidationMarginBufferRatio":0,"settlementDuration":100,"numberOfMarkets":0,"numberOfSpotMarkets":0,"signerNonce":0,"minPerpAuctionDuration":0,"defaultMarketOrderTimeInForce":160,"defaultSpotAuctionDuration":134,"exchangeStatus":1,"liquidationDuration":0,"initialPctToLiquidate":0,"maxNumberOfSubAccounts":0,"maxInitializeUserFee":34464,"padding":[1,0,0,0,0,0,100,0,0,0]}}
```
