use atlas_idl_schema::parse_idl;
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

#[test]
fn test_parse_idl() -> anyhow::Result<()> {
    let drift_idl = parse_idl::parse_idl_file("tests/idls/drift.json")
        .map_err(|e| anyhow::anyhow!("Failed to parse IDL file: {}", e))?;

    let phoenix_idl = parse_idl::parse_idl_file("tests/idls/phoenix_v1.json")
        .map_err(|e| anyhow::anyhow!("Failed to parse IDL file: {}", e))?;

    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());

    // Spot Market
    let account_data = client.get_account_data(
        &solana_sdk::pubkey::Pubkey::from_str("6gMq3mRCKf8aP3ttTyYhuijVZ2LGi14oDsBbkgubfLB3")
            .unwrap(),
    )?;
    let parsed_account = drift_idl.get_parsed_account(account_data, true)?;
    println!("{}\n", serde_json::to_string(&parsed_account).unwrap());

    // State
    let account_data = client.get_account_data(
        &solana_sdk::pubkey::Pubkey::from_str("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN")
            .unwrap(),
    )?;
    let parsed_account = drift_idl.get_parsed_account(account_data, true)?;
    println!("{}\n", serde_json::to_string(&parsed_account).unwrap());

    let ix_data = vec![
        213, 51, 1, 187, 108, 220, 230, 224, 4, 1, 0, 0, 0, 16, 165, 212, 232, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 1, 31, 87, 1, 0, 1, 20, 1, 239, 218, 0, 0, 0, 0, 0, 0,
        1, 31, 87, 1, 0, 0, 0, 0, 0, 0,
    ];
    let parsed_ix = drift_idl.get_parsed_instruction(ix_data, &[], true)?;
    println!("{}\n", serde_json::to_string(&parsed_ix).unwrap());

    let phoenix_cancel_ix_data = vec![
        11, 8, 0, 0, 0, 0, 115, 3, 0, 0, 0, 0, 0, 0, 57, 84, 239, 255, 255, 255, 255, 255, 0, 114,
        3, 0, 0, 0, 0, 0, 0, 56, 84, 239, 255, 255, 255, 255, 255, 0, 112, 3, 0, 0, 0, 0, 0, 0, 55,
        84, 239, 255, 255, 255, 255, 255, 0, 111, 3, 0, 0, 0, 0, 0, 0, 54, 84, 239, 255, 255, 255,
        255, 255, 1, 121, 3, 0, 0, 0, 0, 0, 0, 202, 171, 16, 0, 0, 0, 0, 0, 1, 122, 3, 0, 0, 0, 0,
        0, 0, 203, 171, 16, 0, 0, 0, 0, 0, 1, 123, 3, 0, 0, 0, 0, 0, 0, 204, 171, 16, 0, 0, 0, 0,
        0, 1, 125, 3, 0, 0, 0, 0, 0, 0, 205, 171, 16, 0, 0, 0, 0, 0,
    ];

    let parsed_ix = phoenix_idl.get_parsed_instruction(phoenix_cancel_ix_data, &[], true)?;
    println!("{}\n", serde_json::to_string(&parsed_ix).unwrap());

    Ok(())
}
