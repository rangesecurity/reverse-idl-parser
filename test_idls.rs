use atlas_idl_schema::parse_idl;

/// To test parsing of specific IDL files
/// Make sure the IDL files are present in the same directory as this test file.
/// You can obtain the IDL files from the respective program repositories or Solscan or Solana Explorer.
fn main() {
    println!("Testing Jupiter IDL...");
    match parse_idl::parse_idl_file("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4.json") {
        Ok(idl) => println!("Successfully parsed Jupiter IDL: {}", idl.program_name),
        Err(e) => println!("Error parsing Jupiter IDL: {}", e),
    }

    println!("\nTesting Yvaults IDL...");
    match parse_idl::parse_idl_file("6LtLpnUFNByNXLyCoK9wA2MykKAmQNZKBdY8s47dehDc.json") {
        Ok(idl) => println!("Successfully parsed Yvaults IDL: {}", idl.program_name),
        Err(e) => println!("Error parsing Yvaults IDL: {}", e),
    }
}
